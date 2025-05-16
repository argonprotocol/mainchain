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
    const { ticksBetweenFrames, biddingStartTick } =
      await this.getConfig(client);

    const ticksSinceMiningStart = tick - biddingStartTick;

    return Math.floor(ticksSinceMiningStart / ticksBetweenFrames);
  }

  async getTickRangeForFrame(
    client: ArgonClient,
    frameId: number,
  ): Promise<[number, number]> {
    const { ticksBetweenFrames, biddingStartTick } =
      await this.getConfig(client);

    const startingTick =
      biddingStartTick + Math.floor(frameId * ticksBetweenFrames);
    const endingTick = startingTick + ticksBetweenFrames - 1;

    return [startingTick, endingTick];
  }

  async getForHeader(client: ArgonClient, header: Header) {
    if (header.number.toNumber() === 0) return 0;
    const tick = getTickFromHeader(client, header);
    if (tick === undefined) return undefined;
    return this.getForTick(client, tick);
  }

  private async getConfig(client: ArgonClient) {
    this.miningConfig ??= await client.query.miningSlot
      .miningConfig()
      .then(x => ({
        ticksBetweenSlots: x.ticksBetweenSlots.toNumber(),
        slotBiddingStartAfterTicks: x.slotBiddingStartAfterTicks.toNumber(),
      }));
    this.genesisTick ??= await client.query.ticks
      .genesisTick()
      .then((x: { toNumber: () => number }) => x.toNumber());
    const config = this.miningConfig!;
    const genesisTick = this.genesisTick!;
    return {
      ticksBetweenFrames: config.ticksBetweenSlots,
      slotBiddingStartAfterTicks: config.slotBiddingStartAfterTicks,
      genesisTick,
      biddingStartTick: genesisTick + config.slotBiddingStartAfterTicks,
    };
  }
}
