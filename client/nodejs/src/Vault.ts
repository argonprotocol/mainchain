import {
  ArgonClient,
  type ArgonPrimitivesVault,
  FIXED_U128_DECIMALS,
  formatArgons,
  fromFixedNumber,
  ITxProgressCallback,
  KeyringPair,
  PERMILL_DECIMALS,
  toFixedNumber,
  TxSubmitter,
} from './index';
import BigNumber, * as BN from 'bignumber.js';
import bs58check from 'bs58check';
import { hexToU8a } from '@polkadot/util';
import { TxResult } from './TxResult';
import { ISubmittableOptions } from './TxSubmitter';

const { ROUND_FLOOR } = BN;

export class Vault {
  public securitization!: bigint;
  public argonsLocked!: bigint;
  public argonsPendingActivation!: bigint;
  public argonsScheduledForRelease: Map<number, bigint>;
  public terms!: ITerms;
  public operatorAccountId!: string;
  public isClosed!: boolean;
  public vaultId: number;
  public pendingTerms?: ITerms;
  public pendingTermsChangeTick?: number;
  public openedDate: Date;
  public openedTick: number;
  public securitizationRatio!: number;

  constructor(
    id: number,
    vault: ArgonPrimitivesVault,
    public tickDuration: number,
  ) {
    this.vaultId = id;
    this.openedTick = vault.openedTick.toNumber();
    this.openedDate = new Date(this.openedTick * this.tickDuration);
    this.argonsScheduledForRelease = new Map();
    this.load(vault);
  }

  public load(vault: ArgonPrimitivesVault) {
    this.securitization = vault.securitization.toBigInt();
    this.securitizationRatio = fromFixedNumber(
      vault.securitizationRatio.toBigInt(),
      FIXED_U128_DECIMALS,
    ).toNumber();
    this.argonsLocked = vault.argonsLocked.toBigInt();
    this.argonsPendingActivation = vault.argonsPendingActivation.toBigInt();
    if (vault.argonsScheduledForRelease.size > 0) {
      this.argonsScheduledForRelease.clear();
      for (const [tick, amount] of vault.argonsScheduledForRelease.entries()) {
        this.argonsScheduledForRelease.set(tick.toNumber(), amount.toBigInt());
      }
    }
    this.terms = {
      bitcoinAnnualPercentRate: fromFixedNumber(
        vault.terms.bitcoinAnnualPercentRate.toBigInt(),
        FIXED_U128_DECIMALS,
      ),
      bitcoinBaseFee: vault.terms.bitcoinBaseFee.toBigInt(),
      treasuryProfitSharing: fromFixedNumber(
        vault.terms.treasuryProfitSharing.toBigInt(),
        PERMILL_DECIMALS,
      ),
    };

    this.operatorAccountId = vault.operatorAccountId.toString();
    this.isClosed = vault.isClosed.valueOf();
    if (vault.pendingTerms.isSome) {
      const [tickApply, terms] = vault.pendingTerms.value;
      this.pendingTermsChangeTick = tickApply.toNumber();
      this.pendingTerms = {
        bitcoinAnnualPercentRate: fromFixedNumber(
          terms.bitcoinAnnualPercentRate.toBigInt(),
          FIXED_U128_DECIMALS,
        ),
        bitcoinBaseFee: terms.bitcoinBaseFee.toBigInt(),
        treasuryProfitSharing: fromFixedNumber(
          vault.terms.treasuryProfitSharing.toBigInt(),
          PERMILL_DECIMALS,
        ),
      };
    }
  }

  public availableBitcoinSpace(): bigint {
    const recoverySecuritization = this.recoverySecuritization();
    const reLockable = this.getRelockCapacity();
    return this.securitization - recoverySecuritization - this.argonsLocked + reLockable;
  }

  public getRelockCapacity(): bigint {
    return [...this.argonsScheduledForRelease.values()].reduce((acc, val) => acc + val, 0n);
  }

  public securitizationRatioBN(): BigNumber {
    return new BigNumber(this.securitizationRatio);
  }

