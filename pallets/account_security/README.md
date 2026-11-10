# Account security

AccountSecurity requires independent signatures for selected actions by your main account. Funds
stay in that account. Devices sign off-chain; the submitting transaction carries the operation and
its approvals together. There is no request, pending-call storage or on-chain approval collection.
Only delayed policy replacements are kept on chain, at most one per account group.

The runtime supports native P256 and the existing account signature schemes. Apps/device signing and
coordination still need integration.

## What requires approval?

Each protected account group has one signer set and quorum for exits and sensitive authority
changes, plus a separate offline recovery account.

| Action                                 | Examples                                                                                                   |
| -------------------------------------- | ---------------------------------------------------------------------------------------------------------- |
| Move value outside the protected group | External transfers, Bitcoin releases, localchain deposits, cross-chain withdrawals and burns.              |
| Change sensitive authority             | Add/remove proxies, create a vault, change its delegate or Bitcoin xpub, register council/minting signers. |

Transfers within a linked operational-account group and all mining bids remain unattended, whether
direct or proxied. Registered mining rewards return to the funding account; choosing a different
miner does not redirect those rewards. Installing a proxy still needs approval. Vault delegates are
external destinations, not group members.

Bitcoin receive-address creation, guarantees and Fission creation/ratcheting need Exit approval for
an external vault, but not your own vault or your upstream sponsor's vault. Changing your vault's
xpub still needs approval.

The [runtime policy](../../runtime/common/src/call_authorization.rs) defines exact coverage. Routine
council signature submission (`approve_queue_entries`) is excluded; `collateralize_transfer`
currently needs Exit approval, pending its economic review.

## APIs

| API                      | Use                                                                                                                                                     |
| ------------------------ | ------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `register`               | Enroll the signer set, quorum, recovery account and recovery delay in ticks. Every initial authority must accept.                                       |
| `update_policy`          | Replace the complete configuration immediately or schedule it under the current recovery delay, depending on which authority approves and what changes. |
| `execute`                | Verify the collected signatures and execute the exact call or batch as that account.                                                                    |
| `complete_policy_update` | Apply an accepted pending policy after its tick deadline. Anyone may submit it, identifying the pending update by its consumed authorization nonce.     |
| `cancel_policy_update`   | Main plus the current quorum or recovery authority vetoes or withdraws that specific pending policy.                                                    |

All calls except completion take `account`, `authorization`, and `approvals`. The authorization
object binds the group, nonce, expiry, and policy hash without making them separate public
arguments. Registration and policy updates also take `signers`, `quorum`, `recovery_account`, and
`recovery_delay`. Those policy arguments stay top-level so hardware wallets can label the choices
being approved. Execution instead takes the nested `call`. Stored policies retain their structured
types.

One registration belongs to the default/vault account and protects its linked operational and mining
accounts. Standalone accounts register themselves. Only the default account manages policy; linked
members can execute their own operations under the shared policy.

Use keys independent of the main mnemonic. Recovery must be separate from the protected group and
the signer set. Distinct addresses alone cannot prove independent secrets.

## Signing and submitting

1. Construct the security call with the expected group, next authorization nonce, expiry block and
   current policy hash. Leave its approvals vector empty.
2. Each required device signs that full call using the canonical native signing context. The device
   receives the operation, not only a hash. See the [signing contract](src/lib.rs). Each proof's
   `with_metadata_hash` flag must match its wallet payload's metadata-hash mode.
3. Submit the same security call with the collected signer/signature proofs. The main account signs
   the outer transaction, or supplies an additional proof if a relayer submits.

For a single signer, this is one device approval and one transaction. For multiple signers, collect
the required proofs before submission; there is no required signing order. A complete batch needs
one signature per participating signer, not one per child call. The same quorum covers authority
changes and exits within a batch.

The submitter pays the normal transaction fee. Signer and recovery accounts need no token balance or
activation transaction. Active and pending policies keep their account records alive and advance a
zero native nonce to one; existing nonzero nonces are preserved. Supplying a detached proof does not
consume that native nonce. Registration reserves a storage deposit from the default account. A
delayed policy reserves an additional bounded storage deposit, refunded on completion, cancellation
or invalidation. There are no sponsored approval transactions.

Put application batches inside `execute.call`. `execute` itself must remain top-level. Recovery can
be a plain cold account or a native Proxy/Multisig-controlled account: `register`, `update_policy`
and `cancel_policy_update` accept that authenticated recovery origin through the native wrappers.
Main-account consent must then be an embedded proof; proxy permission cannot substitute for it. Do
not mix these management calls with other operations in a batch or nest them inside `execute`.
Completion is a separate top-level transaction.

## Replay and failure

The authorization nonce is shared by the account group, separate from the submitter's ordinary
transaction nonce. An accepted operation consumes it, so two proposals signed with the same nonce
compete: only one can execute. Policy changes and account linking invalidate outstanding approvals.

