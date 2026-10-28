import {
  appendMintingGatewayActivityRoot,
  hashMintingGatewayGlobalIssuanceCouncilRotatedActivity,
  hashMintingGatewayMintingAuthorityActivatedActivity,
  hashMintingGatewayMintingAuthorityDeactivatedActivity,
  hashMintingGatewayTransferOutOfArgonCanceledActivity,
  hashMintingGatewayTransferOutOfArgonFinalizedActivity,
  hashMintingGatewayTransferToArgonStartedActivity,
  mintingGatewayAbi,
  MintingGatewayEvents,
  type MintingGatewayGlobalIssuanceCouncilRotated,
  type MintingGatewayHashContext,
  type MintingGatewayMintingAuthorityActivated,
  type MintingGatewayMintingAuthorityDeactivated,
  type MintingGatewayTransferOutOfArgonCanceled,
  type MintingGatewayTransferOutOfArgonFinalized,
  type MintingGatewayTransferToArgonStarted,
} from './EvmContracts';
import { decodeEventLog, getAddress, type Hex } from 'viem';
import type { EthereumEventLog } from './EthereumProof';

type DecodedEthereumGatewayActivity =
  | ({
      kind: typeof MintingGatewayEvents.GlobalIssuanceCouncilRotated.name;
    } & MintingGatewayGlobalIssuanceCouncilRotated)
  | ({
      kind: typeof MintingGatewayEvents.MintingAuthorityActivated.name;
    } & MintingGatewayMintingAuthorityActivated)
  | ({
      kind: typeof MintingGatewayEvents.MintingAuthorityDeactivated.name;
    } & MintingGatewayMintingAuthorityDeactivated)
  | ({
      kind: typeof MintingGatewayEvents.TransferOutOfArgonCanceled.name;
    } & MintingGatewayTransferOutOfArgonCanceled)
  | ({
      kind: typeof MintingGatewayEvents.TransferOutOfArgonFinalized.name;
    } & MintingGatewayTransferOutOfArgonFinalized)
  | ({
      kind: typeof MintingGatewayEvents.TransferToArgonStarted.name;
    } & MintingGatewayTransferToArgonStarted);

export type EthereumGatewayActivity = DecodedEthereumGatewayActivity & {
  txHash: Hex;
  transactionIndex: number;
  logIndex: number;
  blockHash: Hex;
  blockNumber: bigint;
};

const gatewayActivityEvents = [
  MintingGatewayEvents.GlobalIssuanceCouncilRotated,
  MintingGatewayEvents.MintingAuthorityActivated,
  MintingGatewayEvents.MintingAuthorityDeactivated,
  MintingGatewayEvents.TransferOutOfArgonCanceled,
  MintingGatewayEvents.TransferOutOfArgonFinalized,
  MintingGatewayEvents.TransferToArgonStarted,
] as const;

export function findEthereumTransferToArgonStartedLogIndexes(
  receipt: { transactionHash: Hex; logs: { address: Hex; topics: Hex[] }[] },
  gatewayAddress: Hex,
): number[] {
  const normalizedGatewayAddress = getAddress(gatewayAddress).toLowerCase();
  const indexes = receipt.logs.flatMap((log, index) =>
    log.address.toLowerCase() === normalizedGatewayAddress &&
    log.topics[0]?.toLowerCase() === MintingGatewayEvents.TransferToArgonStarted.topic
      ? [index]
      : [],
  );

  if (indexes.length === 0) {
    throw new Error(
      `Ethereum receipt ${receipt.transactionHash} did not emit TransferToArgonStarted from gateway ${gatewayAddress}`,
    );
  }

  return indexes;
}

export function decodeEthereumTransferToArgonStartedLog(
  log: Pick<EthereumEventLog, 'topics' | 'data'>,
): MintingGatewayTransferToArgonStarted {
  const decoded = decodeEthereumGatewayActivityLog(log);
  if (decoded.kind !== MintingGatewayEvents.TransferToArgonStarted.name) {
    throw new Error(
      `Expected ${MintingGatewayEvents.TransferToArgonStarted.name} gateway activity log`,
    );
  }

  const { kind: _kind, ...transfer } = decoded;
  return transfer;
}

