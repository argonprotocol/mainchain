import {
  ArgonClient,
  ArgonPrimitivesVault,
  formatArgons,
  KeyringPair,
  toFixedNumber,
  TxSubmitter,
} from './index';
import BigNumber, * as BN from 'bignumber.js';
import { convertFixedU128ToBigNumber, convertPermillToBigNumber } from './utils';
import bs58check from 'bs58check';

const { ROUND_FLOOR } = BN;

export class Vault {
  public securitization: bigint;
  public securitizationRatio: BigNumber;
  public argonsLocked: bigint;
  public argonsPendingActivation: bigint;
  public argonsScheduledForRelease: Map<number, bigint> = new Map();
  public terms: ITerms;
  public operatorAccountId: string;
  public isClosed: boolean;
  public vaultId: number;
  public pendingTerms?: ITerms;
  public pendingTermsChangeTick?: number;
  public openedDate: Date;
  public openedTick: number;

  constructor(
    id: number,
    vault: ArgonPrimitivesVault,
    private tickDuration: number,
  ) {
    this.securitization = vault.securitization.toBigInt();
    this.securitizationRatio = convertFixedU128ToBigNumber(vault.securitizationRatio.toBigInt());
    this.argonsLocked = vault.argonsLocked.toBigInt();
    this.argonsPendingActivation = vault.argonsPendingActivation.toBigInt();
    if (vault.argonsScheduledForRelease.size > 0) {
      this.argonsScheduledForRelease.clear();
      for (const [tick, amount] of vault.argonsScheduledForRelease.entries()) {
        this.argonsScheduledForRelease.set(tick.toNumber(), amount.toBigInt());
      }
    }
    this.terms = {
      bitcoinAnnualPercentRate: convertFixedU128ToBigNumber(
        vault.terms.bitcoinAnnualPercentRate.toBigInt(),
      ),
      bitcoinBaseFee: vault.terms.bitcoinBaseFee.toBigInt(),
      liquidityPoolProfitSharing: convertPermillToBigNumber(
        vault.terms.liquidityPoolProfitSharing.toBigInt(),
      ),
    };

    this.operatorAccountId = vault.operatorAccountId.toString();
    this.isClosed = vault.isClosed.valueOf();
    this.vaultId = id;
    if (vault.pendingTerms.isSome) {
      const [tickApply, terms] = vault.pendingTerms.value;
      this.pendingTermsChangeTick = tickApply.toNumber();
      this.pendingTerms = {
        bitcoinAnnualPercentRate: convertFixedU128ToBigNumber(
          terms.bitcoinAnnualPercentRate.toBigInt(),
        ),
        bitcoinBaseFee: terms.bitcoinBaseFee.toBigInt(),
        liquidityPoolProfitSharing: convertPermillToBigNumber(
          vault.terms.liquidityPoolProfitSharing.toBigInt(),
        ),
      };
    }
    this.openedTick = vault.openedTick.toNumber();
    this.openedDate = new Date(this.openedTick * tickDuration);
  }

  public availableBitcoinSpace(): bigint {
    const recoverySecuritization = this.recoverySecuritization();
    const reLockable = this.getRelockCapacity();
    return this.securitization - recoverySecuritization - this.argonsLocked + reLockable;
  }

  public getRelockCapacity(): bigint {
    return [...this.argonsScheduledForRelease.values()].reduce((acc, val) => acc + val, 0n);
  }

  public recoverySecuritization(): bigint {
    const reserved = new BigNumber(1).div(this.securitizationRatio);
    return (
      this.securitization -
      BigInt(reserved.multipliedBy(this.securitization.toString()).toFixed(0, ROUND_FLOOR))
    );
  }

  public minimumSecuritization(): bigint {
    return BigInt(
      this.securitizationRatio
        .multipliedBy(this.argonsLocked.toString())
        .decimalPlaces(0, BigNumber.ROUND_CEIL)
        .toString(),
    );
  }

  public activatedSecuritization(): bigint {
    const activated = this.argonsLocked - this.argonsPendingActivation;
    let maxRatio = this.securitizationRatio;
    if (this.securitizationRatio.toNumber() > 2) {
      maxRatio = BigNumber(2);
    }
    return BigInt(maxRatio.multipliedBy(activated.toString()).toFixed(0, ROUND_FLOOR));
  }

