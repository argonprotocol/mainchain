import { convertPermillToBigNumber, filterUndefined, formatArgons, formatPercent } from './utils';
import { Table } from 'console-table-printer';
import BigNumber from 'bignumber.js';
import { BlockWatch } from './BlockWatch';
import {
  type ArgonClient,
  type ArgonPrimitivesBlockSealMiningRegistration,
  type KeyringPair,
  type u32,
  type u64,
  type Vec,
} from './index';
import { Vault } from './Vault';
import {
  type PalletLiquidityPoolsLiquidityPool,
  type PalletLiquidityPoolsLiquidityPoolCapital,
} from '@polkadot/types/lookup';
import { TxResult, TxSubmitter } from './TxSubmitter';
import { AccountRegistry } from './AccountRegistry';

const EMPTY_TABLE = {
  headerBottom: { left: ' ', mid: ' ', other: '─', right: ' ' },
  headerTop: { left: ' ', mid: ' ', other: ' ', right: ' ' },
  rowSeparator: { left: ' ', mid: ' ', other: ' ', right: ' ' },
  tableBottom: { left: ' ', mid: ' ', other: ' ', right: ' ' },
  vertical: ' ',
};

interface IContributor {
  address: string;
  amount: bigint;
}

interface IVaultMiningBondFund {
  activatedCapital: bigint;
  contributors?: IContributor[];
  vaultSharingPercent?: BigNumber;
  earnings?: bigint;
}

export class BidPool {
  public bidPoolAmount: bigint = 0n;
  public nextFrameId: number = 1;
  public poolVaultCapitalByFrame: {
    [frameId: number]: {
      [vaultId: number]: IVaultMiningBondFund;
    };
  } = {};

  private vaultSecuritization: {
    vaultId: number;
    activatedSecuritization: bigint;
    bitcoinSpace: bigint;
    vaultSharingPercent: BigNumber;
  }[] = [];
  private printTimeout?: NodeJS.Timeout;
  private blockWatch: BlockWatch;
  private vaultsById: { [id: number]: Vault } = {};
  private tickDuration?: number;
  private lastDistributedFrameId?: number;

  private FrameSubscriptions: { [frameId: number]: () => void } = {};

  constructor(
    readonly client: Promise<ArgonClient>,
    readonly keypair: KeyringPair,
    readonly accountRegistry: AccountRegistry = AccountRegistry.factory(),
  ) {
    this.blockWatch = new BlockWatch(client, { shouldLog: false });
  }

  private async onVaultsUpdated(blockHash: Uint8Array, vaultIdSet: Set<number>) {
    const client = await this.client;

    this.tickDuration ??= (await client.query.ticks.genesisTicker()).tickDurationMillis.toNumber();
    const api = await client.at(blockHash);
    const vaultIds = [...vaultIdSet];
    const rawVaults = await api.query.vaults.vaultsById.multi(vaultIds);
    for (let i = 0; i < vaultIds.length; i += 1) {
      const rawVault = rawVaults[i];
      if (rawVault.isNone) continue;
      const vaultId = vaultIds[i];
      this.vaultsById[vaultId] = new Vault(vaultId, rawVault.unwrap(), this.tickDuration);
    }

    const vaults = Object.entries(this.vaultsById);
    const newSecuritization: BidPool['vaultSecuritization'] = [];
    for (const [vaultId, vault] of vaults) {
      const amount = vault.activatedSecuritizationPerSlot();
      newSecuritization.push({
        vaultId: Number(vaultId),
        bitcoinSpace: vault.availableBitcoinSpace(),
        activatedSecuritization: amount,
        vaultSharingPercent: vault.terms.liquidityPoolProfitSharing,
      });
    }
    newSecuritization.sort((a, b) => {
      const diff2 = b.activatedSecuritization - a.activatedSecuritization;
      if (diff2 !== 0n) return Number(diff2);
      return a.vaultId - b.vaultId;
    });
    this.vaultSecuritization = newSecuritization;
    this.printDebounce();
  }

  public async getBidPool(): Promise<bigint> {
    const client = await this.client;
    const balanceBytes = await client.rpc.state.call('MiningSlotApi_bid_pool', '');
    const balance = client.createType('U128', balanceBytes);
    return balance.toBigInt();
  }

  public async loadAt(blockHash?: Uint8Array) {
    const client = await this.client;
    blockHash ??= Buffer.from((await client.rpc.chain.getHeader()).hash);
    const api = await client.at(blockHash);
    const rawVaultIds = await api.query.vaults.vaultsById.keys();
    const vaultIds = rawVaultIds.map(x => x.args[0].toNumber());
    this.bidPoolAmount = await this.getBidPool();
    this.nextFrameId = (await api.query.miningSlot.nextFrameId()).toNumber();

    const contributors = await api.query.liquidityPools.vaultPoolsByFrame.entries();
    for (const [frameId, funds] of contributors) {
      const FrameIdNumber = frameId.args[0].toNumber();
      this.loadFrameData(FrameIdNumber, funds);
    }
    for (const entrant of await api.query.liquidityPools.capitalActive()) {
      this.setVaultFrameData(entrant.frameId.toNumber(), entrant.vaultId.toNumber(), {
        activatedCapital: entrant.activatedCapital.toBigInt(),
      });
    }
    for (const entrant of await api.query.liquidityPools.capitalRaising()) {
      this.setVaultFrameData(entrant.frameId.toNumber(), entrant.vaultId.toNumber(), {
        activatedCapital: entrant.activatedCapital.toBigInt(),
      });
    }
    await this.onVaultsUpdated(blockHash, new Set(vaultIds));
    this.print();
  }

