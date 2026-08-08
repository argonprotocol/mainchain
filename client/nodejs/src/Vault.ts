import {
  type ArgonClient,
  type ArgonPrimitivesVault,
  FIXED_U128_DECIMALS,
  formatArgons,
  fromFixedNumber,
  MICROGONS_PER_ARGON,
  type PalletTreasuryVaultBondState,
  PERMILL_DECIMALS,
  toFixedNumber,
  TxSubmitter,
} from './index';
import BigNumber from 'bignumber.js';
import bs58check from 'bs58check';
import { hexToU8a } from '@polkadot/util';
import { TxResult } from './TxResult';
import type { ISubmittableOptions, TxSigningAccount } from './TxSubmitter';
import type { PriceIndex } from './PriceIndex';
import { ApiDecoration } from '@polkadot/api/types';
import type {
  bool,
  BTreeMap,
  Bytes,
  Compact,
  Option,
  Struct,
  u128,
  u64,
} from '@polkadot/types-codec';
import type { AccountId32 } from '@polkadot/types/interfaces/runtime';
import type { ITuple } from '@polkadot/types-codec/types';
import { ArgonPrimitivesVaultVaultTerms } from '@polkadot/types/lookup';

interface ArgonPrimitivesVaultV144 extends Struct {
  readonly operatorAccountId: AccountId32;
  readonly name: Option<Bytes>;
  readonly lastNameChangeTick: Option<u64>;
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

type PreviousArgonPrimitivesVault = Omit<
  ArgonPrimitivesVault,
  'flexibleSecuritizationLocked' | 'reservedSecuritizationSpace' | 'flexibleSecuritizedSatoshis'
> & {
  readonly backfillSecuritizationLocked: ArgonPrimitivesVault['flexibleSecuritizationLocked'];
  readonly backfillSecuritizationReserved: ArgonPrimitivesVault['reservedSecuritizationSpace'];
  readonly backfillSecuritizedSatoshis: ArgonPrimitivesVault['flexibleSecuritizedSatoshis'];
};

type PreviousPalletTreasuryVaultBondState = Omit<
  PalletTreasuryVaultBondState,
  'regularBondLots' | 'flexibleBonds' | 'reservedBondSpace'
> & {
  readonly bondLots: PalletTreasuryVaultBondState['regularBondLots'];
  readonly backfillBonds: PalletTreasuryVaultBondState['flexibleBonds'];
  readonly backfillBondsReserved: PalletTreasuryVaultBondState['reservedBondSpace'];
};

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

  public lockedSatoshis!: number;
  public securitizedSatoshis!: number;
  public flexibleSecuritizationLocked!: bigint;
  public reservedSecuritizationSpace!: bigint;
  public flexibleSecuritizedSatoshis!: number;
  public name?: string;
  public lastNameChangeTick?: number;
  public delegateAccountId?: string;

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

