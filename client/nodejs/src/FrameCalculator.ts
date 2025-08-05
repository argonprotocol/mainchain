import { type ArgonClient, getTickFromHeader, type Header } from './index';

/**
 * A frame starts with the bidding start time (noon EDT), and ends the next day at noon EDT. Frame 0 was the first day of
 * bidding, and frame 1 began once the first miners were selected. This occurred on February 24th, 2025 at 12pm EDT.
 *
 * This class calculates fromeId from ticks.
 */
export class FrameCalculator {
  miningConfig: { ticksBetweenSlots: number; slotBiddingStartAfterTicks: number } | undefined;
  genesisTick: number | undefined;
  tickMillis: number | undefined;

  async load(client: ArgonClient) {
    return await this.getConfig(client);
  }

  async getForTick(client: ArgonClient, tick: number) {
    const { ticksBetweenFrames, biddingStartTick } = await this.getConfig(client);

    const ticksSinceMiningStart = tick - biddingStartTick;

    return Math.floor(ticksSinceMiningStart / ticksBetweenFrames);
  }

  async getTickRangeForFrame(client: ArgonClient, frameId: number): Promise<[number, number]> {
    const { ticksBetweenFrames, biddingStartTick } = await this.getConfig(client);

    return FrameCalculator.calculateTickRangeForFrame(frameId, {
      ticksBetweenFrames,
      biddingStartTick,
    });
  }

  async getForHeader(client: ArgonClient, header: Header) {
    if (header.number.toNumber() === 0) return 0;
    const tick = getTickFromHeader(client, header);
    if (tick === undefined) return undefined;
    return this.getForTick(client, tick);
  }

  static frameToDateRange(
    frameId: number,
    config: {
      ticksBetweenFrames: number;
      biddingStartTick: number;
      tickMillis: number;
    },
  ): [Date, Date] {
    const [start, end] = FrameCalculator.calculateTickRangeForFrame(frameId, config);
    return [new Date(start * config.tickMillis), new Date(end * config.tickMillis)];
  }

  static calculateTickRangeForFrame(
    frameId: number,
    config: {
      ticksBetweenFrames: number;
      biddingStartTick: number;
    },
  ): [number, number] {
    const { ticksBetweenFrames, biddingStartTick } = config;

    const startingTick = biddingStartTick + Math.floor(frameId * ticksBetweenFrames);
    const endingTick = startingTick + ticksBetweenFrames - 1;

    return [startingTick, endingTick];
  }

  private async getConfig(client: ArgonClient) {
    this.miningConfig ??= await client.query.miningSlot.miningConfig().then(x => ({
      ticksBetweenSlots: x.ticksBetweenSlots.toNumber(),
      slotBiddingStartAfterTicks: x.slotBiddingStartAfterTicks.toNumber(),
    }));
    this.genesisTick ??= await client.query.ticks
      .genesisTick()
      .then((x: { toNumber: () => number }) => x.toNumber());
    this.tickMillis ??= await client.query.ticks
      .genesisTicker()
      .then(x => x.tickDurationMillis.toNumber());
    const config = this.miningConfig!;
    const genesisTick = this.genesisTick!;
    return {
      ticksBetweenFrames: config.ticksBetweenSlots,
      slotBiddingStartAfterTicks: config.slotBiddingStartAfterTicks,
      genesisTick,
      tickMillis: this.tickMillis!,
      biddingStartTick: genesisTick + config.slotBiddingStartAfterTicks,
    };
  }
}
