import type { Address, ContractFunctionArgs, Hex } from 'viem';
import { encodeAbiParameters, keccak256, stringToHex } from 'viem';
import { mintingGatewayAbi } from './generated.js';

export const MINTING_GATEWAY_UPDATE_KINDS = {
  globalIssuanceCouncilRotate: 0,
  mintingAuthorityActivate: 1,
  mintingAuthorityDeactivate: 2,
} as const;

export type MintingGatewayHashContext = {
  chainId: bigint;
  gatewayAddress: Address;
};

type ApplyGatewayUpdatesArgs = ContractFunctionArgs<
  typeof mintingGatewayAbi,
  'nonpayable',
  'applyGatewayUpdates'
>;
type FinalizeTransferOutOfArgonArgs = ContractFunctionArgs<
  typeof mintingGatewayAbi,
  'nonpayable',
  'finalizeTransferOutOfArgon'
>;
type MigrateArgs = ContractFunctionArgs<typeof mintingGatewayAbi, 'nonpayable', 'migrate'>;

export type MintingGatewayCouncilSnapshot = ApplyGatewayUpdatesArgs[0];
export type MintingGatewayGatewayUpdate = ApplyGatewayUpdatesArgs[1][number];
export type MintingGatewayTransferOutOfArgonRequest = FinalizeTransferOutOfArgonArgs[0];
export type MintingGatewayTransferOutOfArgonProof = FinalizeTransferOutOfArgonArgs[1];
export type MintingGatewayMintingAuthorization =
  MintingGatewayTransferOutOfArgonProof['authorizations'][number];
export type MintingGatewayMigrationAssetDistribution = MigrateArgs[0];
export type MintingGatewayGlobalIssuanceCouncilRotateTarget = {
  council: MintingGatewayCouncilSnapshot;
  microgonsPerArgonot: bigint;
};

export type MintingGatewayMintingAuthorityActivationTarget = {
  mintingAuthorityId: Hex;
  microgonCollateral: bigint;
  micronotCollateral: bigint;
  signingKey: Address;
};

export type MintingGatewayMintingAuthorityDeactivateTarget = {
  mintingAuthorityId: Hex;
  signingKey: Address;
};

type ActivateMintingAuthorityApprovalArgs = {
  queueNonce: bigint;
  approvingCouncilHash: Hex;
  previousUpdateHash: Hex;
  target: MintingGatewayMintingAuthorityActivationTarget;
};

type RotateGlobalIssuanceCouncilApprovalArgs = {
  queueNonce: bigint;
  approvingCouncilHash: Hex;
  previousUpdateHash: Hex;
  target: MintingGatewayGlobalIssuanceCouncilRotateTarget;
};

type MintingAuthorizationHashArgs = {
  request: MintingGatewayTransferOutOfArgonRequest;
  microgonCollateral: MintingGatewayMintingAuthorization['microgonCollateral'];
  micronotCollateral: MintingGatewayMintingAuthorization['micronotCollateral'];
};

type MintingAuthorityDeactivationHashArgs = {
  queueNonce: bigint;
  target: MintingGatewayMintingAuthorityDeactivateTarget;
  previousUpdateHash: Hex;
};

type GatewayUpdateApprovalArgs = {
  queueNonce: bigint;
  approvingCouncilHash: Hex;
  kind: (typeof MINTING_GATEWAY_UPDATE_KINDS)[keyof typeof MINTING_GATEWAY_UPDATE_KINDS];
  targetId: Hex;
  targetPayloadHash: Hex;
  previousUpdateHash: Hex;
};

const COUNCIL_SNAPSHOT_PARAMETERS = [
  { name: 'signers', type: 'address[]' },
  { name: 'weights', type: 'uint256[]' },
] as const;

const GLOBAL_ISSUANCE_COUNCIL_ROTATE_TARGET_PARAMETERS = [
  {
    type: 'tuple',
    components: [
      {
        name: 'council',
        type: 'tuple',
        components: COUNCIL_SNAPSHOT_PARAMETERS,
      },
      { name: 'microgonsPerArgonot', type: 'uint128' },
    ],
  },
] as const;

const ACTIVATION_TARGET_PARAMETERS = [
  {
    type: 'tuple',
    components: [
      { name: 'mintingAuthorityId', type: 'bytes32' },
      { name: 'microgonCollateral', type: 'uint128' },
      { name: 'micronotCollateral', type: 'uint128' },
      { name: 'signingKey', type: 'address' },
    ],
  },
] as const;

const DEACTIVATION_TARGET_PARAMETERS = [
  {
    type: 'tuple',
    components: [
      { name: 'mintingAuthorityId', type: 'bytes32' },
      { name: 'signingKey', type: 'address' },
    ],
  },
] as const;

