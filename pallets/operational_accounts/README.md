# Operational Accounts

## Registration Minimums

An account may register only once all of the following are true:

- The linked accounts have at least `MinimumUniswapTransfer` in cumulative qualifying Uniswap
  transfer amount.
- The registered vault account has at least `MinimumBitcoin` in bitcoin lock value.
- The registered vault account has at least `MinimumBonds` in active bonds.

Meeting those minimums means the account is eligible to register.

## Operational Certification

A registered account becomes operationally certified once all of the following are true:

- The account has at least `OperationalMinimumUniswapTransfer` in cumulative qualifying Uniswap
  transfer amount.
- The registered vault account has at least `OperationalMinimumVaultSecuritization` in
  securitization.
- The account has at least `MiningSeatsForOperational` mining seats.

Once eligible, any managed account may call `activate` to mark the account operationally certified.

## Follow-On Access Codes

After an account becomes operationally certified, it can earn additional access codes through:

- Downstream certifications: when a downstream account becomes operationally certified, the upstream
  account earns one pending access code.
- Bitcoin progress: vault bitcoin lock growth accumulates toward `BitcoinLockSizeForAccessCode`.
- Mining seat progress: mining seat wins accumulate toward `MiningSeatsPerAccessCode`.

Access codes are capped by `MaxAvailableAccessCodes`.

## Rewards

Certification rewards are paid when an account becomes operationally certified.

- `OperationalCertificationReward` is paid to the newly certified account.
- If the account has an operationally certified upstream account, that upstream account also
  receives the same certification reward.
- `OperationalCertificationBonusReward` is paid each time `OperationalCertificationsPerBonusReward`
  downstream certifications are reached.

Rewards accrue to the operational account and can be claimed in whole-Argon increments from any
managed account.