  /**
   * Returns the amount of Argons available to match per liquidity pool
   */
  public activatedSecuritizationPerSlot(): bigint {
    const activated = this.activatedSecuritization();
    return activated / 10n;
  }

  public calculateBitcoinFee(amount: bigint): bigint {
    const fee = this.terms.bitcoinAnnualPercentRate
      .multipliedBy(Number(amount))
      .integerValue(BigNumber.ROUND_CEIL);
    return BigInt(fee.toString()) + this.terms.bitcoinBaseFee;
  }

  public static async get(client: ArgonClient, vaultId: number): Promise<Vault> {
    const rawVault = await client.query.vaults.vaultsById(vaultId);
    if (rawVault.isNone) {
      throw new Error(`Vault with id ${vaultId} not found`);
    }
    const tickDuration = (await client.query.ticks
      .genesisTicker()
      .then(x => x.tickDurationMillis.toNumber()))!;
    return new Vault(vaultId, rawVault.unwrap(), tickDuration);
  }

  public static async create(
    client: ArgonClient,
    keypair: KeyringPair,
    args: {
      securitization: bigint | number;
      securitizationRatio: number;
      annualPercentRate: number;
      baseFee: bigint | number;
      bitcoinXpub: string;
      liquidityPoolProfitSharing: number;
      tip?: bigint;
    },
    config: { tickDurationMillis?: number } = {},
  ): Promise<Vault> {
    const { securitization, securitizationRatio, annualPercentRate, baseFee, bitcoinXpub, tip } =
      args;
    let xpubBytes = Buffer.from(bitcoinXpub, 'hex');
    if (xpubBytes.length !== 78) {
      if (
        bitcoinXpub.startsWith('xpub') ||
        bitcoinXpub.startsWith('tpub') ||
        bitcoinXpub.startsWith('zpub')
      ) {
        const bytes = bs58check.decode(bitcoinXpub);
        if (bytes.length !== 78) {
          throw new Error('Invalid Bitcoin xpub key length, must be 78 bytes');
        }
        xpubBytes = Buffer.from(bytes);
      }
    }
    const tx = new TxSubmitter(
      client,
      client.tx.vaults.create({
        terms: {
          // convert to fixed u128
          bitcoinAnnualPercentRate: toFixedNumber(annualPercentRate, 18),
          bitcoinBaseFee: BigInt(baseFee),
          liquidityPoolProfitSharing: toFixedNumber(args.liquidityPoolProfitSharing, 6),
        },
        securitizationRatio: toFixedNumber(securitizationRatio, 18),
        securitization: BigInt(securitization),
        bitcoinXpubkey: xpubBytes,
      }),
      keypair,
    );
    const canAfford = await tx.canAfford({ tip, unavailableBalance: BigInt(securitization) });
    if (!canAfford.canAfford) {
      throw new Error(
        `Insufficient balance to create vault. Required: ${formatArgons(securitization)}, Available: ${formatArgons(canAfford.availableBalance)}`,
      );
    }
    const result = await tx.submit({ waitForBlock: true, tip });
    let vaultId: number | undefined;
    for (const event of result.events) {
      if (client.events.vaults.VaultCreated.is(event)) {
        vaultId = event.data.vaultId.toNumber();
        break;
      }
    }
    if (vaultId === undefined) {
      throw new Error('Vault creation failed, no VaultCreated event found');
    }
    const rawVault = await client.query.vaults.vaultsById(vaultId);
    if (rawVault.isNone) {
      throw new Error('Vault creation failed, vault not found');
    }
    const tickDuration =
      config.tickDurationMillis ??
      (await client.query.ticks.genesisTicker().then(x => x.tickDurationMillis.toNumber()))!;
    return new Vault(vaultId, rawVault.unwrap(), tickDuration);
  }
}

export interface ITerms {
  readonly bitcoinAnnualPercentRate: BigNumber;
  readonly bitcoinBaseFee: bigint;
  readonly liquidityPoolProfitSharing: BigNumber;
}

export interface IBondedArgons {
  readonly allocated: bigint;
  readonly reserved: bigint;
}