  public async watch(): Promise<{ unsubscribe: () => void }> {
    await this.loadAt();
    await this.blockWatch.start();
    this.blockWatch.events.on('vaults-updated', (b, v) => this.onVaultsUpdated(b.hash, v));
    const api = await this.client;
    this.blockWatch.events.on('event', async (_, event) => {
      if (api.events.liquidityPools.BidPoolDistributed.is(event)) {
        const { frameId: rawFrameId } = event.data;
        this.lastDistributedFrameId = rawFrameId.toNumber();
        this.bidPoolAmount = await this.getBidPool();

        this.FrameSubscriptions[rawFrameId.toNumber()]?.();
        const entrant = await api.query.liquidityPools.vaultPoolsByFrame(rawFrameId);
        this.loadFrameData(rawFrameId.toNumber(), entrant);
        this.printDebounce();
      }
      if (api.events.liquidityPools.NextBidPoolCapitalLocked.is(event)) {
        const { frameId } = event.data;

        for (let inc = 0; inc < 2; inc++) {
          const id = frameId.toNumber() + inc;
          if (!this.FrameSubscriptions[id]) {
            this.FrameSubscriptions[id] = await api.query.liquidityPools.vaultPoolsByFrame(
              id,
              async entrant => {
                this.loadFrameData(id, entrant);
                this.printDebounce();
              },
            );
          }
        }
      }
    });

    const unsubscribe = await api.queryMulti<
      [
        Vec<ArgonPrimitivesBlockSealMiningRegistration>,
        u64,
        Vec<PalletLiquidityPoolsLiquidityPoolCapital>,
        Vec<PalletLiquidityPoolsLiquidityPoolCapital>,
      ]
    >(
      [
        api.query.miningSlot.bidsForNextSlotCohort as any,
        api.query.miningSlot.nextFrameId as any,
        api.query.liquidityPools.capitalActive as any,
        api.query.liquidityPools.capitalRaising as any,
      ],
      async ([_bids, nextFrameId, openVaultBidPoolCapital, nextPoolCapital]) => {
        this.bidPoolAmount = await this.getBidPool();
        this.nextFrameId = nextFrameId.toNumber();
        for (const entrant of [...openVaultBidPoolCapital, ...nextPoolCapital]) {
          this.setVaultFrameData(entrant.frameId.toNumber(), entrant.vaultId.toNumber(), {
            activatedCapital: entrant.activatedCapital.toBigInt(),
          });
        }
        this.printDebounce();
      },
    );

    return { unsubscribe };
  }

  public async bondArgons(
    vaultId: number,
    amount: bigint,
    options?: { tip: bigint },
  ): Promise<TxResult> {
    const client = await this.client;

    const tx = client.tx.liquidityPools.bondArgons(vaultId, amount);
    const txSubmitter = new TxSubmitter(client, tx, this.keypair);
    const affordability = await txSubmitter.canAfford({
      tip: options?.tip,
      unavailableBalance: amount,
    });

    if (!affordability.canAfford) {
      console.warn('Insufficient balance to bond argons to liquidity pool', {
        ...affordability,
        argonsNeeded: amount,
      });
      throw new Error('Insufficient balance to bond argons to liquidity pool');
    }

    const result = await txSubmitter.submit({
      tip: options?.tip,
      useLatestNonce: true,
    });
    await result.inBlockPromise;
    return result;
  }

  public printDebounce() {
    if (this.printTimeout) {
      clearTimeout(this.printTimeout);
    }
    this.printTimeout = setTimeout(() => {
      this.print();
    }, 100);
  }

  public getOperatorName(vaultId: number): string {
    const vault = this.vaultsById[vaultId];
    return this.accountRegistry.getName(vault.operatorAccountId) ?? vault.operatorAccountId;
  }