const GATEWAY_UPDATE_APPROVAL_PARAMETERS = [
  { type: 'bytes32' },
  { type: 'uint256' },
  { type: 'address' },
  { type: 'uint64' },
  { type: 'bytes32' },
  { type: 'uint8' },
  { type: 'bytes32' },
  { type: 'bytes32' },
  { type: 'bytes32' },
] as const;

const MINTING_AUTHORITY_HASH_PARAMETERS = [
  { type: 'bytes32' },
  { type: 'uint128' },
  { type: 'uint128' },
  { type: 'address' },
] as const;

const MINTING_AUTHORITY_ACTIVATION_PARAMETERS = [
  { type: 'bytes32' },
  { type: 'uint256' },
  { type: 'address' },
  { type: 'bytes32' },
] as const;

const GLOBAL_ISSUANCE_COUNCIL_ROTATION_PARAMETERS = [
  { type: 'bytes32' },
  { type: 'uint256' },
  { type: 'address' },
  { type: 'bytes32' },
  { type: 'uint128' },
] as const;

const MINTING_AUTHORITY_DEACTIVATION_PARAMETERS = [
  { type: 'bytes32' },
  { type: 'uint256' },
  { type: 'address' },
  { type: 'uint64' },
  { type: 'bytes32' },
  { type: 'address' },
  { type: 'bytes32' },
] as const;

const TRANSFER_OUT_OF_ARGON_REQUEST_PARAMETERS = [
  { type: 'bytes32' },
  { type: 'uint64' },
  { type: 'uint64' },
  { type: 'uint64' },
  { type: 'address' },
  { type: 'uint64' },
  { type: 'address' },
  { type: 'uint128' },
  { type: 'uint128' },
] as const;

const MINTING_AUTHORIZATION_PARAMETERS = [
  { type: 'bytes32' },
  { type: 'uint256' },
  { type: 'address' },
  { type: 'bytes32' },
  { type: 'uint128' },
  { type: 'uint128' },
] as const;

const GLOBAL_ISSUANCE_COUNCIL_ROTATION_TAG = keccak256(
  stringToHex('ARGON_GLOBAL_ISSUANCE_COUNCIL_ROTATION'),
);
const MINTING_AUTHORITY_ACTIVATION_TAG = keccak256(
  stringToHex('ARGON_MINTING_AUTHORITY_ACTIVATION'),
);
const MINTING_AUTHORITY_DEACTIVATION_TAG = keccak256(
  stringToHex('ARGON_MINTING_AUTHORITY_DEACTIVATION'),
);
const GATEWAY_UPDATE_APPROVAL_TAG = keccak256(stringToHex('ARGON_GATEWAY_UPDATE'));
const TRANSFER_OUT_OF_ARGON_AUTHORIZATION_TAG = keccak256(
  stringToHex('ARGON_TRANSFER_OUT_OF_ARGON_AUTHORIZATION'),
);

export function encodeMintingGatewayCouncilSnapshot(snapshot: MintingGatewayCouncilSnapshot): Hex {
  return encodeAbiParameters(
    [{ type: 'tuple', components: COUNCIL_SNAPSHOT_PARAMETERS }],
    [snapshot],
  );
}

export function encodeMintingGatewayGlobalIssuanceCouncilRotateTarget(
  target: MintingGatewayGlobalIssuanceCouncilRotateTarget,
): Hex {
  return encodeAbiParameters(GLOBAL_ISSUANCE_COUNCIL_ROTATE_TARGET_PARAMETERS, [target]);
}

export function encodeMintingGatewayMintingAuthorityActivationTarget(
  target: MintingGatewayMintingAuthorityActivationTarget,
): Hex {
  return encodeAbiParameters(ACTIVATION_TARGET_PARAMETERS, [target]);
}

export function encodeMintingGatewayMintingAuthorityDeactivateTarget(
  target: MintingGatewayMintingAuthorityDeactivateTarget,
): Hex {
  return encodeAbiParameters(DEACTIVATION_TARGET_PARAMETERS, [target]);
}

export function hashMintingGatewayGlobalIssuanceCouncil(
  snapshot: MintingGatewayCouncilSnapshot,
): Hex {
  return keccak256(
    encodeAbiParameters(COUNCIL_SNAPSHOT_PARAMETERS, [snapshot.signers, snapshot.weights]),
  );
}

export function hashMintingGatewayMintingAuthority(
  target: MintingGatewayMintingAuthorityActivationTarget,
): Hex {
  return keccak256(
    encodeAbiParameters(MINTING_AUTHORITY_HASH_PARAMETERS, [
      target.mintingAuthorityId,
      target.microgonCollateral,
      target.micronotCollateral,
      target.signingKey,
    ]),
  );
}

