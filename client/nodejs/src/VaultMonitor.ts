import {
  type ArgonClient,
  type ArgonPrimitivesVault,
  Option,
  Vault,
  BlockWatch,
  MiningBids,
  Accountset,
  formatArgons,
  formatPercent,
} from './index';
import { printTable } from 'console-table-printer';
import { createNanoEvents } from './utils';

export class VaultMonitor {
  public events = createNanoEvents<{
    'bitcoin-space-above': (vaultId: number, amount: bigint) => void;
    'liquidity-pool-space-above': (vaultId: number, amount: bigint) => void;
  }>();
  public readonly vaultsById: { [id: number]: Vault } = {};
  public readonly blockWatch: BlockWatch;
  public readonly mainchain: Promise<ArgonClient>;
  public activatedCapitalByVault: { [vaultId: number]: bigint } = {};
  private lastPrintedBids: Uint8Array | undefined;
  private readonly miningBids: MiningBids;
  private tickDuration: number = 0;
  private vaultOnlyWatchMode: boolean = false;
  private shouldLog: boolean = true;

  constructor(
    readonly accountset: Accountset,
    readonly alerts: WatchAlerts = {},
    readonly options: {
      vaultOnlyWatchMode?: boolean;
      shouldLog?: boolean;
    } = {},
  ) {
    this.mainchain = accountset.client;
    if (options.vaultOnlyWatchMode !== undefined) {
      this.vaultOnlyWatchMode = options.vaultOnlyWatchMode;
    }
    if (options.shouldLog !== undefined) {
      this.shouldLog = options.shouldLog;
    }
    this.miningBids = new MiningBids(this.mainchain, this.shouldLog);
    this.blockWatch = new BlockWatch(this.mainchain, {
      shouldLog: this.shouldLog,
    });
    this.blockWatch.events.on('vaults-updated', (header, vaultIds) =>
      this.onVaultsUpdated(header.hash, vaultIds),
    );
    this.blockWatch.events.on('mining-bid', async (header, _bid) => {
      await this.miningBids.loadAt(this.accountset.namedAccounts, header.hash);
      this.printBids(header.hash);
    });
    this.blockWatch.events.on('mining-bid-ousted', async header => {
      await this.miningBids.loadAt(this.accountset.namedAccounts, header.hash);
      this.printBids(header.hash);
    });
  }

  public stop() {
    this.blockWatch.stop();
  }

  public async monitor(justPrint = false) {
    const client = await this.mainchain;

    this.tickDuration = (await client.query.ticks.genesisTicker()).tickDurationMillis.toNumber();
    const blockHeader = await client.rpc.chain.getHeader();
    const blockHash = blockHeader.hash.toU8a();
    console.log(
      `${justPrint ? 'Run' : 'Started'} at block ${blockHeader.number} - ${blockHeader.hash.toHuman()}`,
    );

    await this.miningBids.loadAt(this.accountset.namedAccounts, blockHash);
    const vaults = await client.query.vaults.vaultsById.entries();
    for (const [storageKey, rawVault] of vaults) {
      const vaultId = storageKey.args[0].toNumber();
      this.updateVault(vaultId, rawVault);
    }

    await client.query.liquidityPools.capitalRaising(x => {
      this.activatedCapitalByVault = {};
      for (const entry of x) {
        const vaultId = entry.vaultId.toNumber();
        this.activatedCapitalByVault[vaultId] = entry.activatedCapital.toBigInt();
      }
      for (const [vaultId, vault] of Object.entries(this.vaultsById)) {
        const id = Number(vaultId);
        this.activatedCapitalByVault[id] ??= 0n;
        this.checkMiningBondAlerts(id, vault);
      }
    });
    this.printVaults();
    if (!this.vaultOnlyWatchMode && this.shouldLog) {
      this.miningBids.print();
    }

    if (!justPrint) await this.blockWatch.start();
  }