  public print() {
    console.clear();
    const lastDistributedFrameId = this.lastDistributedFrameId;
    const distributedFrame = this.poolVaultCapitalByFrame[this.lastDistributedFrameId ?? -1] ?? {};
    if (Object.keys(distributedFrame).length > 0) {
      console.log(`\n\nDistributed (Frame ${lastDistributedFrameId})`);

      const rows = [];
      let maxWidth = 0;
      for (const [key, entry] of Object.entries(distributedFrame)) {
        const { table, width } = this.createBondCapitalTable(
          entry.earnings ?? 0n,
          entry.contributors ?? [],
          `Earnings (shared = ${formatPercent(entry.vaultSharingPercent)})`,
        );
        if (width > maxWidth) {
          maxWidth = width;
        }
        rows.push({
          Vault: key,
          Who: this.getOperatorName(Number(key)),
          Balances: table,
        });
      }
      new Table({
        columns: [
          { name: 'Vault', alignment: 'left' },
          { name: 'Who', alignment: 'left' },
          {
            name: 'Balances',
            title: 'Contributor Balances',
            alignment: 'center',
            minLen: maxWidth,
          },
        ],
        rows,
      }).printTable();
    }
    console.log(
      `\n\nActive Bid Pool: ${formatArgons(this.bidPoolAmount)} (Frame ${this.nextFrameId})`,
    );
    const Frame = this.poolVaultCapitalByFrame[this.nextFrameId];
    if (Object.keys(Frame ?? {}).length > 0) {
      const rows = [];
      let maxWidth = 0;
      for (const [key, entry] of Object.entries(Frame)) {
        const { table, width } = this.createBondCapitalTable(
          entry.activatedCapital,
          entry.contributors ?? [],
        );
        if (width > maxWidth) {
          maxWidth = width;
        }
        rows.push({
          Vault: key,
          Who: this.getOperatorName(Number(key)),
          'Pool Capital': table,
        });
      }
      new Table({
        columns: [
          { name: 'Vault', alignment: 'left' },
          { name: 'Who', alignment: 'left' },
          { name: 'Pool Capital', alignment: 'left', minLen: maxWidth },
        ],
        rows,
      }).printTable();
    }

    const raisingFunds = this.poolVaultCapitalByFrame[this.nextFrameId + 1] ?? [];
    let maxWidth = 0;
    const nextCapital = [];
    for (const x of this.vaultSecuritization) {
      const entry = raisingFunds[x.vaultId] ?? {};
      const { table, width } = this.createBondCapitalTable(
        x.activatedSecuritization,
        entry.contributors ?? [],
      );
      if (width > maxWidth) {
        maxWidth = width;
      }
      nextCapital.push({
        Vault: x.vaultId,
        Owner: this.getOperatorName(x.vaultId),
        'Bitcoin Space': formatArgons(x.bitcoinSpace),
        'Activated Securitization': `${formatArgons(x.activatedSecuritization)} / slot`,
        'Liquidity Pool': `${formatPercent(x.vaultSharingPercent)} profit sharing${table}`,
      });
    }
    if (nextCapital.length) {
      console.log(`\n\nRaising Funds (Frame ${this.nextFrameId + 1}):`);
      new Table({
        columns: [
          { name: 'Vault', alignment: 'left' },
          { name: 'Owner', alignment: 'left' },
          { name: 'Bitcoin Space', alignment: 'right' },
          { name: 'Activated Securitization', alignment: 'right' },
          { name: 'Liquidity Pool', alignment: 'left', minLen: maxWidth },
        ],
        rows: nextCapital,
      }).printTable();
    }
  }

  private setVaultFrameData(frameId: number, vaultId: number, data: Partial<IVaultMiningBondFund>) {
    this.poolVaultCapitalByFrame ??= {};
    this.poolVaultCapitalByFrame[frameId] ??= {};
    this.poolVaultCapitalByFrame[frameId][vaultId] ??= {
      activatedCapital:
        data.activatedCapital ?? data.contributors?.reduce((a, b) => a + b.amount, 0n) ?? 0n,
    };

    Object.assign(this.poolVaultCapitalByFrame[frameId][vaultId], filterUndefined(data));
  }

  private createBondCapitalTable(total: bigint, contributors: IContributor[], title = 'Total') {
    const table = new Table({
      style: EMPTY_TABLE,
      columns: [
        { name: 'who', title: title, minLen: 10, alignment: 'right' },
        {
          name: 'amount',
          title: formatArgons(total),
          minLen: 7,
          alignment: 'left',
        },
      ],
    });
    for (const x of contributors) {
      table.addRow({
        who: this.accountRegistry.getName(x.address) ?? x.address,
        amount: formatArgons(x.amount),
      });
    }
    const str = table.render();
    const width = str.indexOf('\n');
    return { table: str, width };
  }

  private loadFrameData(
    frameId: number,
    vaultFunds: Iterable<[u32, PalletLiquidityPoolsLiquidityPool]>,
  ): void {
    for (const [vaultId, fund] of vaultFunds) {
      const vaultIdNumber = vaultId.toNumber();
      const contributors = fund.contributorBalances.map(([a, b]) => ({
        address: a.toHuman(),
        amount: b.toBigInt(),
      }));
      if (fund.distributedProfits.isSome) {
        if (frameId > (this.lastDistributedFrameId ?? 0)) {
          this.lastDistributedFrameId = frameId;
        }
      }
      this.setVaultFrameData(frameId, vaultIdNumber, {
        earnings: fund.distributedProfits.isSome
          ? fund.distributedProfits.unwrap().toBigInt()
          : undefined,
        vaultSharingPercent: convertPermillToBigNumber(fund.vaultSharingPercent.toBigInt()),
        contributors,
      });
    }
  }
}
