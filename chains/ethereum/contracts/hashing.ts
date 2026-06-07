import type { Address, ContractFunctionArgs, Hex } from 'viem';
import { encodeAbiParameters, keccak256, stringToHex } from 'viem';
import type {
  MintingGatewayActivityState,
  MintingGatewayMintingAuthorityCollateral,
} from './generated.js';
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
export type MintingGatewayActivityBlockLocator = {
  blockNumber: bigint;
  startGatewayActivityNonce: bigint;
  endGatewayActivityNonce: bigint;
  activityRoot: Hex;
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
  epochMicrogonsPerArgonot: bigint;
};
export type MintingGatewayGlobalIssuanceCouncilHashArgs = MintingGatewayCouncilSnapshot & {
  epochMicrogonsPerArgonot: bigint;
};

export type MintingGatewayMintingAuthorityActivationTarget = {
  microgonCollateral: bigint;
  micronotCollateral: bigint;
  signingKey: Address;
};

export type MintingGatewayMintingAuthorityDeactivateTarget = {
  signingKey: Address;
};
type GlobalIssuanceCouncilRotatedActivityHashArgs = {
  councilHash: Hex;
  approvalHash: Hex;
  relayerArgonAccountId: Hex;
  gatewayState: MintingGatewayActivityState;
};
type MintingAuthorityActivatedActivityHashArgs = {
  signingKey: Address;
  microgonCollateral: bigint;
  micronotCollateral: bigint;
  coactivationCount: number | bigint;
  sharedSignatureCount: number | bigint;
  approvalHash: Hex;
  relayerArgonAccountId: Hex;
  gatewayState: MintingGatewayActivityState;
};
type MintingAuthorityDeactivatedActivityHashArgs = {
  signingKey: Address;
  microgonCollateral: bigint;
  micronotCollateral: bigint;
  approvalHash: Hex;
  relayerArgonAccountId: Hex;
  gatewayState: MintingGatewayActivityState;
};
type TransferToArgonStartedActivityHashArgs = {
  from: Address;
  token: Address;
  amount: bigint;
  argonAccountId: Hex;
  gatewayState: MintingGatewayActivityState;
};
type TransferOutOfArgonCanceledActivityHashArgs = {
  transferId: Hex;
  gatewayState: MintingGatewayActivityState;
};
type TransferOutOfArgonFinalizedActivityHashArgs = {
  transferId: Hex;
  token: Address;
  amount: bigint;
  mintingCollateral: MintingGatewayMintingAuthorityCollateral[];
  gatewayState: MintingGatewayActivityState;
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
const GATEWAY_ACTIVITY_STATE_PARAMETERS = [
  { name: 'gatewayActivityNonce', type: 'uint64' },
  { name: 'argonApprovalsNonce', type: 'uint64' },
  { name: 'argonCirculation', type: 'uint128' },
  { name: 'argonotCirculation', type: 'uint128' },
] as const;
const MINTING_AUTHORITY_COLLATERAL_PARAMETERS = [
  { name: 'signingKey', type: 'address' },
  { name: 'microgonCollateral', type: 'uint128' },
  { name: 'micronotCollateral', type: 'uint128' },
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
      { name: 'epochMicrogonsPerArgonot', type: 'uint128' },
    ],
  },
] as const;

const ACTIVATION_TARGET_PARAMETERS = [
  {
    type: 'tuple',
    components: [
      { name: 'microgonCollateral', type: 'uint128' },
      { name: 'micronotCollateral', type: 'uint128' },
      { name: 'signingKey', type: 'address' },
    ],
  },
] as const;