  public load(
    vault: ArgonPrimitivesVault | PreviousArgonPrimitivesVault | ArgonPrimitivesVaultV144,
  ): void {
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
    if ('lockedSatoshis' in vault) {
      this.lockedSatoshis = vault.lockedSatoshis.toNumber();
      this.securitizedSatoshis = vault.securitizedSatoshis.toNumber();
    } else {
      this.lockedSatoshis = 0;
      this.securitizedSatoshis = 0;
    }
    if ('flexibleSecuritizationLocked' in vault) {
      this.flexibleSecuritizationLocked = vault.flexibleSecuritizationLocked.toBigInt();
      this.reservedSecuritizationSpace = vault.reservedSecuritizationSpace.toBigInt();
      this.flexibleSecuritizedSatoshis = vault.flexibleSecuritizedSatoshis.toNumber();
    } else if ('backfillSecuritizationLocked' in vault) {
      this.flexibleSecuritizationLocked = vault.backfillSecuritizationLocked.toBigInt();
      this.reservedSecuritizationSpace = vault.backfillSecuritizationReserved.toBigInt();
      this.flexibleSecuritizedSatoshis = vault.backfillSecuritizedSatoshis.toNumber();
    } else {
      this.flexibleSecuritizationLocked = 0n;
      this.reservedSecuritizationSpace = 0n;
      this.flexibleSecuritizedSatoshis = 0;
    }

    this.operatorAccountId = vault.operatorAccountId.toString();
    this.isClosed = vault.isClosed.valueOf();
    this.pendingTerms = undefined;
    this.pendingTermsChangeTick = undefined;
    this.name = undefined;
    this.lastNameChangeTick = undefined;
    this.delegateAccountId = undefined;
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
          terms.treasuryProfitSharing.toBigInt(),
          PERMILL_DECIMALS,
        ),
      };
    }
    if ('name' in vault && vault.name.isSome) {
      this.name = decodeVaultName(vault.name.unwrap());
    }
    if ('lastNameChangeTick' in vault && vault.lastNameChangeTick.isSome) {
      this.lastNameChangeTick = vault.lastNameChangeTick.unwrap().toNumber();
    }
    const legacyDelegateAccount = (
      vault as {
        bitcoinLockDelegateAccount?: Option<AccountId32>;
      }
    ).bitcoinLockDelegateAccount;
    if (legacyDelegateAccount?.isSome) {
      this.delegateAccountId = legacyDelegateAccount.unwrap().toHuman();
    }
    if ('delegateAccountId' in vault && vault.delegateAccountId.isSome) {
      this.delegateAccountId = vault.delegateAccountId.unwrap().toHuman();
    }
  }

  public availableBitcoinSpace(lockOwner?: string): bigint {
    const availableSecuritization = this.availableSecuritizationSpace(lockOwner);
    const microgons = BigNumber(availableSecuritization).div(this.securitizationRatioBN());
    return bigNumberToBigInt(microgons);
  }

  public availableSecuritizationSpace(lockOwner?: string): bigint {
    const regularSecuritizationLocked =
      this.securitizationLocked > this.flexibleSecuritizationLocked
        ? this.securitizationLocked - this.flexibleSecuritizationLocked
        : 0n;
    const securitizationSpace =
      this.securitization > regularSecuritizationLocked
        ? this.securitization - regularSecuritizationLocked
        : 0n;
    const available =
      securitizationSpace > this.reservedSecuritizationSpace
        ? securitizationSpace - this.reservedSecuritizationSpace
        : 0n;

    if (lockOwner === this.operatorAccountId) {
      const physicallyAvailable =
        this.securitization > this.securitizationLocked
          ? this.securitization - this.securitizationLocked
          : 0n;
      return available < physicallyAvailable ? available : physicallyAvailable;
    }

    return available;
  }

  public availableBondSpace(
    priceIndex: PriceIndex,
    bondState?:
      | Iterable<{ activeBonds: number }>
      | PalletTreasuryVaultBondState
      | PreviousPalletTreasuryVaultBondState,
    bondFullCapacityPerFrame?: boolean,
  ): bigint {
    const securitizedSatoshis = this.effectiveSecuritizedSatoshis();
    const microgonsPerBond = BigInt(MICROGONS_PER_ARGON);
    let bondCapacity = 0;
    if (securitizedSatoshis > 0n) {
      const totalBondCapacityMicrogons =
        priceIndex.getSatoshiPriceInTargetMicrogons(securitizedSatoshis);
      const capacityMicrogons = bondFullCapacityPerFrame
        ? totalBondCapacityMicrogons
        : totalBondCapacityMicrogons / 10n;
      bondCapacity = Number(capacityMicrogons / microgonsPerBond);
    }
    let regularBonds = 0;
    let reservedBondSpace = 0;
    if (bondState && 'regularBondLots' in bondState) {
      regularBonds = [...bondState.regularBondLots].reduce(
        (sum, bondLot) => sum + bondLot.bonds.toNumber(),
        0,
      );
      reservedBondSpace = bondState.reservedBondSpace.toNumber();
    } else if (bondState && 'bondLots' in bondState) {
      regularBonds = [...bondState.bondLots].reduce(
        (sum, bondLot) => sum + bondLot.bonds.toNumber(),
        0,
      );
      reservedBondSpace = bondState.backfillBondsReserved.toNumber();
    } else {
      regularBonds = [...(bondState ?? [])].reduce((sum, bondLot) => sum + bondLot.activeBonds, 0);
    }
    const unavailableBonds = regularBonds + reservedBondSpace;
    const availableBonds = unavailableBonds < bondCapacity ? bondCapacity - unavailableBonds : 0;

    return BigInt(availableBonds) * microgonsPerBond;
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

  public effectiveSecuritizedSatoshis(): bigint {
    const totalSatoshis = BigInt(this.securitizedSatoshis);
    const flexibleSatoshis = BigInt(this.flexibleSecuritizedSatoshis);
    if (this.flexibleSecuritizationLocked === 0n) return totalSatoshis;

    const confirmedSecuritization =
      this.securitizationLocked > this.securitizationPendingActivation
        ? this.securitizationLocked - this.securitizationPendingActivation
        : 0n;
    const confirmedRegularSecuritizationLocked =
      confirmedSecuritization > this.flexibleSecuritizationLocked
        ? confirmedSecuritization - this.flexibleSecuritizationLocked
        : 0n;
    const flexibleSecuritizationSpace =
      this.securitization > confirmedRegularSecuritizationLocked
        ? this.securitization - confirmedRegularSecuritizationLocked
        : 0n;
    const undisplacedFlexibleSecuritization =
      this.flexibleSecuritizationLocked < flexibleSecuritizationSpace
        ? this.flexibleSecuritizationLocked
        : flexibleSecuritizationSpace;
    const earningFlexibleSatoshis =
      (flexibleSatoshis * undisplacedFlexibleSecuritization) / this.flexibleSecuritizationLocked;

    return totalSatoshis - flexibleSatoshis + earningFlexibleSatoshis;
  }

  /**
   * Returns the amount of securitization available to match per treasury pool
   */
  public activatedSecuritizationPerSlot(): bigint {
    const activated = this.activatedSecuritization();
    return activated / 10n;
  }

  public calculateBitcoinFee(amount: bigint): bigint {
    const feeBn = this.terms.bitcoinAnnualPercentRate
      .multipliedBy(amount)
      .integerValue(BigNumber.ROUND_CEIL);
    return BigInt(feeBn.toString()) + this.terms.bitcoinBaseFee;
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
    txSigner: TxSigningAccount,
    args: {
      securitization: bigint | number;
      securitizationRatio: number;
      annualPercentRate: number;
      baseFee: bigint | number;
      bitcoinXpub: string;
      delegateAccountId?: string;
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
      delegateAccountId,
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
        treasuryBonusProfitSharing: toFixedNumber(0, PERMILL_DECIMALS),
      },
      securitizationRatio: toFixedNumber(securitizationRatio, FIXED_U128_DECIMALS),
      securitization: BigInt(securitization),
      bitcoinXpubkey: xpubBytes,
      delegateAccountId: delegateAccountId ?? null,
    };
    const tx = new TxSubmitter(client, client.tx.vaults.create(vaultParams), txSigner);
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

  public static async setName(
    client: ArgonClient,
    txSigner: TxSigningAccount,
    args: {
      name?: string | null;
    } & ISubmittableOptions,
  ): Promise<TxResult> {
    const legacySetName: typeof client.tx.operationalAccounts.setName | undefined = Reflect.get(
      client.tx.vaults,
      'setName',
    );
    const setName = client.tx.operationalAccounts.setName?.meta
      ? client.tx.operationalAccounts.setName
      : legacySetName;
    if (!setName) {
      throw new Error('The connected runtime does not support setting an operational name');
    }
    const tx = new TxSubmitter(client, setName(encodeVaultName(args.name)), txSigner);

    return tx.submit({
      ...args,
      useLatestNonce: true,
    });
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

function decodeVaultName(name: Bytes): string {
  return new TextDecoder().decode(Uint8Array.from(name));
}

function encodeVaultName(name?: string | null): string | null {
  if (name === undefined || name === null) {
    return null;
  }
  if (!/^[A-Z][A-Za-z0-9]{0,17}$/.test(name)) {
    throw new Error(
      'Vault name must start with a capital letter and contain at most 18 alphanumeric characters',
    );
  }
  return name;
}
