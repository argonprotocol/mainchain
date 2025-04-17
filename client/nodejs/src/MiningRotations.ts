import { type ArgonClient, getTickFromHeader, type Header } from './index';

/**
 * A rotation is the period from noon EDT to the next noon EDT that a cohort of
 * miners rotates. The first rotation was the period between bidding start and Cohort 1 beginning.
 */
export class MiningRotations {
  private miningConfig:
    | { ticksBetweenSlots: number; slotBiddingStartAfterTicks: number }
    | undefined;
  private genesisTick: number | undefined;

  async getForTick(client: ArgonClient, tick: number) {
    this.miningConfig ??= await client.query.miningSlot
      .miningConfig()
      .then(x => ({
        ticksBetweenSlots: x.ticksBetweenSlots.toNumber(),
        slotBiddingStartAfterTicks: x.slotBiddingStartAfterTicks.toNumber(),
      }));
    this.genesisTick ??= await client.query.ticks
      .genesisTick()
      .then(x => x.toNumber());

    const ticksBetweenSlots = this.miningConfig!.ticksBetweenSlots;
    const slot1Tick =
      this.genesisTick! + this.miningConfig!.slotBiddingStartAfterTicks;
    // Calculate the number of ticks since the start of bidding. Once bidding started, it was rotation 1
    const ticksSinceMiningStart = tick - slot1Tick;

    return Math.floor(ticksSinceMiningStart / ticksBetweenSlots);
  }

  async getForHeader(client: ArgonClient, header: Header) {
    const tick = getTickFromHeader(client, header);
    if (tick === undefined) return undefined;
    return this.getForTick(client, tick);
  }
}
