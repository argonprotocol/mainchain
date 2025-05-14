import { type ArgonClient, getTickFromHeader, type Header } from './index';

/**
 * A frame starts with the bidding start time (noon EDT), and ends the next day at noon EDT. Frame 0 was the first day of
 * bidding, and frame 1 began once the first miners were selected. This occurred on February 24th, 2025 at 12pm EDT.
 *
 * This class calculates fromeId from ticks.
 */
export class FrameCalculator {
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
    const biddingStartTick =
      this.genesisTick! + this.miningConfig!.slotBiddingStartAfterTicks;

    const ticksSinceMiningStart = tick - biddingStartTick;

    return Math.floor(ticksSinceMiningStart / ticksBetweenSlots);
  }

  async getForHeader(client: ArgonClient, header: Header) {
    if (header.number.toNumber() === 0) return 0;
    const tick = getTickFromHeader(client, header);
    if (tick === undefined) return undefined;
    return this.getForTick(client, tick);
  }
}
