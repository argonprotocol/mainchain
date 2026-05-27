export type MintingAuthorityActivationGasMeasurements = {
  singleActivationGas: bigint;
  batchActivationGas: bigint;
  batchActivationCount: number;
  sharedSignatureCount: number;
};

export type MintingAuthorityActivationPricingRecommendation = {
  activationGasCost: bigint;
  signatureGasCost: bigint;
  quotedSingleActivationGas: bigint;
  activationBatchMarginalGas: bigint;
  sharedSignatureGasTotal: bigint;
  note: string;
};

export function deriveMintingAuthorityActivationPricingRecommendation(
  measurements: MintingAuthorityActivationGasMeasurements,
): MintingAuthorityActivationPricingRecommendation {
  const { singleActivationGas, batchActivationGas, batchActivationCount, sharedSignatureCount } =
    measurements;

  if (singleActivationGas <= 0n) {
    throw new Error('singleActivationGas must be greater than zero');
  }
  if (batchActivationGas <= singleActivationGas) {
    throw new Error('batchActivationGas must be greater than singleActivationGas');
  }
  if (batchActivationCount < 2) {
    throw new Error('batchActivationCount must be at least 2');
  }
  if (sharedSignatureCount < 1) {
    throw new Error('sharedSignatureCount must be at least 1');
  }

  const activationBatchMarginalGas = divideCeil(
    batchActivationGas - singleActivationGas,
    BigInt(batchActivationCount - 1),
  );
  const sharedSignatureGasTotal = singleActivationGas - activationBatchMarginalGas;
  const signatureGasCost = divideCeil(sharedSignatureGasTotal, BigInt(sharedSignatureCount));
  const quotedSingleActivationGas =
    activationBatchMarginalGas + signatureGasCost * BigInt(sharedSignatureCount);

  return {
    activationGasCost: activationBatchMarginalGas,
    signatureGasCost,
    quotedSingleActivationGas,
    activationBatchMarginalGas,
    sharedSignatureGasTotal,
    note: 'Conservative split derived from one single-activation measurement and one shared-signature activation batch. Any excess hold is expected to refund on proof-back.',
  };
}

function divideCeil(dividend: bigint, divisor: bigint) {
  if (divisor <= 0n) {
    throw new Error('divisor must be greater than zero');
  }

  return (dividend + divisor - 1n) / divisor;
}
