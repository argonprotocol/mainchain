# Operational Accounts

## Treasury Certification

An account becomes treasury certified once all of the following are true:

- The account has at least `TreasuryMinimumUniswapTransfer` in cumulative qualifying Uniswap
  transfer amount.
- The registered account has at least `TreasuryMinimumBitcoin` in bitcoin lock value.
- The registered account has at least `TreasuryMinimumBonds` in active bonds.

The pallet sets `is_treasury_certified` when the account first reaches those requirements. The
requirements are checked again when a referrer later spends an upgrade code on that account.

## Operations Upgrade

A treasury-certified account can be moved onto the operations flow through `upgrade_account`.

- The caller must be the recorded `referrer`, if one was registered.
- The referrer must have at least one available upgrade code.
- The account must still satisfy the current treasury certification requirements when the upgrade
  happens.

Upgrading marks the account with `is_upgraded_to_operations`, but does not make it operationally
certified yet.

## Operational Certification

An upgraded account becomes operationally certified once all of the following are true:

- The account has at least `OperationalMinimumUniswapTransfer` in cumulative qualifying Uniswap
  transfer amount.
- The registered vault account has at least `OperationalMinimumVaultSecuritization` in
  securitization.
- The account has at least `MiningSeatsForOperational` mining seats.

Once eligible, any managed account may call `activate`.

## Follow-On Upgrade Codes

After an account becomes operationally certified, it can earn additional upgrade codes through:

- Operational referrals: when a downstream account becomes operationally certified, the referrer
  earns one pending upgrade code.
- Bitcoin progress: vault bitcoin lock growth accumulates toward `BitcoinLockSizeForUpgradeCode`.
- Mining seat progress: mining seat wins accumulate toward `MiningSeatsPerUpgradeCode`.

Upgrade codes are capped by `MaxAvailableUpgradeCodes`.

## Rewards

Activation rewards are paid when an account becomes operationally certified.

- `OperationalActivationReward` is paid to the newly activated account.
- If the account has an operationally certified referrer, that referrer also receives the same
  activation reward.
- `OperationalReferralBonusReward` is paid each time `OperationalReferralsPerBonusReward`
  operational referrals are reached.

Rewards accrue to the operational account and can be claimed in whole-Argon increments from any
managed account.