  public recoverySecuritization(): bigint {
    const reserved = new BigNumber(1).div(this.securitizationRatioBN());
    return (
      this.securitization -
      BigInt(reserved.multipliedBy(this.securitization.toString()).toFixed(0, ROUND_FLOOR))
    );
  }

  public minimumSecuritization(): bigint {
    return BigInt(
      this.securitizationRatioBN()
        .multipliedBy(this.argonsLocked.toString())
        .decimalPlaces(0, BigNumber.ROUND_CEIL)
        .toString(),
    );
  }

  public activatedSecuritization(): bigint {
    const activated = this.argonsLocked - this.argonsPendingActivation;
    const maxRatio = BigNumber(Math.min(this.securitizationRatio, 2));

    return BigInt(maxRatio.multipliedBy(activated.toString()).toFixed(0, ROUND_FLOOR));
  }

  /**
   * Returns the amount of Argons available to match per treasury pool
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

  public static async get(
    client: ArgonClient,
    vaultId: number,
    tickDurationMillis?: number,
  ): Promise<Vault> {
    const rawVault = await client.query.vaults.vaultsById(vaultId);
    if (rawVault.isNone) {
      throw new Error(`Vault with id ${vaultId} not found`);
    }
    const tickDuration =
      tickDurationMillis ??
      (await client.query.ticks.genesisTicker().then(x => x.tickDurationMillis.toNumber()))!;
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
      treasuryProfitSharing: number;
      doNotExceedBalance?: bigint;
    } & ISubmittableOptions,
    config: { tickDurationMillis?: number } = {},
  ): Promise<{ getVault(): Promise<Vault>; txResult: TxResult }> {
    const {
      securitization,
      securitizationRatio,
      annualPercentRate,
      baseFee,
      bitcoinXpub,
      tip,
      doNotExceedBalance,
      txProgressCallback,
    } = args;
    let xpubBytes = hexToU8a(bitcoinXpub);
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
        xpubBytes = bytes;
      }
    }
    const vaultParams = {
      terms: {
        // convert to fixed u128
        bitcoinAnnualPercentRate: toFixedNumber(annualPercentRate, FIXED_U128_DECIMALS),
        bitcoinBaseFee: BigInt(baseFee),
        treasuryProfitSharing: toFixedNumber(args.treasuryProfitSharing, PERMILL_DECIMALS),
      },
      securitizationRatio: toFixedNumber(securitizationRatio, FIXED_U128_DECIMALS),
      securitization: BigInt(securitization),
      bitcoinXpubkey: xpubBytes,
    };
    const tx = new TxSubmitter(client, client.tx.vaults.create(vaultParams), keypair);
    if (doNotExceedBalance) {
      const finalTip = tip ?? 0n;
      let txFee = await tx.feeEstimate(finalTip);
      while (txFee + finalTip + vaultParams.securitization > doNotExceedBalance) {
        vaultParams.securitization = doNotExceedBalance - txFee - finalTip;
        tx.tx = client.tx.vaults.create(vaultParams);
        txFee = await tx.feeEstimate(finalTip);
      }
    }
    const canAfford = await tx.canAfford({ tip, unavailableBalance: BigInt(securitization) });
    if (!canAfford.canAfford) {
      throw new Error(
        `Insufficient balance to create vault. Required: ${formatArgons(securitization)}, Available: ${formatArgons(canAfford.availableBalance)}`,
      );
    }

    const result = await tx.submit({
      ...args,
      useLatestNonce: true,
    });
    const tickDuration =
      config.tickDurationMillis ??
      (await client.query.ticks.genesisTicker().then(x => x.tickDurationMillis.toNumber()))!;

    async function getVault(): Promise<Vault> {
      await result.waitForFinalizedBlock;
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
      return new Vault(vaultId, rawVault.unwrap(), tickDuration);
    }
    return { getVault, txResult: result };
  }
}

export interface ITerms {
  readonly bitcoinAnnualPercentRate: BigNumber;
  readonly bitcoinBaseFee: bigint;
  readonly treasuryProfitSharing: BigNumber;
}
