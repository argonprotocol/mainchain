import { ArgonClient } from './index';

export interface IArgonCpiSnapshot {
  // The target price of the argon as a fixed point number (18 decimals)
  argonUsdTargetPrice: bigint;
  // The current price of the argon as a fixed point number (18 decimals)
  argonUsdPrice: bigint;
  // The block hash in which the cpi was finalized
  finalizedBlock: Uint8Array;
  tick: bigint;
}

/**
 * The WageProtector class is used to protect wages from inflation by using the current Argon Price Index to adjust the
 * baseAmount to current conditions. This ensures that the wage is always stable and does not lose or gain value based on
 * demand for the argon or fiat monetary supply changes.
 */
export class WageProtector {
  constructor(public latestCpi: IArgonCpiSnapshot) {}

  /**
   * Converts the base wage to the current wage using the latest CPI snapshot
   *
   * @param baseWage The base wage to convert
   * @returns The protected wage
   */
  public getProtectedWage(baseWage: bigint): bigint {
    return (
      (baseWage * this.latestCpi.argonUsdTargetPrice) /
      this.latestCpi.argonUsdPrice
    );
  }

  /**
   * Subscribes to the current CPI and calls the callback function whenever the CPI changes
   * @param client The ArgonClient to use
   * @param callback The callback function to call when the CPI changes
   * @returns An object with an unsubscribe function that can be called to stop the subscription
   */
  public static async subscribe(
    client: ArgonClient,
    callback: (cpi: WageProtector) => void,
  ): Promise<{
    unsubscribe: () => void;
  }> {
    const unsubscribe = await client.query.priceIndex.current(async cpi => {
      if (cpi.isNone) {
        return;
      }
      const finalizedBlock = await client.rpc.chain.getFinalizedHead();

      callback(
        new WageProtector({
          argonUsdTargetPrice: cpi.value.argonUsdTargetPrice.toBigInt(),
          argonUsdPrice: cpi.value.argonUsdPrice.toBigInt(),
          finalizedBlock: finalizedBlock.toU8a(),
          tick: cpi.value.tick.toBigInt(),
        }),
      );
    });
    return { unsubscribe };
  }

  /**
   * Creates a new WageProtector instance by subscribing to the current CPI and waiting for the first value
   * @param client The ArgonClient to use
   */
  public static async create(client: ArgonClient): Promise<WageProtector> {
    return new Promise<WageProtector>(async (resolve, reject) => {
      try {
        const { unsubscribe } = await WageProtector.subscribe(client, x => {
          resolve(x);
          unsubscribe();
        });
      } catch (e) {
        reject(e);
      }
    });
  }
}
