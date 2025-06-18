import type { ArgonPrimitivesVault } from './index';
import BigNumber, * as BN from 'bignumber.js';
import { convertFixedU128ToBigNumber, convertPermillToBigNumber } from './utils';

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

  constructor(id: number, vault: ArgonPrimitivesVault, tickDuration: number) {
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
    this.openedDate = vault.openedTick
      ? new Date(vault.openedTick.toNumber() * tickDuration)
      : new Date();
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
