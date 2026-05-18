import type { Address, ContractFunctionArgs, Hex } from 'viem';
import { encodeAbiParameters, keccak256, stringToHex } from 'viem';
import { mintingGatewayV2Abi } from './generated.js';

export const MINTING_GATEWAY_V2_UPDATE_KINDS = {
  globalIssuanceCouncilRotate: 0,
  mintingAuthorityActivate: 1,
  mintingAuthorityDeactivate: 2,
} as const;

export type MintingGatewayV2HashContext = {
  chainId: bigint;
  gatewayAddress: Address;
};

type ApplyGatewayUpdatesArgs = ContractFunctionArgs<
  typeof mintingGatewayV2Abi,
  'nonpayable',
  'applyGatewayUpdates'
>;
type FinalizeTransferOutOfArgonArgs = ContractFunctionArgs<
  typeof mintingGatewayV2Abi,
  'nonpayable',
  'finalizeTransferOutOfArgon'
>;
type MigrateArgs = ContractFunctionArgs<typeof mintingGatewayV2Abi, 'nonpayable', 'migrate'>;

export type MintingGatewayV2CouncilSnapshot = ApplyGatewayUpdatesArgs[0];
export type MintingGatewayV2GatewayUpdate = ApplyGatewayUpdatesArgs[1][number];
export type MintingGatewayV2TransferOutOfArgonRequest = FinalizeTransferOutOfArgonArgs[0];
export type MintingGatewayV2TransferOutOfArgonProof = FinalizeTransferOutOfArgonArgs[1];
export type MintingGatewayV2MintingAuthorization =
  MintingGatewayV2TransferOutOfArgonProof['authorizations'][number];
export type MintingGatewayV2MigrationAssetDistribution = MigrateArgs[0];
export type MintingGatewayV2GlobalIssuanceCouncilRotateTarget = {
  council: MintingGatewayV2CouncilSnapshot;
  microgonsPerArgonot: bigint;
};

export type MintingGatewayV2MintingAuthorityActivationTarget = {
  mintingAuthorityId: Hex;
  microgonCollateral: bigint;
  micronotCollateral: bigint;
  signingKey: Address;
};

export type MintingGatewayV2MintingAuthorityDeactivateTarget = {
  signingKey: Address;
};

type ActivateMintingAuthorityApprovalArgs = {
  queueNonce: bigint;
  approvingCouncilHash: Hex;
  previousUpdateHash: Hex;
  target: MintingGatewayV2MintingAuthorityActivationTarget;
};

type RotateGlobalIssuanceCouncilApprovalArgs = {
  queueNonce: bigint;
  approvingCouncilHash: Hex;
  previousUpdateHash: Hex;
  target: MintingGatewayV2GlobalIssuanceCouncilRotateTarget;
};

type MintingAuthorizationHashArgs = {
  request: MintingGatewayV2TransferOutOfArgonRequest;
  microgonCollateral: MintingGatewayV2MintingAuthorization['microgonCollateral'];
  micronotCollateral: MintingGatewayV2MintingAuthorization['micronotCollateral'];
};

type MintingAuthorityDeactivationHashArgs = {
  queueNonce: bigint;
  target: MintingGatewayV2MintingAuthorityDeactivateTarget;
  previousUpdateHash: Hex;
};