const DEACTIVATION_TARGET_PARAMETERS = [
  {
    type: 'tuple',
    components: [{ name: 'signingKey', type: 'address' }],
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

function signingKeyTargetId(signingKey: Address): Hex {
  return `0x${signingKey.slice(2).padStart(64, '0').toLowerCase()}`;
}

const TRANSFER_OUT_OF_ARGON_REQUEST_PARAMETERS = [
  { type: 'bytes32' },
  { type: 'uint64' },
  { type: 'uint64' },
  { type: 'uint128' },
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
const GATEWAY_ACTIVITY_HASH_PARAMETERS = [
  { type: 'bytes32' },
  { type: 'uint8' },
  { type: 'uint256' },
  { type: 'address' },
] as const;
const GLOBAL_ISSUANCE_COUNCIL_ROTATED_ACTIVITY_PARAMETERS = [
  ...GATEWAY_ACTIVITY_HASH_PARAMETERS,
  { type: 'bytes32' },
  { type: 'bytes32' },
  { type: 'bytes32' },
  { type: 'bytes32' },
] as const;
const MINTING_AUTHORITY_ACTIVATED_ACTIVITY_PARAMETERS = [
  ...GATEWAY_ACTIVITY_HASH_PARAMETERS,
  { type: 'address' },
  { type: 'uint128' },
  { type: 'uint128' },
  { type: 'uint32' },
  { type: 'uint32' },
  { type: 'bytes32' },
  { type: 'bytes32' },
  { type: 'bytes32' },
] as const;
const MINTING_AUTHORITY_DEACTIVATED_ACTIVITY_PARAMETERS = [
  ...GATEWAY_ACTIVITY_HASH_PARAMETERS,
  { type: 'address' },
  { type: 'uint128' },
  { type: 'uint128' },
  { type: 'bytes32' },
  { type: 'bytes32' },
  { type: 'bytes32' },
] as const;
const TRANSFER_TO_ARGON_STARTED_ACTIVITY_PARAMETERS = [
  ...GATEWAY_ACTIVITY_HASH_PARAMETERS,
  { type: 'address' },
  { type: 'address' },
  { type: 'uint128' },
  { type: 'bytes32' },
  { type: 'bytes32' },
] as const;
const TRANSFER_OUT_OF_ARGON_CANCELED_ACTIVITY_PARAMETERS = [
  ...GATEWAY_ACTIVITY_HASH_PARAMETERS,
  { type: 'bytes32' },
  { type: 'bytes32' },
] as const;
const TRANSFER_OUT_OF_ARGON_FINALIZED_ACTIVITY_PARAMETERS = [
  ...GATEWAY_ACTIVITY_HASH_PARAMETERS,
  { type: 'bytes32' },
  { type: 'address' },
  { type: 'uint128' },
  { type: 'bytes32' },
  { type: 'bytes32' },
] as const;

const GLOBAL_ISSUANCE_COUNCIL_ROTATION_TAG = keccak256(
  stringToHex('ARGON_GLOBAL_ISSUANCE_COUNCIL_ROTATION'),
);
const MINTING_AUTHORITY_ACTIVATION_TAG = keccak256(
  stringToHex('ARGON_MINTING_AUTHORITY_ACTIVATION'),
);
const GATEWAY_UPDATE_APPROVAL_TAG = keccak256(stringToHex('ARGON_GATEWAY_UPDATE_APPROVAL'));
const TRANSFER_OUT_OF_ARGON_AUTHORIZATION_TAG = keccak256(
  stringToHex('ARGON_TRANSFER_OUT_OF_ARGON_AUTHORIZATION'),
);
const GLOBAL_ISSUANCE_COUNCIL_ROTATED_ACTIVITY_TAG = keccak256(
  stringToHex('ARGON_GLOBAL_ISSUANCE_COUNCIL_ROTATED_ACTIVITY'),
);
const MINTING_AUTHORITY_ACTIVATED_ACTIVITY_TAG = keccak256(
  stringToHex('ARGON_MINTING_AUTHORITY_ACTIVATED_ACTIVITY'),
);
const MINTING_AUTHORITY_DEACTIVATED_ACTIVITY_TAG = keccak256(
  stringToHex('ARGON_MINTING_AUTHORITY_DEACTIVATED_ACTIVITY'),
);
const TRANSFER_TO_ARGON_STARTED_ACTIVITY_TAG = keccak256(
  stringToHex('ARGON_TRANSFER_TO_ARGON_STARTED_ACTIVITY'),
);
const TRANSFER_OUT_OF_ARGON_CANCELED_ACTIVITY_TAG = keccak256(
  stringToHex('ARGON_TRANSFER_OUT_OF_ARGON_CANCELED_ACTIVITY'),
);
const TRANSFER_OUT_OF_ARGON_FINALIZED_ACTIVITY_TAG = keccak256(
  stringToHex('ARGON_TRANSFER_OUT_OF_ARGON_FINALIZED_ACTIVITY'),
);
export const MINTING_GATEWAY_ACTIVITY_HASH_VERSION = 1;

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
  council: MintingGatewayGlobalIssuanceCouncilHashArgs,
): Hex {
  return keccak256(
    encodeAbiParameters(
      [{ type: 'address[]' }, { type: 'uint256[]' }, { type: 'uint128' }],
      [council.signers, council.weights, council.epochMicrogonsPerArgonot],
    ),
  );
}

export function hashMintingGatewayMintingAuthority(
  target: MintingGatewayMintingAuthorityActivationTarget,
): Hex {
  return keccak256(
    encodeAbiParameters(MINTING_AUTHORITY_HASH_PARAMETERS, [
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
    targetId: signingKeyTargetId(args.target.signingKey),
    targetPayloadHash: hashMintingGatewayActivateMintingAuthority(context, args.target),
    previousUpdateHash: args.previousUpdateHash,
  });
}

export function hashMintingGatewayRotateGlobalIssuanceCouncilApproval(
  context: MintingGatewayHashContext,
  args: RotateGlobalIssuanceCouncilApprovalArgs,
): Hex {
  const nextCouncilHash = hashMintingGatewayGlobalIssuanceCouncil({
    ...args.target.council,
    epochMicrogonsPerArgonot: args.target.epochMicrogonsPerArgonot,
  });

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
      hashMintingGatewayGlobalIssuanceCouncil({
        ...target.council,
        epochMicrogonsPerArgonot: target.epochMicrogonsPerArgonot,
      }),
      target.epochMicrogonsPerArgonot,
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

export function hashMintingGatewayTransferOutOfArgonRequest(
  request: MintingGatewayTransferOutOfArgonRequest,
): Hex {
  return keccak256(
    encodeAbiParameters(TRANSFER_OUT_OF_ARGON_REQUEST_PARAMETERS, [
      request.argonAccountId,
      request.argonTransferNonce,
      request.chainId,
      request.microgonsPerArgonot,
      request.recipient,
      request.validUntilBlock,
      request.token,
      request.amount,
      request.mintingAuthorityTip,
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

export function hashMintingGatewayActivityState(gatewayState: MintingGatewayActivityState): Hex {
  return keccak256(
    encodeAbiParameters(
      [{ type: 'tuple', components: GATEWAY_ACTIVITY_STATE_PARAMETERS }],
      [gatewayState],
    ),
  );
}

export function appendMintingGatewayActivityRoot(currentRoot: Hex, activityHash: Hex): Hex {
  return keccak256(
    encodeAbiParameters(
      [{ type: 'uint8' }, { type: 'bytes32' }, { type: 'bytes32' }],
      [MINTING_GATEWAY_ACTIVITY_HASH_VERSION, currentRoot, activityHash],
    ),
  );
}

export function hashMintingGatewayActivityBlockLocator(
  locator: MintingGatewayActivityBlockLocator,
): Hex {
  return keccak256(
    encodeAbiParameters(
      [
        { type: 'uint8' },
        { type: 'uint64' },
        { type: 'uint64' },
        { type: 'uint64' },
        { type: 'bytes32' },
      ],
      [
        MINTING_GATEWAY_ACTIVITY_HASH_VERSION,
        locator.blockNumber,
        locator.startGatewayActivityNonce,
        locator.endGatewayActivityNonce,
        locator.activityRoot,
      ],
    ),
  );
}

export function hashMintingGatewayGlobalIssuanceCouncilRotatedActivity(
  context: MintingGatewayHashContext,
  args: GlobalIssuanceCouncilRotatedActivityHashArgs,
): Hex {
  return keccak256(
    encodeAbiParameters(GLOBAL_ISSUANCE_COUNCIL_ROTATED_ACTIVITY_PARAMETERS, [
      GLOBAL_ISSUANCE_COUNCIL_ROTATED_ACTIVITY_TAG,
      MINTING_GATEWAY_ACTIVITY_HASH_VERSION,
      context.chainId,
      context.gatewayAddress,
      args.councilHash,
      args.approvalHash,
      args.relayerArgonAccountId,
      hashMintingGatewayActivityState(args.gatewayState),
    ]),
  );
}

export function hashMintingGatewayMintingAuthorityActivatedActivity(
  context: MintingGatewayHashContext,
  args: MintingAuthorityActivatedActivityHashArgs,
): Hex {
  return keccak256(
    encodeAbiParameters(MINTING_AUTHORITY_ACTIVATED_ACTIVITY_PARAMETERS, [
      MINTING_AUTHORITY_ACTIVATED_ACTIVITY_TAG,
      MINTING_GATEWAY_ACTIVITY_HASH_VERSION,
      context.chainId,
      context.gatewayAddress,
      args.signingKey,
      args.microgonCollateral,
      args.micronotCollateral,
      Number(args.coactivationCount),
      Number(args.sharedSignatureCount),
      args.approvalHash,
      args.relayerArgonAccountId,
      hashMintingGatewayActivityState(args.gatewayState),
    ]),
  );
}

export function hashMintingGatewayMintingAuthorityDeactivatedActivity(
  context: MintingGatewayHashContext,
  args: MintingAuthorityDeactivatedActivityHashArgs,
): Hex {
  return keccak256(
    encodeAbiParameters(MINTING_AUTHORITY_DEACTIVATED_ACTIVITY_PARAMETERS, [
      MINTING_AUTHORITY_DEACTIVATED_ACTIVITY_TAG,
      MINTING_GATEWAY_ACTIVITY_HASH_VERSION,
      context.chainId,
      context.gatewayAddress,
      args.signingKey,
      args.microgonCollateral,
      args.micronotCollateral,
      args.approvalHash,
      args.relayerArgonAccountId,
      hashMintingGatewayActivityState(args.gatewayState),
    ]),
  );
}

export function hashMintingGatewayTransferToArgonStartedActivity(
  context: MintingGatewayHashContext,
  args: TransferToArgonStartedActivityHashArgs,
): Hex {
  return keccak256(
    encodeAbiParameters(TRANSFER_TO_ARGON_STARTED_ACTIVITY_PARAMETERS, [
      TRANSFER_TO_ARGON_STARTED_ACTIVITY_TAG,
      MINTING_GATEWAY_ACTIVITY_HASH_VERSION,
      context.chainId,
      context.gatewayAddress,
      args.from,
      args.token,
      args.amount,
      args.argonAccountId,
      hashMintingGatewayActivityState(args.gatewayState),
    ]),
  );
}

export function hashMintingGatewayTransferOutOfArgonCanceledActivity(
  context: MintingGatewayHashContext,
  args: TransferOutOfArgonCanceledActivityHashArgs,
): Hex {
  return keccak256(
    encodeAbiParameters(TRANSFER_OUT_OF_ARGON_CANCELED_ACTIVITY_PARAMETERS, [
      TRANSFER_OUT_OF_ARGON_CANCELED_ACTIVITY_TAG,
      MINTING_GATEWAY_ACTIVITY_HASH_VERSION,
      context.chainId,
      context.gatewayAddress,
      args.transferId,
      hashMintingGatewayActivityState(args.gatewayState),
    ]),
  );
}

export function hashMintingGatewayTransferOutOfArgonFinalizedActivity(
  context: MintingGatewayHashContext,
  args: TransferOutOfArgonFinalizedActivityHashArgs,
): Hex {
  return keccak256(
    encodeAbiParameters(TRANSFER_OUT_OF_ARGON_FINALIZED_ACTIVITY_PARAMETERS, [
      TRANSFER_OUT_OF_ARGON_FINALIZED_ACTIVITY_TAG,
      MINTING_GATEWAY_ACTIVITY_HASH_VERSION,
      context.chainId,
      context.gatewayAddress,
      args.transferId,
      args.token,
      args.amount,
      keccak256(
        encodeAbiParameters(
          [{ type: 'tuple[]', components: MINTING_AUTHORITY_COLLATERAL_PARAMETERS }],
          [args.mintingCollateral],
        ),
      ),
      hashMintingGatewayActivityState(args.gatewayState),
    ]),
  );
}
