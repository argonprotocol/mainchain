import {
  EvmContracts,
  type IArgonQueryable,
  type PalletCrosschainTransferCouncilApprovalQueueEntry,
} from '@argonprotocol/mainchain';
import type { ContractFunctionArgs } from 'viem';
import { getAddress, keccak256, type Address, type Hex } from 'viem';
import { toEvmRecoverableSignature } from './EthereumE2eUtils';

const {
  encodeMintingGatewayMintingAuthorityActivationTarget,
  encodeMintingGatewayMintingAuthorityDeactivateTarget,
  hashMintingGatewayActivateMintingAuthority,
  hashMintingGatewayGatewayUpdateApproval,
  hashMintingGatewayMintingAuthorityDeactivation,
  mintingGatewayAbi,
  MINTING_GATEWAY_UPDATE_KINDS,
} = EvmContracts;

type MintingGatewayHashContext = Parameters<
  typeof EvmContracts.hashMintingGatewayGatewayUpdateApproval
>[0];
type MintingGatewayMintingAuthorityActivationTarget = Parameters<
  typeof EvmContracts.encodeMintingGatewayMintingAuthorityActivationTarget
>[0];
type ApplyGatewayUpdatesArgs = ContractFunctionArgs<
  typeof EvmContracts.mintingGatewayAbi,
  'nonpayable',
  'applyGatewayUpdates'
>;
type MintingGatewayCouncilSnapshot = ApplyGatewayUpdatesArgs[0];
type MintingGatewayGatewayUpdate = ApplyGatewayUpdatesArgs[1][number];

export type EthereumGatewayUpdateBatch = {
  destinationChain: 'Ethereum';
  chainId: bigint;
  gatewayAddress: Address;
  currentCouncilHash: Hex;
  currentCouncil: MintingGatewayCouncilSnapshot;
  argonApprovalsNonce: bigint;
  argonApprovalsHash: Hex;
  paused: boolean;
  pendingClearOutQueueNonces: bigint[];
  firstQueueNonce?: bigint;
  lastQueueNonce?: bigint;
  updates: MintingGatewayGatewayUpdate[];
};

type LoadedCouncil = {
  totalWeight: bigint;
  members: {
    signer: Address;
    weight: bigint;
  }[];
};

type ApprovalQueueEntry = PalletCrosschainTransferCouncilApprovalQueueEntry;

