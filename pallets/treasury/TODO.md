# Treasury reward work

- [ ] Indexed Collect: remove the temporary `MaxArgonBondLots` cap when frame payouts no longer walk
      every lot. Upstream participation deliberately excludes this cap; until cutover, reaching it
      can block new bond purchases without lowering the participation requirement.
- [ ] Treasury Positions refactor: consider a bounded list of concrete encumbered bond lots per
      account (candidate cap: 10 lots, not 10 bonds). Enforce admission when reserving new backing;
      release and burn must use that bounded list rather than scan every account lot. Preserve exact
      microgon backing through partial burns/releases and explicitly migrate existing encumbrances.
      Benchmark and compose the Treasury provider costs into receipt/storage gateway settlement
      weights. Returning actual weight afterward does not replace an up-front execution bound.
- [ ] Future release: pay the 3% Bitcoin-liquid pool using a per-account fission tally exposed to
      operations; burn any unfilled share. Burn the full pool in this release.
- [ ] Revisit vault revenue metrics: report utilization and other useful operating measures rather
      than treating active bond principal as the main performance metric.
