export type MintingAuthorityActivationGasMeasurements = {
  singleActivationGas: bigint;
  batchActivationGas: bigint;
  batchActivationCount: number;
  sharedSignatureCount: number;
  oneMemberSingleActivationGas?: bigint;
  oneMemberSharedSignatureCount?: number;
  smallCouncilSingleActivationGas?: bigint;
  smallCouncilSharedSignatureCount?: number;
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
  targetSharedSignatureCount = measurements.sharedSignatureCount,
): MintingAuthorityActivationPricingRecommendation {
  const {
    singleActivationGas,
    batchActivationGas,
    batchActivationCount,
    sharedSignatureCount,
    oneMemberSingleActivationGas,
    oneMemberSharedSignatureCount,
    smallCouncilSingleActivationGas,
    smallCouncilSharedSignatureCount,
  } = measurements;

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
  if (targetSharedSignatureCount < 1) {
    throw new Error('targetSharedSignatureCount must be at least 1');
  }

  const activationBatchMarginalGas = divideCeil(
    batchActivationGas - singleActivationGas,
    BigInt(batchActivationCount - 1),
  );
  const activationAnchors = [
    {
      sharedSignatureCount,
      singleActivationGas,
    },
  ];

  if (oneMemberSingleActivationGas !== undefined || oneMemberSharedSignatureCount !== undefined) {
    if (oneMemberSingleActivationGas === undefined || oneMemberSharedSignatureCount === undefined) {
      throw new Error(
        'oneMemberSingleActivationGas and oneMemberSharedSignatureCount must be provided together',
      );
    }
    if (oneMemberSingleActivationGas <= activationBatchMarginalGas) {
      throw new Error('oneMemberSingleActivationGas must exceed activationBatchMarginalGas');
    }
    if (
      oneMemberSharedSignatureCount < 1 ||
      oneMemberSharedSignatureCount >= sharedSignatureCount
    ) {
      throw new Error(
        'oneMemberSharedSignatureCount must be at least 1 and less than sharedSignatureCount',
      );
    }

    activationAnchors.push({
      sharedSignatureCount: oneMemberSharedSignatureCount,
      singleActivationGas: oneMemberSingleActivationGas,
    });
  }

  if (
    smallCouncilSingleActivationGas !== undefined ||
    smallCouncilSharedSignatureCount !== undefined
  ) {
    if (
      smallCouncilSingleActivationGas === undefined ||
      smallCouncilSharedSignatureCount === undefined
    ) {
      throw new Error(
        'smallCouncilSingleActivationGas and smallCouncilSharedSignatureCount must be provided together',
      );
    }
    if (smallCouncilSingleActivationGas <= activationBatchMarginalGas) {
      throw new Error('smallCouncilSingleActivationGas must exceed activationBatchMarginalGas');
    }
    if (
      smallCouncilSharedSignatureCount < 1 ||
      smallCouncilSharedSignatureCount >= sharedSignatureCount
    ) {
      throw new Error(
        'smallCouncilSharedSignatureCount must be at least 1 and less than sharedSignatureCount',
      );
    }

    activationAnchors.push({
      sharedSignatureCount: smallCouncilSharedSignatureCount,
      singleActivationGas: smallCouncilSingleActivationGas,
    });
  }

  if (activationAnchors.length > 1) {
    const anchorBySharedSignatureCount = new Map<number, bigint>();

    for (const anchor of activationAnchors) {
      anchorBySharedSignatureCount.set(
        anchor.sharedSignatureCount,
        maxBigInt(
          anchorBySharedSignatureCount.get(anchor.sharedSignatureCount) ?? 0n,
          anchor.singleActivationGas,
        ),
      );
    }

    const consolidatedAnchors = [...anchorBySharedSignatureCount]
      .map(([anchorSharedSignatureCount, anchorSingleActivationGas]) => ({
        sharedSignatureCount: anchorSharedSignatureCount,
        singleActivationGas: anchorSingleActivationGas,
      }))
      .sort((left, right) => left.sharedSignatureCount - right.sharedSignatureCount);

    let variableSignatureGasCost = 0n;

    for (let leftIndex = 0; leftIndex < consolidatedAnchors.length - 1; ++leftIndex) {
      const leftAnchor = consolidatedAnchors[leftIndex]!;
      for (let rightIndex = leftIndex + 1; rightIndex < consolidatedAnchors.length; ++rightIndex) {
        const rightAnchor = consolidatedAnchors[rightIndex]!;
        const deltaGas =
          rightAnchor.singleActivationGas > leftAnchor.singleActivationGas
            ? rightAnchor.singleActivationGas - leftAnchor.singleActivationGas
            : 0n;
        const deltaSharedSignatureCount =
          rightAnchor.sharedSignatureCount - leftAnchor.sharedSignatureCount;

        variableSignatureGasCost = maxBigInt(
          variableSignatureGasCost,
          divideCeil(deltaGas, BigInt(deltaSharedSignatureCount)),
        );
      }
    }

    let sharedFixedGas = 0n;

    for (const anchor of consolidatedAnchors) {
      sharedFixedGas = maxBigInt(
        sharedFixedGas,
        anchor.singleActivationGas -
          activationBatchMarginalGas -
          variableSignatureGasCost * BigInt(anchor.sharedSignatureCount),
      );
    }

    const signatureGasCost =
      variableSignatureGasCost + divideCeil(sharedFixedGas, BigInt(targetSharedSignatureCount));
    const sharedSignatureGasTotal = signatureGasCost * BigInt(targetSharedSignatureCount);
    const quotedSingleActivationGas = activationBatchMarginalGas + sharedSignatureGasTotal;

    return {
      activationGasCost: activationBatchMarginalGas,
      signatureGasCost,
      quotedSingleActivationGas,
      activationBatchMarginalGas,
      sharedSignatureGasTotal,
      note: `Conservative split derived from ${consolidatedAnchors.length} council activation measurements, calibrated for ${targetSharedSignatureCount} shared signatures per activation tranche. Any excess hold is expected to refund on proof-back.`,
    };
  }

  const sharedSignatureCountBigInt = BigInt(sharedSignatureCount);

  const sharedSignatureGasTotal = singleActivationGas - activationBatchMarginalGas;
  const signatureGasCost = divideCeil(sharedSignatureGasTotal, sharedSignatureCountBigInt);
  const quotedSingleActivationGas =
    activationBatchMarginalGas + signatureGasCost * sharedSignatureCountBigInt;

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

function maxBigInt(left: bigint, right: bigint) {
  return left > right ? left : right;
}
