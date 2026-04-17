# Operational Accounts

## Operational Eligibility

An account is eligible to become **operational** once it satisfies all of the following:

- The vault has been created.
- The vault securitization is at least `OperationalMinimumVaultSecuritization`.
- At least one Argon Uniswap transfer has been recorded.
- The bitcoin lock total is greater than zero.
- At least `MiningSeatsForOperational` mining seats have been won.
- The account has participated in at least one treasury pool.

Once eligible, any managed account may call `activate` to make the account operational. Activation
awards the first referral, starts the vault operational-minimum lock, and records any activation
rewards.

## Referral Proof Lifecycle

1. A sponsor signs a referral grant over a referral code public key and expiry frame off-chain.
2. The sponsor sends the referral secret to a recruit.
3. The recruit registers with a `ReferralProof` containing the referral claim and sponsor grant.

Registering with a referral proof links the sponsor only if the sponsor currently has referral
capacity. A linked referral consumes one available referral and may materialize one already-earned
pending referral if capacity is available. Linked referral codes are tracked so the same referral
code cannot sponsor more than one registration before the referral proof expires; repeat uses
register without a sponsor link. Referral codes are cleaned up after expiration.

## How Referrals Are Earned

After an account is operational, it can earn additional referrals through:

- **Sponsored operational recruit:** when a sponsored account becomes operational, the sponsor earns
  one referral (only if the sponsor is operational).
- **Bitcoin lock progress:** total locked increases accumulate toward `BitcoinLockSizeForReferral`.
  Progress decreases if total locked falls. Once a referral is earned, bitcoin progress resets to
  zero.
- **Mining seats:** after operational, mining seat wins accumulate toward `MiningSeatsPerReferral`
  and award a referral. The seats used to become operational count toward this progress. Each
  earning category keeps at most one pending referral. Additional progress in a category does not
  stack into a second pending referral until the pending one is materialized.

## Limits

- **Available referrals** are capped by `MaxAvailableReferrals`.
- **Expired referral cleanup work** is capped by `MaxExpiredReferralCodeCleanupsPerBlock` per block.
  The number of referrals that can expire in the same frame is not capped.

## Rewards

Operational rewards are triggered when an account becomes operational for both the new operational
account and its operational sponsor (if present). Referral bonus rewards are paid each time
`ReferralBonusEveryXOperationalSponsees` sponsored accounts become operational.

Rewards accrue to the operational account and can be claimed in whole-Argon increments from any
managed account. The claimed funds are paid to the managed account that submits the claim.
