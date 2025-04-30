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
    const slot1StartTick =
      this.genesisTick! +
      this.miningConfig!.slotBiddingStartAfterTicks +
      ticksBetweenSlots;
    if (tick < slot1StartTick) return 0;
    // Calculate the number of ticks since the start of bidding. Once bidding started, it was rotation 1
    const ticksSinceSlot1 = tick - slot1StartTick;

    return Math.floor(ticksSinceSlot1 / ticksBetweenSlots);
  }

  async getForHeader(client: ArgonClient, header: Header) {
    if (header.number.toNumber() === 0) return 0;
    const tick = getTickFromHeader(client, header);
    if (tick === undefined) return undefined;
    return this.getForTick(client, tick);
  }
}