export async function getReadyEthereumGatewayUpdates(
  client: IArgonQueryable,
  gatewayClient: {
    readContract: (...args: any[]) => Promise<unknown>;
  },
  options: {
    destinationChain?: 'Ethereum';
    maxQueueEntries?: number;
  } = {},
): Promise<EthereumGatewayUpdateBatch> {
  const destinationChain = options.destinationChain ?? 'Ethereum';
  const maxQueueEntries = options.maxQueueEntries ?? 100;
  if (maxQueueEntries < 1) {
    throw new Error(`maxQueueEntries must be at least 1, received ${maxQueueEntries}`);
  }

  const chainConfigOption =
    await client.query.crosschainTransfer.chainConfigBySourceChain(destinationChain);
  if (chainConfigOption.isNone) {
    throw new Error(`Crosschain config not found for ${destinationChain}`);
  }

  const chainConfig = chainConfigOption.unwrap();
  if (!chainConfig.isEvm) {
    throw new Error(`Chain config for ${destinationChain} is not EVM-shaped`);
  }

  const gatewayAddress = getAddress(toHexValue(chainConfig.asEvm.gateway));
  const chainId = chainConfig.asEvm.chainId.toBigInt();
  const hashContext: MintingGatewayHashContext = { chainId, gatewayAddress };

  const currentCouncilHashOption =
    await client.query.crosschainTransfer.activeGlobalIssuanceCouncilByDestinationChain(
      destinationChain,
    );
  if (currentCouncilHashOption.isNone) {
    throw new Error(`Active GlobalIssuanceCouncil not found for ${destinationChain}`);
  }

  const currentCouncilHash = toHexValue(currentCouncilHashOption.unwrap());
  const councilCache = new Map<Hex, LoadedCouncil>();
  const currentCouncil = councilToSnapshot(
    await loadCouncilByHash(client, currentCouncilHash, councilCache),
  );

  const [rawArgonApprovalsNonce, rawArgonApprovalsHash, rawPaused] = await Promise.all([
    gatewayClient.readContract({
      abi: mintingGatewayAbi,
      address: gatewayAddress,
      functionName: 'argonApprovalsNonce',
    }),
    gatewayClient.readContract({
      abi: mintingGatewayAbi,
      address: gatewayAddress,
      functionName: 'argonApprovalsHash',
    }),
    gatewayClient.readContract({
      abi: mintingGatewayAbi,
      address: gatewayAddress,
      functionName: 'paused',
    }),
  ]);
  const argonApprovalsNonce = rawArgonApprovalsNonce as bigint;
  const argonApprovalsHash = rawArgonApprovalsHash as Hex;
  const paused = rawPaused as boolean;

  const pendingClearOutQueueNonces: bigint[] = [];
  const candidateUpdates: MintingGatewayGatewayUpdate[] = [];
  let expectedPreviousApprovalHash = argonApprovalsHash;
  let readyQueueEntriesScanned = 0;

  if (!paused) {
    for (
      let queueNonce = argonApprovalsNonce + 1n;
      readyQueueEntriesScanned < maxQueueEntries;
      queueNonce += 1n
    ) {
      const entryOption =
        await client.query.crosschainTransfer.councilApprovalQueueByDestinationChainAndNonce(
          destinationChain,
          queueNonce,
        );
      if (entryOption.isNone) {
        break;
      }

      const entry = entryOption.unwrap();
      const approvingCouncilHash = toHexValue(entry.approvingCouncilHash);
      if (
        !(await queueEntryIsReady(
          client,
          entry,
          approvingCouncilHash,
          councilCache,
          hashContext,
          queueNonce,
        ))
      ) {
        break;
      }
      readyQueueEntriesScanned += 1;

      if (toHexValue(entry.previousApprovalHash) !== expectedPreviousApprovalHash) {
        throw new Error(
          `Queue nonce ${queueNonce} expected previous approval hash ${expectedPreviousApprovalHash}, received ${toHexValue(entry.previousApprovalHash)}`,
        );
      }

      const update = await buildGatewayUpdate(client, destinationChain, hashContext, queueNonce, {
        entry,
        approvingCouncilHash,
      });
      candidateUpdates.push(update);
      expectedPreviousApprovalHash = toHexValue(entry.approvalHash);
    }
  }

  while (
    candidateUpdates.length > 0 &&
    candidateUpdates[candidateUpdates.length - 1]?.kind ===
      MINTING_GATEWAY_UPDATE_KINDS.mintingAuthorityDeactivate
  ) {
    pendingClearOutQueueNonces.unshift(candidateUpdates.pop()!.queueNonce);
  }

  const updates = candidateUpdates;
  const firstQueueNonce = updates[0]?.queueNonce;
  const lastQueueNonce = updates[updates.length - 1]?.queueNonce;

  return {
    destinationChain,
    chainId,
    gatewayAddress,
    currentCouncilHash,
    currentCouncil,
    argonApprovalsNonce,
    argonApprovalsHash,
    paused,
    pendingClearOutQueueNonces,
    ...(firstQueueNonce !== undefined ? { firstQueueNonce, lastQueueNonce } : {}),
    updates,
  };
}