  public printVaults() {
    if (!this.shouldLog) return;
    const vaults = [];
    for (const [vaultId, vault] of Object.entries(this.vaultsById)) {
      vaults.push({
        id: vaultId,
        btcSpace: `${formatArgons(vault.availableBitcoinSpace())} (${formatArgons(vault.argonsPendingActivation)} pending)`,
        btcDeal: `${formatArgons(vault.terms.bitcoinBaseFee)} + ${formatPercent(vault.terms.bitcoinAnnualPercentRate)}`,
        securitization: `${formatArgons(vault.securitization)} at ${vault.securitizationRatio.toFormat(1)}x`,
        securActivated: `${formatArgons(vault.activatedSecuritizationPerSlot())}/slot`,
        liquidPoolDeal: `${formatPercent(vault.terms.liquidityPoolProfitSharing)} sharing`,
        operator: `${this.accountset.namedAccounts.has(vault.operatorAccountId) ? ` (${this.accountset.namedAccounts.get(vault.operatorAccountId)})` : vault.operatorAccountId}`,
        state: vault.isClosed ? 'closed' : vault.openedDate < new Date() ? 'open' : 'pending',
      });
    }
    if (vaults.length) {
      if (this.vaultOnlyWatchMode) {
        console.clear();
      }
      console.log('\n\nVaults:');
      printTable(vaults);
    }
  }

  private async recheckAfterActive(vaultId: number) {
    const activationDate = this.vaultsById[vaultId].openedDate;
    if (this.shouldLog) {
      console.log(`Waiting for vault ${vaultId} to activate ${activationDate}`);
    }
    await new Promise(resolve => setTimeout(resolve, activationDate.getTime() - Date.now()));
    const client = await this.mainchain;
    let isReady = false;
    while (!isReady) {
      const rawVault = await client.query.vaults.vaultsById(vaultId);
      if (!rawVault.isSome) return;
      const vault = new Vault(vaultId, rawVault.value, this.tickDuration);
      this.vaultsById[vaultId] = vault;
      if (vault.isClosed) return;
      if (vault.openedDate < new Date()) {
        isReady = true;
        break;
      }
      await new Promise(resolve => setTimeout(resolve, 100));
    }
    this.checkAlerts(vaultId, this.vaultsById[vaultId]);
  }

  private async onVaultsUpdated(blockHash: Uint8Array, vaultIds: Set<number>) {
    await this.reloadVaultsAt([...vaultIds], blockHash).catch(err => {
      console.error(`Failed to reload vault ${[...vaultIds]} at block ${blockHash}:`, err);
    });
    this.printVaults();
  }

  private async reloadVaultsAt(vaultIds: number[], blockHash: Uint8Array) {
    const client = await this.mainchain;
    const api = await client.at(blockHash);
    const vaults = await api.query.vaults.vaultsById.multi(vaultIds);
    for (let i = 0; i < vaultIds.length; i += 1) {
      this.updateVault(vaultIds[i], vaults[i]);
    }
  }

  private updateVault(vaultId: number, rawVault: Option<ArgonPrimitivesVault>) {
    if (rawVault.isNone) return;
    const vault = new Vault(vaultId, rawVault.value, this.tickDuration);
    this.vaultsById[vaultId] = vault;
    if (vault.openedDate > new Date()) {
      void this.recheckAfterActive(vaultId);
    } else {
      this.checkAlerts(vaultId, vault);
    }
  }

  private checkAlerts(vaultId: number, vault: Vault) {
    if (this.alerts.bitcoinSpaceAvailable !== undefined) {
      const availableBitcoinSpace = vault.availableBitcoinSpace();
      if (availableBitcoinSpace >= this.alerts.bitcoinSpaceAvailable) {
        console.warn(
          `Vault ${vaultId} has available bitcoins above ${formatArgons(this.alerts.bitcoinSpaceAvailable)}`,
        );
        this.events.emit('bitcoin-space-above', vaultId, availableBitcoinSpace);
      }
    }
  }

  private checkMiningBondAlerts(vaultId: number, vault: Vault) {
    if (this.alerts.liquidityPoolSpaceAvailable === undefined) return;

    const activatedSecuritization = vault.activatedSecuritizationPerSlot();
    const capitalization = this.activatedCapitalByVault[vaultId] ?? 0n;
    const available = activatedSecuritization - capitalization;
    if (available >= this.alerts.liquidityPoolSpaceAvailable) {
      this.events.emit('liquidity-pool-space-above', vaultId, available);
    }
  }

  private printBids(blockHash: Uint8Array) {
    if (!this.shouldLog) return;
    if (this.lastPrintedBids === blockHash) return;
    this.miningBids.print();
    this.lastPrintedBids = blockHash;
  }
}

export type WatchAlerts = {
  /**
   * Alert when a vault has available space for bitcoins to move in greater than or equal to this amount
   */
  bitcoinSpaceAvailable?: bigint;
  /**
   * Liquidity pool space available
   */
  liquidityPoolSpaceAvailable?: bigint;
};
