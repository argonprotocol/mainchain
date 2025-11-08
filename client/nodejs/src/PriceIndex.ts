import {
  ArgonClient,
  FIXED_U128_DECIMALS,
  fromFixedNumber,
  MICROGONS_PER_ARGON,
  SATS_PER_BTC,
} from './index';
import BigNumber from 'bignumber.js';
import { ApiDecoration } from '@polkadot/api/types';

export class PriceIndex {
  btcUsdPrice?: BigNumber;
  argonotUsdPrice?: BigNumber;
  argonUsdPrice?: BigNumber;
  argonUsdTargetPrice?: BigNumber;
  argonTimeWeightedAverageLiquidity?: BigNumber;
  lastUpdatedTick?: number;

  async load(client: ArgonClient | ApiDecoration<'promise'>): Promise<this> {
    const current = await client.query.priceIndex.current();
    if (!current.isSome) {
      this.argonUsdPrice = undefined;
      this.argonotUsdPrice = undefined;
      this.btcUsdPrice = undefined;
      this.argonUsdTargetPrice = undefined;
      this.argonTimeWeightedAverageLiquidity = undefined;
      this.lastUpdatedTick = undefined;
      return this;
    }
    const value = current.unwrap();

    this.btcUsdPrice = fromFixedNumber(value.btcUsdPrice.toBigInt(), FIXED_U128_DECIMALS);
    this.argonotUsdPrice = fromFixedNumber(value.argonotUsdPrice.toBigInt(), FIXED_U128_DECIMALS);
    this.argonUsdPrice = fromFixedNumber(value.argonUsdPrice.toBigInt(), FIXED_U128_DECIMALS);
    this.argonUsdTargetPrice = fromFixedNumber(
      value.argonUsdTargetPrice.toBigInt(),
      FIXED_U128_DECIMALS,
    );
    this.argonTimeWeightedAverageLiquidity = fromFixedNumber(
      value.argonTimeWeightedAverageLiquidity.toBigInt(),
      FIXED_U128_DECIMALS,
    );
    this.lastUpdatedTick = value.tick.toNumber();
    return this;
  }

  getBtcMicrogonPrice(satoshis: bigint | number): bigint {
    if (this.btcUsdPrice === undefined || this.argonUsdPrice === undefined) {
      throw new Error('PriceIndex not loaded');
    }

    const satoshiCents = this.btcUsdPrice.multipliedBy(satoshis).dividedBy(SATS_PER_BTC);

    const microgons = satoshiCents.multipliedBy(MICROGONS_PER_ARGON).dividedBy(this.argonUsdPrice);
    return BigInt(microgons.integerValue(BigNumber.ROUND_DOWN).toString());
  }

  get rValue(): BigNumber {
    if (this.argonUsdTargetPrice === undefined || this.argonUsdPrice === undefined) {
      throw new Error('PriceIndex not loaded');
    }
    return this.argonUsdPrice.div(this.argonUsdTargetPrice);
  }

  get argonCpi(): BigNumber {
    if (this.argonUsdTargetPrice === undefined || this.argonUsdPrice === undefined) {
      throw new Error('PriceIndex not loaded');
    }
    const ratio = this.argonUsdTargetPrice.div(this.argonUsdPrice);
    return ratio.minus(1);
  }
}