async function buildGatewayUpdate(
  client: IArgonQueryable,
  destinationChain: 'Ethereum',
  hashContext: MintingGatewayHashContext,
  queueNonce: bigint,
  queueItem: {
    entry: ApprovalQueueEntry;
    approvingCouncilHash: Hex;
  },
): Promise<MintingGatewayGatewayUpdate> {
  const { entry, approvingCouncilHash } = queueItem;
  if (entry.target.isMintingAuthorityActivation) {
    const signatures = getSortedSignatures(entry.signatures);
    const signingKey = getAddress(toHexValue(entry.target.asMintingAuthorityActivation));
    const authorityOption =
      await client.query.crosschainTransfer.mintingAuthoritiesBySigner(signingKey);
    if (authorityOption.isNone) {
      throw new Error(
        `Minting authority activation ${signingKey} not found for queue nonce ${queueNonce}`,
      );
    }

    const authority = authorityOption.unwrap();
    if (authority.destinationChain.type !== destinationChain) {
      throw new Error(
        `Minting authority ${signingKey} belongs to ${String(authority.destinationChain.type)}, expected ${String(destinationChain)}`,
      );
    }

    const target: MintingGatewayMintingAuthorityActivationTarget = {
      microgonCollateral: authority.gatewayRemainingMicrogonCollateral.toBigInt(),
      micronotCollateral: authority.gatewayRemainingMicronotCollateral.toBigInt(),
      signingKey,
    };
    const payload = encodeMintingGatewayMintingAuthorityActivationTarget(target);
    const targetPayloadHash = payloadHashFromActivationPayload(hashContext, target);
    const approvalHash = hashMintingGatewayGatewayUpdateApproval(hashContext, {
      queueNonce,
      approvingCouncilHash,
      kind: MINTING_GATEWAY_UPDATE_KINDS.mintingAuthorityActivate,
      targetId: `0x${signingKey.slice(2).padStart(64, '0').toLowerCase()}`,
      targetPayloadHash,
      previousUpdateHash: toHexValue(entry.previousApprovalHash),
    });

    if (toHexValue(entry.targetPayloadHash) !== targetPayloadHash) {
      throw new Error(`Queue nonce ${queueNonce} target payload hash does not match authority`);
    }
    if (toHexValue(entry.approvalHash) !== approvalHash) {
      throw new Error(
        `Queue nonce ${queueNonce} approval hash does not match authority: actual=${toHexValue(entry.approvalHash)} expected=${approvalHash} previous=${toHexValue(entry.previousApprovalHash)} council=${approvingCouncilHash} targetPayload=${toHexValue(entry.targetPayloadHash)}`,
      );
    }

    return {
      queueNonce,
      kind: MINTING_GATEWAY_UPDATE_KINDS.mintingAuthorityActivate,
      payload,
      signatures,
    };
  }

  if (entry.target.isMintingAuthorityDeactivation) {
    const { payload, signatures } = validateDeactivationEntry(
      hashContext,
      queueNonce,
      entry,
      approvingCouncilHash,
    );

    return {
      queueNonce,
      kind: MINTING_GATEWAY_UPDATE_KINDS.mintingAuthorityDeactivate,
      payload,
      signatures,
    };
  }

  throw new Error(`Unsupported approval queue target ${entry.target.type}`);
}

async function loadCouncilByHash(
  client: IArgonQueryable,
  councilHash: Hex,
  cache: Map<Hex, LoadedCouncil>,
): Promise<LoadedCouncil> {
  const cached = cache.get(councilHash);
  if (cached) {
    return cached;
  }

  const councilOption =
    await client.query.crosschainTransfer.globalIssuanceCouncilByHash(councilHash);
  if (councilOption.isNone) {
    throw new Error(`GlobalIssuanceCouncil ${councilHash} not found`);
  }

  const council = councilOption.unwrap();
  const loaded = {
    totalWeight: council.totalWeight.toBigInt(),
    members: [...council.members.entries()]
      .map(([signer, member]) => ({
        signer: getAddress(toHexValue(signer)),
        weight: member.weight.toBigInt(),
      }))
      .sort((left, right) => left.signer.localeCompare(right.signer)),
  };

  cache.set(councilHash, loaded);
  return loaded;
}

function queueEntryHasQuorum(entry: ApprovalQueueEntry, council: LoadedCouncil): boolean {
  const signedWeight = [...entry.signatures.entries()].reduce((total, [signer]) => {
    const signerAddress = getAddress(toHexValue(signer));
    const member = council.members.find(x => x.signer === signerAddress);
    if (!member) {
      throw new Error(`Signature submitted by ${signerAddress}, which is not in the council`);
    }

    return total + member.weight;
  }, 0n);

  return signedWeight * 2n > council.totalWeight;
}

