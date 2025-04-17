import {
  convertPermillToBigNumber,
  filterUndefined,
  formatArgons,
  formatPercent,
} from './utils';
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
  public nextCohortId: number = 1;
  public poolVaultCapitalByCohort: {
    [cohortId: number]: {
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
  private lastDistributedCohortId?: number;

  private cohortSubscriptions: { [cohortId: number]: () => void } = {};

  constructor(
    readonly client: Promise<ArgonClient>,
    readonly keypair: KeyringPair,
    readonly accountRegistry: AccountRegistry = AccountRegistry.factory(),
  ) {
    this.blockWatch = new BlockWatch(client, { shouldLog: false });
  }

  private async onVaultsUpdated(
    blockHash: Uint8Array,
    vaultIdSet: Set<number>,
  ) {
    const client = await this.client;

    this.tickDuration ??= (
      await client.query.ticks.genesisTicker()
    ).tickDurationMillis.toNumber();
    const api = await client.at(blockHash);
    const vaultIds = [...vaultIdSet];
    const rawVaults = await api.query.vaults.vaultsById.multi(vaultIds);
    for (let i = 0; i < vaultIds.length; i += 1) {
      const rawVault = rawVaults[i];
      if (rawVault.isNone) continue;
      const vaultId = vaultIds[i];
      this.vaultsById[vaultId] = new Vault(
        vaultId,
        rawVault.unwrap(),
        this.tickDuration,
      );
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
    const balanceBytes = await client.rpc.state.call(
      'MiningSlotApi_bid_pool',
      '',
    );
    const balance = client.createType('U128', balanceBytes);
    return balance.toBigInt();
  }

  public async loadAt(blockHash?: Uint8Array) {
    const client = await this.client;
    blockHash ??= (await client.rpc.chain.getHeader()).hash.toU8a();
    const api = await client.at(blockHash);
    const rawVaultIds = await api.query.vaults.vaultsById.keys();
    const vaultIds = rawVaultIds.map(x => x.args[0].toNumber());
    this.bidPoolAmount = await this.getBidPool();
    this.nextCohortId = (await api.query.miningSlot.nextCohortId()).toNumber();

    const contributors =
      await api.query.liquidityPools.liquidityPoolsByCohort.entries();
    for (const [cohortId, funds] of contributors) {
      const cohortIdNumber = cohortId.args[0].toNumber();
      this.loadCohortData(cohortIdNumber, funds);
    }
    for (const entrant of await api.query.liquidityPools.openLiquidityPoolCapital()) {
      this.setVaultCohortData(this.nextCohortId, entrant.vaultId.toNumber(), {
        activatedCapital: entrant.activatedCapital.toBigInt(),
      });
    }
    for (const entrant of await api.query.liquidityPools.nextLiquidityPoolCapital()) {
      this.setVaultCohortData(this.nextCohortId, entrant.vaultId.toNumber(), {
        activatedCapital: entrant.activatedCapital.toBigInt(),
      });
    }
    await this.onVaultsUpdated(blockHash, new Set(vaultIds));
    this.print();
  }

  public async watch(): Promise<{ unsubscribe: () => void }> {
    await this.loadAt();
    await this.blockWatch.start();
    this.blockWatch.events.on('vaults-updated', (b, v) =>
      this.onVaultsUpdated(b.hash, v),
    );
    const api = await this.client;
    this.blockWatch.events.on('event', async (_, event) => {
      if (api.events.liquidityPools.BidPoolDistributed.is(event)) {
        const { cohortId: rawCohortId } = event.data;
        this.lastDistributedCohortId = rawCohortId.toNumber();
        this.bidPoolAmount = await this.getBidPool();

        this.cohortSubscriptions[rawCohortId.toNumber()]?.();
        const entrant =
          await api.query.liquidityPools.liquidityPoolsByCohort(rawCohortId);
        this.loadCohortData(rawCohortId.toNumber(), entrant);
        this.printDebounce();
      }
      if (api.events.liquidityPools.NextBidPoolCapitalLocked.is(event)) {
        const { cohortId } = event.data;

        for (let inc = 0; inc < 2; inc++) {
          const id = cohortId.toNumber() + inc;
          if (!this.cohortSubscriptions[id]) {
            this.cohortSubscriptions[id] =
              await api.query.liquidityPools.liquidityPoolsByCohort(
                id,
                async entrant => {
                  this.loadCohortData(id, entrant);
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
        api.query.miningSlot.nextSlotCohort as any,
        api.query.miningSlot.nextCohortId as any,
        api.query.liquidityPools.openLiquidityPoolCapital as any,
        api.query.liquidityPools.nextLiquidityPoolCapital as any,
      ],
      async ([
        _nextSlotCohort,
        nextCohortId,
        openVaultBidPoolCapital,
        nextPoolCapital,
      ]) => {
        this.bidPoolAmount = await this.getBidPool();
        this.nextCohortId = nextCohortId.toNumber();
        for (const entrant of [
          ...openVaultBidPoolCapital,
          ...nextPoolCapital,
        ]) {
          this.setVaultCohortData(
            entrant.cohortId.toNumber(),
            entrant.vaultId.toNumber(),
            {
              activatedCapital: entrant.activatedCapital.toBigInt(),
            },
          );
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
    return (
      this.accountRegistry.getName(vault.operatorAccountId) ??
      vault.operatorAccountId
    );
  }

  public print() {
    console.clear();
    const lastDistributedCohortId = this.lastDistributedCohortId;
    const distributedCohort =
      this.poolVaultCapitalByCohort[this.lastDistributedCohortId ?? -1] ?? {};
    if (Object.keys(distributedCohort).length > 0) {
      console.log(`\n\nDistributed (cohort ${lastDistributedCohortId})`);

      const rows = [];
      let maxWidth = 0;
      for (const [key, entry] of Object.entries(distributedCohort)) {
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
      `\n\nActive Bid Pool: ${formatArgons(this.bidPoolAmount)} (cohort ${this.nextCohortId})`,
    );
    const cohort = this.poolVaultCapitalByCohort[this.nextCohortId];
    if (Object.keys(cohort ?? {}).length > 0) {
      const rows = [];
      let maxWidth = 0;
      for (const [key, entry] of Object.entries(cohort)) {
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

    const nextPool = this.poolVaultCapitalByCohort[this.nextCohortId + 1] ?? [];
    let maxWidth = 0;
    const nextCapital = [];
    for (const x of this.vaultSecuritization) {
      const entry = nextPool[x.vaultId] ?? {};
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
      console.log(`\n\nNext (cohort ${this.nextCohortId + 1}):`);
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

  private setVaultCohortData(
    cohortId: number,
    vaultId: number,
    data: Partial<IVaultMiningBondFund>,
  ) {
    this.poolVaultCapitalByCohort ??= {};
    this.poolVaultCapitalByCohort[cohortId] ??= {};
    this.poolVaultCapitalByCohort[cohortId][vaultId] ??= {
      activatedCapital:
        data.activatedCapital ??
        data.contributors?.reduce((a, b) => a + b.amount, 0n) ??
        0n,
    };

    Object.assign(
      this.poolVaultCapitalByCohort[cohortId][vaultId],
      filterUndefined(data),
    );
  }

  private createBondCapitalTable(
    total: bigint,
    contributors: IContributor[],
    title = 'Total',
  ) {
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

  private loadCohortData(
    cohortId: number,
    vaultFunds: Iterable<[u32, PalletLiquidityPoolsLiquidityPool]>,
  ): void {
    for (const [vaultId, fund] of vaultFunds) {
      const vaultIdNumber = vaultId.toNumber();
      const contributors = fund.contributorBalances.map(([a, b]) => ({
        address: a.toHuman(),
        amount: b.toBigInt(),
      }));
      if (fund.distributedProfits.isSome) {
        if (cohortId > (this.lastDistributedCohortId ?? 0)) {
          this.lastDistributedCohortId = cohortId;
        }
      }
      this.setVaultCohortData(cohortId, vaultIdNumber, {
        earnings: fund.distributedProfits.isSome
          ? fund.distributedProfits.unwrap().toBigInt()
          : undefined,
        vaultSharingPercent: convertPermillToBigNumber(
          fund.vaultSharingPercent.toBigInt(),
        ),
        contributors,
      });
    }
  }
}