Clients must check `Executed.result`. Once execution starts, even a failed child consumes the nonce.
An atomic `batch_all` rolls back child effects on failure; retrying requires fresh authorization.
Invalid proofs or an expired/stale authorization do not dispatch the operation.

## Change or recover keys

Every update needs main-account consent. Newly added signers and a new recovery account must accept.
The current policy stays active throughout a pending change; ordinary operations continue.

| Situation                                   | Authority besides main                                                        | When it applies                                             | Who can veto, with main consent |
| ------------------------------------------- | ----------------------------------------------------------------------------- | ----------------------------------------------------------- | ------------------------------- |
| Change normal signers or quorum             | Current quorum; new signers accept                                            | Immediately                                                 | Not delayed                     |
| Lose enough devices to break quorum         | Current recovery authority; new signers accept                                | After the current recovery delay                            | Current normal quorum           |
| Replace a lost or retiring recovery account | Current quorum; new recovery authority accepts. Old recovery is not required. | After the current recovery delay                            | Current recovery authority      |
| Change the recovery delay                   | Current quorum                                                                | After the **old** delay, including when reducing it to zero | Current recovery authority      |
| Lose recovery authority and normal quorum   | No remaining recovery route                                                   | Cannot recover through this pallet                          | —                               |
| Lose main mnemonic                          | Other authorities cannot impersonate main                                     | Cannot recover through this pallet                          | —                               |

Set `recovery_delay` to the number of consensus ticks you want for detection and cancellation. Zero
explicitly removes that window. Once scheduled, advance to `execute_after_tick` and submit
`complete_policy_update`; the signatures' original block expiry is no longer a completion deadline.
Cancellation also permits the proposing authority to withdraw its own request. Neither side can
replace an existing pending request to restart its timer. An immediate normal-quorum policy update
or operational-account linking invalidates a pending request and returns its deposit.

Recovery may itself use native Multisig or a pure proxy whose controllers you manage independently.
AccountSecurity does not add another recovery-quorum format. Losing every controller of that
recovery account has the same effect as losing its private key. Recovery monitoring matters: a delay
offers no protection if nobody can see and cancel an unwanted change before it matures.

For a cold-key setup, keep recovery offline and use an offline signing tool. These updates replace
approval authorities, not the main mnemonic or external Bitcoin keys.

## Devices

| Device                                              | Status / remaining work                                                                                                                                                                                         |
| --------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Ledger / Polkadot Vault                             | Simulator/parser tests cover detached review of transfers, batches, registration and policy updates. Physical-device qualification remains. A device's signature is a proof, not its own submitted transaction. |
| Apple Secure Enclave / Android hardware-backed keys | P256 verification is implemented. App signing integration and authentication checks remain.                                                                                                                     |
| Windows Hello with P256                             | An upstream reference is linked below; no signing adapter is implemented here. App integration and real-device prompt/hardware checks remain.                                                                   |
| PIV-capable YubiKey                                 | Opt-in physical PIV test covers enrollment and execution against the real runtime. A production Apps adapter and PIN/touch qualification remain; the key has no transaction-details display.                    |
| FIDO/WebAuthn keys or passkeys                      | Not supported yet. Needs an on-chain WebAuthn assertion verifier as well as app integration.                                                                                                                    |

A
[Windows reference implementation](https://github.com/dkackman/tauri-plugin-secure-element/blob/e47796b9d29b03d13b61cbeb1524c2aea8e04acc/tauri-plugin-secure-element/src/windows.rs#L749)
sets authentication policy on the key itself. Per-signature enforcement has not been tested on a
real Windows device here; hardware backing must be checked rather than assumed.

A phone can independently review a decoded operation. A same-computer key resists extraction, but
its PIN/biometric prompt is not an independent display of transaction details.

The [signer tests](../../testing/signers) exercise Ledger's simulator and Vault's decoder, not
Vault's mobile UI. The optional physical YubiKey test uses an **existing ECCP256 PIV signing key and
certificate**:

```sh
ARGON_YUBIKEY_SLOT=9c bash testing/signers/run.sh yubikey
```

Install `yubico-piv-tool` and OpenSSL first. Connect only the intended test device. The test prompts
for the existing PIN twice (enrollment and execution), and requires touch if the key's policy does.
It never generates/replaces keys, resets the device or changes PIN/touch settings. A wrong PIN can
consume a retry; do not guess. The test checks native DER conversion, a zero-balance signing
account, successful execution, altered-call rejection and replay rejection. It is excluded from
`all` and ordinary Cargo tests; adding it is not evidence of a physical-device pass.

## Limits

This is not an account freeze or withdrawal delay. Mining automation can still overbid, and some
vault/bond operations can lock liquidity. Announcement delays and pause remain separate features.

It does not secure compromised Ethereum council keys at their external signing boundary or raw
Bitcoin spending when an attacker holds the necessary Bitcoin keys. Recovery does not rotate those
keys. Runtime signature weights and real-device interoperability still require release
qualification.
