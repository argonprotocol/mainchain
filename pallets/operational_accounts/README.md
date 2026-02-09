# Operational Accounts

## Operational Status

An account is considered **operational** once it satisfies all of the following:

- The vault has been created.
- At least one Argon Uniswap transfer has been recorded.
- The bitcoin lock total has reached the configured threshold (`MinBitcoinLockSizeForOperational`).
- At least `MiningSeatsForOperational` mining seats have been won.
- The account has participated in at least one treasury pool.

When an account becomes operational for the first time, it is awarded **one access code**.

## Access Code Lifecycle

1. A sponsor issues an access code by calling `issue_access_code` with a public key.
2. The sponsor sends the access code link to a recruit.
3. The recruit **activates** the access code by calling `register` with the access code proof.

Registering a code issues one **unactivated access code** (issued but not yet activated). Activating
a code consumes one unactivated access code for the sponsor and may materialize one already-earned
pending code if issuance room is available.

## How Access Codes Are Earned

After an account is operational, it can earn additional access codes through:

- **Sponsored operational recruit:** when a sponsored account becomes operational, the sponsor earns
  one access code (only if the sponsor is operational).
- **Bitcoin lock progress:** total locked increases accumulate toward
  `BitcoinLockSizeForAccessCode`. Progress decreases if total locked falls. Once a code is earned,
  bitcoin progress resets to zero.
- **Mining seats:** after operational, mining seat wins accumulate toward `MiningSeatsPerAccessCode`
  and award an access code. The seats used to become operational count toward this progress.
- **Expired access codes:** if an unactivated access code expires (issued but not yet activated),
  the sponsor regains one issuable access code.

Each earning category keeps at most one pending access code. Additional progress in a category does
not stack into a second pending code until the pending one is materialized.

## Limits

- **Issuable access codes** are capped by `MaxIssuableAccessCodes`.
- **Unactivated access codes** (issued but not yet activated) are capped by
  `MaxUnactivatedAccessCodes`.

## Rewards

Operational rewards are triggered when an account becomes operational for both the new operational
account and its operational sponsor (if present). Referral bonus rewards are paid each time
`ReferralBonusEveryXOperationalSponsees` sponsored accounts become operational.