type GatewayUpdateApprovalArgs = {
  queueNonce: bigint;
  approvingCouncilHash: Hex;
  kind: (typeof MINTING_GATEWAY_V2_UPDATE_KINDS)[keyof typeof MINTING_GATEWAY_V2_UPDATE_KINDS];
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

export function encodeMintingGatewayV2CouncilSnapshot(
  snapshot: MintingGatewayV2CouncilSnapshot,
): Hex {
  return encodeAbiParameters(
    [{ type: 'tuple', components: COUNCIL_SNAPSHOT_PARAMETERS }],
    [snapshot],
  );
}

export function encodeMintingGatewayV2GlobalIssuanceCouncilRotateTarget(
  target: MintingGatewayV2GlobalIssuanceCouncilRotateTarget,
): Hex {
  return encodeAbiParameters(GLOBAL_ISSUANCE_COUNCIL_ROTATE_TARGET_PARAMETERS, [target]);
}

export function encodeMintingGatewayV2MintingAuthorityActivationTarget(
  target: MintingGatewayV2MintingAuthorityActivationTarget,
): Hex {
  return encodeAbiParameters(ACTIVATION_TARGET_PARAMETERS, [target]);
}

export function encodeMintingGatewayV2MintingAuthorityDeactivateTarget(
  target: MintingGatewayV2MintingAuthorityDeactivateTarget,
): Hex {
  return encodeAbiParameters(DEACTIVATION_TARGET_PARAMETERS, [target]);
}

export function hashMintingGatewayV2GlobalIssuanceCouncil(
  snapshot: MintingGatewayV2CouncilSnapshot,
): Hex {
  return keccak256(
    encodeAbiParameters(COUNCIL_SNAPSHOT_PARAMETERS, [snapshot.signers, snapshot.weights]),
  );
}

export function hashMintingGatewayV2MintingAuthority(
  target: MintingGatewayV2MintingAuthorityActivationTarget,
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

export function hashMintingGatewayV2ActivateMintingAuthorityApproval(
  context: MintingGatewayV2HashContext,
  args: ActivateMintingAuthorityApprovalArgs,
): Hex {
  return hashMintingGatewayV2GatewayUpdateApproval(context, {
    queueNonce: args.queueNonce,
    approvingCouncilHash: args.approvingCouncilHash,
    kind: MINTING_GATEWAY_V2_UPDATE_KINDS.mintingAuthorityActivate,
    targetId: args.target.mintingAuthorityId,
    targetPayloadHash: hashMintingGatewayV2ActivateMintingAuthority(context, args.target),
    previousUpdateHash: args.previousUpdateHash,
  });
}

export function hashMintingGatewayV2RotateGlobalIssuanceCouncilApproval(
  context: MintingGatewayV2HashContext,
  args: RotateGlobalIssuanceCouncilApprovalArgs,
): Hex {
  const nextCouncilHash = hashMintingGatewayV2GlobalIssuanceCouncil(args.target.council);

  return hashMintingGatewayV2GatewayUpdateApproval(context, {
    queueNonce: args.queueNonce,
    approvingCouncilHash: args.approvingCouncilHash,
    kind: MINTING_GATEWAY_V2_UPDATE_KINDS.globalIssuanceCouncilRotate,
    targetId: nextCouncilHash,
    targetPayloadHash: hashMintingGatewayV2RotateGlobalIssuanceCouncil(context, args.target),
    previousUpdateHash: args.previousUpdateHash,
  });
}

export function hashMintingGatewayV2ActivateMintingAuthority(
  context: MintingGatewayV2HashContext,
  target: MintingGatewayV2MintingAuthorityActivationTarget,
): Hex {
  return keccak256(
    encodeAbiParameters(MINTING_AUTHORITY_ACTIVATION_PARAMETERS, [
      MINTING_AUTHORITY_ACTIVATION_TAG,
      context.chainId,
      context.gatewayAddress,
      hashMintingGatewayV2MintingAuthority(target),
    ]),
  );
}

export function hashMintingGatewayV2RotateGlobalIssuanceCouncil(
  context: MintingGatewayV2HashContext,
  target: MintingGatewayV2GlobalIssuanceCouncilRotateTarget,
): Hex {
  return keccak256(
    encodeAbiParameters(GLOBAL_ISSUANCE_COUNCIL_ROTATION_PARAMETERS, [
      GLOBAL_ISSUANCE_COUNCIL_ROTATION_TAG,
      context.chainId,
      context.gatewayAddress,
      hashMintingGatewayV2GlobalIssuanceCouncil(target.council),
      target.microgonsPerArgonot,
    ]),
  );
}

export function hashMintingGatewayV2GatewayUpdateApproval(
  context: MintingGatewayV2HashContext,
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

export function hashMintingGatewayV2MintingAuthorityDeactivation(
  context: MintingGatewayV2HashContext,
  args: MintingAuthorityDeactivationHashArgs,
): Hex {
  return keccak256(
    encodeAbiParameters(MINTING_AUTHORITY_DEACTIVATION_PARAMETERS, [
      MINTING_AUTHORITY_DEACTIVATION_TAG,
      context.chainId,
      context.gatewayAddress,
      args.queueNonce,
      args.target.signingKey,
      args.previousUpdateHash,
    ]),
  );
}

export function hashMintingGatewayV2TransferOutOfArgonRequest(
  request: MintingGatewayV2TransferOutOfArgonRequest,
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

export function hashMintingGatewayV2MintingAuthorization(
  context: MintingGatewayV2HashContext,
  args: MintingAuthorizationHashArgs,
): Hex {
  return keccak256(
    encodeAbiParameters(MINTING_AUTHORIZATION_PARAMETERS, [
      TRANSFER_OUT_OF_ARGON_AUTHORIZATION_TAG,
      context.chainId,
      context.gatewayAddress,
      hashMintingGatewayV2TransferOutOfArgonRequest(args.request),
      args.microgonCollateral,
      args.micronotCollateral,
    ]),
  );
}
