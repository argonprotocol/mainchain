import BigNumber from 'bignumber.js';

export function toFixedNumber(
  value: string | number | BigNumber, // accept string to avoid early precision loss
  decimals: number,
): bigint {
  const factor = new BigNumber(10).pow(decimals);
  const bn = new BigNumber(value);
  // truncate toward 0; use ROUND_FLOOR if you really need floor for positives
  const int = bn.times(factor).integerValue(BigNumber.ROUND_DOWN);
  return BigInt(int.toFixed(0));
}

export function fromFixedNumber(value: bigint, decimals: number = FIXED_U128_DECIMALS): BigNumber {
  const factor = new BigNumber(10).pow(decimals);
  const bn = new BigNumber(value.toString());
  return bn.div(factor);
}

export const FIXED_U128_DECIMALS = 18;
export const PERMILL_DECIMALS = 6;
