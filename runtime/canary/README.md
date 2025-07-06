This Runtime goes against the convention a little by driving a feature flag in the main runtime
instead of an entirely new one. The intent is to reduce the complexity that is inherited by having
runtimes with multiple "metadatas" generated. Once that happens, you need to split on how subxt is
generated and how the runtime is generated. We might someday need this, but until then, let's aim
for a one-to-one mapping of Canary to Development/Testnet and Argon runtime is Mainnet + the chain
spec.

## To Avoid

- A full duplication of the runtime config and apis. This ends up with a lot of bloat and potential
  for subtle changed config bugs.
- Any changes that will create a new metadata. Once this is needed, we might need to eject from this
  setup, or be smarter about how we load the runtimes in subxt (we would need to build for both and
  then split calls or move to all dynamic calls).

## How it works

There's a canary feature flag that is set in the main runtime. This flag is used to enable/disable
the canary runtime. This runtime will build with teh canary flag on. If you want to split out a
setting for canary, you can do so in the main argon runtime.