export function decodeEthereumGatewayActivityLog(
  log: Pick<EthereumEventLog, 'topics' | 'data'>,
): DecodedEthereumGatewayActivity {
  const topic = log.topics[0]?.toLowerCase();
  if (!topic) {
    throw new Error('Gateway activity log is missing an event signature topic');
  }

  const event = gatewayActivityEvents.find(candidate => candidate.topic === topic);
  if (!event) {
    throw new Error(`Unsupported gateway activity event topic ${topic}`);
  }

  const { args } = decodeEventLog({
    abi: mintingGatewayAbi,
    eventName: event.name,
    topics: log.topics as [Hex, ...Hex[]],
    data: log.data,
    strict: true,
  });

  return {
    kind: event.name,
    ...args,
  } as DecodedEthereumGatewayActivity;
}

export function hashEthereumGatewayActivity(
  context: MintingGatewayHashContext,
  activity: EthereumGatewayActivity,
): Hex {
  switch (activity.kind) {
    case MintingGatewayEvents.TransferToArgonStarted.name:
      return hashMintingGatewayTransferToArgonStartedActivity(context, {
        from: activity.from,
        token: activity.token,
        amount: activity.amount,
        argonAccountId: activity.argonAccountId,
        gatewayState: activity.gatewayState,
      });
    case MintingGatewayEvents.MintingAuthorityActivated.name:
      return hashMintingGatewayMintingAuthorityActivatedActivity(context, {
        signingKey: activity.signingKey,
        microgonCollateral: activity.microgonCollateral,
        micronotCollateral: activity.micronotCollateral,
        coactivationCount: activity.coactivationCount,
        sharedSignatureCount: activity.sharedSignatureCount,
        approvalHash: activity.approvalHash,
        relayerArgonAccountId: activity.relayerArgonAccountId,
        gatewayState: activity.gatewayState,
      });
    case MintingGatewayEvents.GlobalIssuanceCouncilRotated.name:
      return hashMintingGatewayGlobalIssuanceCouncilRotatedActivity(context, {
        councilHash: activity.councilHash,
        approvalHash: activity.approvalHash,
        relayerArgonAccountId: activity.relayerArgonAccountId,
        gatewayState: activity.gatewayState,
      });
    case MintingGatewayEvents.MintingAuthorityDeactivated.name:
      return hashMintingGatewayMintingAuthorityDeactivatedActivity(context, {
        signingKey: activity.signingKey,
        microgonCollateral: activity.microgonCollateral,
        micronotCollateral: activity.micronotCollateral,
        approvalHash: activity.approvalHash,
        relayerArgonAccountId: activity.relayerArgonAccountId,
        gatewayState: activity.gatewayState,
      });
    case MintingGatewayEvents.TransferOutOfArgonFinalized.name:
      return hashMintingGatewayTransferOutOfArgonFinalizedActivity(context, {
        transferId: activity.transferId,
        token: activity.token,
        amount: activity.amount,
        mintingCollateral: activity.mintingCollateral.map(row => ({ ...row })),
        gatewayState: activity.gatewayState,
      });
    case MintingGatewayEvents.TransferOutOfArgonCanceled.name:
      return hashMintingGatewayTransferOutOfArgonCanceledActivity(context, {
        transferId: activity.transferId,
        gatewayState: activity.gatewayState,
      });
  }

  const exhaustiveCheck: never = activity;
  throw new Error(`Unhandled gateway activity: ${String(exhaustiveCheck)}`);
}

export function appendEthereumGatewayActivityRoot(currentRoot: Hex, activity: Hex): Hex {
  return appendMintingGatewayActivityRoot(currentRoot, activity);
}
