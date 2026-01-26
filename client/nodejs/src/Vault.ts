import {
  ArgonClient,
  type ArgonPrimitivesVault,
  FIXED_U128_DECIMALS,
  formatArgons,
  fromFixedNumber,
  KeyringPair,
  PERMILL_DECIMALS,
  toFixedNumber,
  TxSubmitter,
} from './index';
import BigNumber from 'bignumber.js';
import bs58check from 'bs58check';
import { hexToU8a } from '@polkadot/util';
import { TxResult } from './TxResult';
import { ISubmittableOptions } from './TxSubmitter';
import { ApiDecoration } from '@polkadot/api/types';
import type { bool, BTreeMap, Compact, Option, Struct, u128, u64 } from '@polkadot/types-codec';
import type { AccountId32 } from '@polkadot/types/interfaces/runtime';
import type { ITuple } from '@polkadot/types-codec/types';
import { ArgonPrimitivesVaultVaultTerms } from '@polkadot/types/lookup';

interface ArgonPrimitivesVaultV144 extends Struct {
  readonly operatorAccountId: AccountId32;
  readonly securitization: Compact<u128>;
  readonly argonsLocked: Compact<u128>;
  readonly argonsPendingActivation: Compact<u128>;
  readonly argonsScheduledForRelease: BTreeMap<u64, u128>;
  readonly securitizationRatio: Compact<u128>;
  readonly isClosed: bool;
  readonly terms: ArgonPrimitivesVaultVaultTerms;
  readonly pendingTerms: Option<ITuple<[u64, ArgonPrimitivesVaultVaultTerms]>>;
  readonly openedTick: Compact<u64>;
}

export class Vault {
  public securitization!: bigint;
  public securitizationLocked!: bigint;
  public securitizationPendingActivation!: bigint;
  /**
   * Map of bitcoin height to amount of securitization released at that height
   */
  public securitizationReleaseSchedule: Map<number, bigint>;
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
    this.securitizationReleaseSchedule = new Map();
    this.load(vault);
  }

  public load(vault: ArgonPrimitivesVault | ArgonPrimitivesVaultV144): void {
    this.securitization = vault.securitization.toBigInt();
    this.securitizationRatio = fromFixedNumber(
      vault.securitizationRatio.toBigInt(),
      FIXED_U128_DECIMALS,
    ).toNumber();
    this.securitizationReleaseSchedule.clear();
    let schedule: BTreeMap<u64, u128>;
    if ('argonsLocked' in vault) {
      // spec 143 compatibility - don't bother with ratio as it was forced to 1:1
      this.securitizationLocked = vault.argonsLocked.toBigInt();
      this.securitizationPendingActivation = vault.argonsPendingActivation.toBigInt();
      schedule = vault.argonsScheduledForRelease;
    } else {
      this.securitizationLocked = vault.securitizationLocked.toBigInt();
      this.securitizationPendingActivation = vault.securitizationPendingActivation.toBigInt();
      schedule = vault.securitizationReleaseSchedule;
    }
    if (schedule.size > 0) {
      for (const [bitcoinHeight, amount] of schedule.entries()) {
        this.securitizationReleaseSchedule.set(bitcoinHeight.toNumber(), amount.toBigInt());
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
    const availableSecuritization = this.availableSecuritization();
    const microgons = BigNumber(availableSecuritization).div(this.securitizationRatioBN());
    return bigNumberToBigInt(microgons);
  }

  public availableSecuritization(): bigint {
    return this.securitization - this.securitizationLocked;
  }

  public getRelockCapacity(): bigint {
    return [...this.securitizationReleaseSchedule.values()].reduce((acc, val) => acc + val, 0n);
  }

  public securitizationRatioBN(): BigNumber {
    return new BigNumber(this.securitizationRatio);
  }

  public activatedSecuritization(): bigint {
    return this.securitizationLocked - this.securitizationPendingActivation;
  }

  /**
   * Returns the amount of securitization available to match per treasury pool
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
    client: ArgonClient | ApiDecoration<'promise'>,
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
    if (securitizationRatio < 1 || securitizationRatio > 2) {
      throw new Error('Securitization ratio must be between 1 and 2');
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
      return Vault.get(client, vaultId, config.tickDurationMillis);
    }
    return { getVault, txResult: result };
  }
}

export interface ITerms {
  readonly bitcoinAnnualPercentRate: BigNumber;
  readonly bitcoinBaseFee: bigint;
  readonly treasuryProfitSharing: BigNumber;
}
function bigNumberToBigInt(bn: BigNumber): bigint {
  return BigInt(bn.integerValue(BigNumber.ROUND_DOWN).toString());
}