async function queueEntryIsReady(
  client: IArgonQueryable,
  entry: ApprovalQueueEntry,
  approvingCouncilHash: Hex,
  councilCache: Map<Hex, LoadedCouncil>,
  hashContext: MintingGatewayHashContext,
  queueNonce: bigint,
): Promise<boolean> {
  if (entry.target.isMintingAuthorityDeactivation) {
    validateDeactivationEntry(hashContext, queueNonce, entry, approvingCouncilHash);
    return true;
  }

  const approvingCouncil = await loadCouncilByHash(client, approvingCouncilHash, councilCache);
  return queueEntryHasQuorum(entry, approvingCouncil);
}

function councilToSnapshot(council: LoadedCouncil): MintingGatewayCouncilSnapshot {
  return {
    signers: council.members.map(member => member.signer),
    weights: council.members.map(member => member.weight),
  };
}

function payloadHashFromActivationPayload(
  hashContext: MintingGatewayHashContext,
  target: MintingGatewayMintingAuthorityActivationTarget,
): Hex {
  return hashMintingGatewayActivateMintingAuthority(hashContext, target);
}

function payloadHashFromDeactivationPayload(target: { signingKey: Address }): Hex {
  return keccak256(encodeMintingGatewayMintingAuthorityDeactivateTarget(target));
}

function validateDeactivationEntry(
  hashContext: MintingGatewayHashContext,
  queueNonce: bigint,
  entry: ApprovalQueueEntry,
  approvingCouncilHash: Hex,
): {
  payload: ReturnType<typeof encodeMintingGatewayMintingAuthorityDeactivateTarget>;
  signatures: MintingGatewayGatewayUpdate['signatures'];
} {
  const signingKey = getAddress(toHexValue(entry.target.asMintingAuthorityDeactivation));
  const target = { signingKey };
  const payload = encodeMintingGatewayMintingAuthorityDeactivateTarget(target);
  const targetPayloadHash = payloadHashFromDeactivationPayload(target);
  const approvalHash = hashMintingGatewayMintingAuthorityDeactivation(hashContext, {
    queueNonce,
    target,
    previousUpdateHash: toHexValue(entry.previousApprovalHash),
  });

  if (toHexValue(entry.targetPayloadHash) !== targetPayloadHash) {
    throw new Error(`Queue nonce ${queueNonce} target payload hash does not match deactivation`);
  }
  if (toHexValue(entry.approvalHash) !== approvalHash) {
    throw new Error(
      `Queue nonce ${queueNonce} approval hash does not match deactivation: actual=${toHexValue(entry.approvalHash)} expected=${approvalHash} previous=${toHexValue(entry.previousApprovalHash)} council=${approvingCouncilHash}`,
    );
  }

  const deactivationSignatures = [...entry.signatures.entries()];
  if (deactivationSignatures.length !== 1) {
    throw new Error(
      `Queue nonce ${queueNonce} expected exactly one deactivation signature, received ${deactivationSignatures.length}`,
    );
  }

  const [signer] = deactivationSignatures[0];
  if (getAddress(toHexValue(signer)) !== signingKey) {
    throw new Error(
      `Queue nonce ${queueNonce} deactivation signature was submitted by ${getAddress(toHexValue(signer))}, expected ${signingKey}`,
    );
  }

  return {
    payload,
    signatures: getSortedSignatures(entry.signatures),
  };
}

function getSortedSignatures(
  signatures: ApprovalQueueEntry['signatures'],
): MintingGatewayGatewayUpdate['signatures'] {
  return [...signatures.entries()]
    .sort(([leftSigner], [rightSigner]) =>
      toHexValue(leftSigner).localeCompare(toHexValue(rightSigner)),
    )
    .map(([, signature]) => toEvmRecoverableSignature(toHexValue(signature)));
}

function toHexValue(value: { toHex(): string }): Hex {
  return value.toHex() as Hex;
}
