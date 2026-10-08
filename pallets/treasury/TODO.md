# Treasury reward work

- [ ] Indexed Collect: remove the temporary `MaxArgonBondLots` cap when frame payouts no longer walk
      every lot. Upstream participation deliberately excludes this cap; until cutover, reaching it
      can block new bond purchases without lowering the participation requirement.
- [ ] Future release: pay the 3% Bitcoin-liquid pool using a per-account fission tally exposed to
      operations; burn any unfilled share. Burn the full pool in this release.
- [ ] Revisit vault revenue metrics: report utilization and other useful operating measures rather
      than treating active bond principal as the main performance metric.