export function hashMintingGatewayActivateMintingAuthorityApproval(
  context: MintingGatewayHashContext,
  args: ActivateMintingAuthorityApprovalArgs,
): Hex {
  return hashMintingGatewayGatewayUpdateApproval(context, {
    queueNonce: args.queueNonce,
    approvingCouncilHash: args.approvingCouncilHash,
    kind: MINTING_GATEWAY_UPDATE_KINDS.mintingAuthorityActivate,
    targetId: args.target.mintingAuthorityId,
    targetPayloadHash: hashMintingGatewayActivateMintingAuthority(context, args.target),
    previousUpdateHash: args.previousUpdateHash,
  });
}

export function hashMintingGatewayRotateGlobalIssuanceCouncilApproval(
  context: MintingGatewayHashContext,
  args: RotateGlobalIssuanceCouncilApprovalArgs,
): Hex {
  const nextCouncilHash = hashMintingGatewayGlobalIssuanceCouncil(args.target.council);

  return hashMintingGatewayGatewayUpdateApproval(context, {
    queueNonce: args.queueNonce,
    approvingCouncilHash: args.approvingCouncilHash,
    kind: MINTING_GATEWAY_UPDATE_KINDS.globalIssuanceCouncilRotate,
    targetId: nextCouncilHash,
    targetPayloadHash: hashMintingGatewayRotateGlobalIssuanceCouncil(context, args.target),
    previousUpdateHash: args.previousUpdateHash,
  });
}

export function hashMintingGatewayActivateMintingAuthority(
  context: MintingGatewayHashContext,
  target: MintingGatewayMintingAuthorityActivationTarget,
): Hex {
  return keccak256(
    encodeAbiParameters(MINTING_AUTHORITY_ACTIVATION_PARAMETERS, [
      MINTING_AUTHORITY_ACTIVATION_TAG,
      context.chainId,
      context.gatewayAddress,
      hashMintingGatewayMintingAuthority(target),
    ]),
  );
}

export function hashMintingGatewayRotateGlobalIssuanceCouncil(
  context: MintingGatewayHashContext,
  target: MintingGatewayGlobalIssuanceCouncilRotateTarget,
): Hex {
  return keccak256(
    encodeAbiParameters(GLOBAL_ISSUANCE_COUNCIL_ROTATION_PARAMETERS, [
      GLOBAL_ISSUANCE_COUNCIL_ROTATION_TAG,
      context.chainId,
      context.gatewayAddress,
      hashMintingGatewayGlobalIssuanceCouncil(target.council),
      target.microgonsPerArgonot,
    ]),
  );
}

export function hashMintingGatewayGatewayUpdateApproval(
  context: MintingGatewayHashContext,
  args: GatewayUpdateApprovalArgs,
): Hex {
  return keccak256(
    encodeAbiParameters(GATEWAY_UPDATE_APPROVAL_PARAMETERS, [
      GATEWAY_UPDATE_APPROVAL_TAG,
      context.chainId,
      context.gatewayAddress,
      args.queueNonce,
      args.approvingCouncilHash,
      args.kind,
      args.targetId,
      args.targetPayloadHash,
      args.previousUpdateHash,
    ]),
  );
}

export function hashMintingGatewayMintingAuthorityDeactivation(
  context: MintingGatewayHashContext,
  args: MintingAuthorityDeactivationHashArgs,
): Hex {
  return keccak256(
    encodeAbiParameters(MINTING_AUTHORITY_DEACTIVATION_PARAMETERS, [
      MINTING_AUTHORITY_DEACTIVATION_TAG,
      context.chainId,
      context.gatewayAddress,
      args.queueNonce,
      args.target.mintingAuthorityId,
      args.target.signingKey,
      args.previousUpdateHash,
    ]),
  );
}

export function hashMintingGatewayTransferOutOfArgonRequest(
  request: MintingGatewayTransferOutOfArgonRequest,
): Hex {
  return keccak256(
    encodeAbiParameters(TRANSFER_OUT_OF_ARGON_REQUEST_PARAMETERS, [
      request.argonAccountId,
      request.argonTransferNonce,
      request.chainId,
      request.councilNumber,
      request.recipient,
      request.validUntilBlock,
      request.token,
      request.amount,
      request.finalizationTip,
    ]),
  );
}

export function hashMintingGatewayMintingAuthorization(
  context: MintingGatewayHashContext,
  args: MintingAuthorizationHashArgs,
): Hex {
  return keccak256(
    encodeAbiParameters(MINTING_AUTHORIZATION_PARAMETERS, [
      TRANSFER_OUT_OF_ARGON_AUTHORIZATION_TAG,
      context.chainId,
      context.gatewayAddress,
      hashMintingGatewayTransferOutOfArgonRequest(args.request),
      args.microgonCollateral,
      args.micronotCollateral,
    ]),
  );
}
