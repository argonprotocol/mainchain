import { expect, it } from 'vitest';
import { decodeEventLog, encodeFunctionData, type Hex } from 'viem';
import purchaseReceipt from '../../../../chains/ethereum/fixtures/uniswap-v4-purchase.json';
import * as mainchain from '../index';

it('exports typed EVM contracts through the EvmContracts namespace', () => {
  expect(mainchain.EvmContracts.argonTokenArtifact.abi).toBeTruthy();
  expect(mainchain.EvmContracts.mintingGatewayAbi).toBeTruthy();
  expect(mainchain.EvmContracts.MINTING_GATEWAY_RUNTIME_TO_ERC20_SCALE).toBeGreaterThan(0n);
  expect('argonTokenArtifact' in mainchain).toBe(false);
});

it('preserves V0 commitments and binds the V1 format including zero inflation', () => {
  const council = {
    version: 0,
    signers: [`0x${'44'.repeat(20)}`, `0x${'55'.repeat(20)}`],
    weights: [60n, 40n],
  } as const;
  const context = { chainId: 1n, gatewayAddress: `0x${'11'.repeat(20)}` } as const;
  const withoutInflation = { council, epochMicrogonsPerArgonot: 6_000_000n } as const;
  const withInflation = {
    ...withoutInflation,
    council: { ...council, version: 1 },
    inflationPercentSinceLaunch: 60_000_000_000_000_000n,
  } as const;

  expect(
    mainchain.EvmContracts.hashMintingGatewayGlobalIssuanceCouncil({
      ...council,
      ...withoutInflation,
    }),
  ).toBe('0x0df99e9f55797439ddadf1526e5d5a9fa5be256c168aed2924b5f4cfdb57f5bf');
  expect(
    mainchain.EvmContracts.hashMintingGatewayRotateGlobalIssuanceCouncil(context, withoutInflation),
  ).toBe('0x8157fa1ca76b0c2824c5daa0839ba2dac30886d242551736349dd9488aea34e4');
  expect(
    mainchain.EvmContracts.hashMintingGatewayGlobalIssuanceCouncil({
      ...council,
      ...withoutInflation,
      inflationPercentSinceLaunch: 0n,
    }),
  ).toBe('0x0df99e9f55797439ddadf1526e5d5a9fa5be256c168aed2924b5f4cfdb57f5bf');
  expect(
    mainchain.EvmContracts.hashMintingGatewayRotateGlobalIssuanceCouncil(context, {
      ...withoutInflation,
      inflationPercentSinceLaunch: 0n,
    }),
  ).toBe('0x8157fa1ca76b0c2824c5daa0839ba2dac30886d242551736349dd9488aea34e4');
  expect(
    mainchain.EvmContracts.hashMintingGatewayGlobalIssuanceCouncil({
      ...council,
      version: 1,
      epochMicrogonsPerArgonot: 6_000_000n,
      inflationPercentSinceLaunch: 0n,
    }),
  ).toBe('0x536d8ac38df8d21c6c2a21a13927ed23f9a09333a728ca443fefbc07ca394747');
  expect(
    mainchain.EvmContracts.hashMintingGatewayGlobalIssuanceCouncil({
      ...withInflation.council,
      ...withInflation,
    }),
  ).toBe('0x498be932c4933742e9dc549c9d1c13da17b97204dd97862583187cdd1ebc29e8');
  expect(
    mainchain.EvmContracts.hashMintingGatewayRotateGlobalIssuanceCouncil(context, withInflation),
  ).toBe('0x666759c0be11ebb37d6d44fc761727cb3b0076e14fc2fc8319cbce0e3f3db528');
  expect(
    mainchain.EvmContracts.hashMintingGatewayGlobalIssuanceCouncil({
      ...withInflation.council,
      ...withInflation,
      inflationPercentSinceLaunch: withInflation.inflationPercentSinceLaunch + 1n,
    }),
  ).not.toBe(
    mainchain.EvmContracts.hashMintingGatewayGlobalIssuanceCouncil({
      ...withInflation.council,
      ...withInflation,
    }),
  );
});

it('pins transfer-out hashes and finalize calldata to a known vector', () => {
  const request = {
    argonAccountId: `0x${'22'.repeat(32)}`,
    argonTransferNonce: 1n,
    chainId: 1n,
    microgonsPerArgonot: 7n,
    recipient: `0x${'55'.repeat(20)}`,
    validUntilBlock: 10n,
    token: `0x${'44'.repeat(20)}`,
    amount: 25n,
    mintingAuthorityTip: 1n,
  } as const;
  const gateway = {
    chainId: 1n,
    gatewayAddress: `0x${'11'.repeat(20)}`,
  } as const;
  const authorization = {
    request,
    microgonCollateral: 16n,
    micronotCollateral: 0n,
  } as const;
  const finalizeProof = {
    authorizations: [
      {
        microgonCollateral: 16n,
        micronotCollateral: 0n,
        signature: `0x${'33'.repeat(65)}`,
      },
    ],
  } as const;

  expect(mainchain.EvmContracts.hashMintingGatewayTransferOutOfArgonRequest(request)).toBe(
    '0x15547a58403a407ace23aaafbc0343f0221b1564405e9974f692e5f25b9b08ce',
  );
  expect(
    mainchain.EvmContracts.hashMintingGatewayMintingAuthorization(gateway, authorization),
  ).toBe('0x94fb1ef1202e2f7b0e2943219453e28153d2a97069bdb48ce28a695a6cf3bbb8');
  expect(
    encodeFunctionData({
      abi: mainchain.EvmContracts.mintingGatewayAbi,
      functionName: 'finalizeTransferOutOfArgon',
      args: [request, finalizeProof],
    }),
  ).toBe(
    '0x138f878122222222222222222222222222222222222222222222222222222222222222220000000000000000000000000000000000000000000000000000000000000001000000000000000000000000000000000000000000000000000000000000000100000000000000000000000000000000000000000000000000000000000000070000000000000000000000005555555555555555555555555555555555555555000000000000000000000000000000000000000000000000000000000000000a00000000000000000000000044444444444444444444444444444444444444440000000000000000000000000000000000000000000000000000000000000019000000000000000000000000000000000000000000000000000000000000000100000000000000000000000000000000000000000000000000000000000001400000000000000000000000000000000000000000000000000000000000000020000000000000000000000000000000000000000000000000000000000000000100000000000000000000000000000000000000000000000000000000000000200000000000000000000000000000000000000000000000000000000000000010000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000600000000000000000000000000000000000000000000000000000000000000041333333333333333333333333333333333333333333333333333333333333333333333333333333333333333333333333333333333333333333333333333333333300000000000000000000000000000000000000000000000000000000000000',
  );
});

it('changes the transfer request hash when microgonsPerArgonot changes', () => {
  const request = {
    argonAccountId: `0x${'22'.repeat(32)}`,
    argonTransferNonce: 1n,
    chainId: 1n,
    microgonsPerArgonot: 7n,
    recipient: `0x${'55'.repeat(20)}`,
    validUntilBlock: 10n,
    token: `0x${'44'.repeat(20)}`,
    amount: 25n,
    mintingAuthorityTip: 1n,
  } as const;

  expect(
    mainchain.EvmContracts.hashMintingGatewayTransferOutOfArgonRequest({
      ...request,
      microgonsPerArgonot: request.microgonsPerArgonot + 1n,
    }),
  ).not.toBe(mainchain.EvmContracts.hashMintingGatewayTransferOutOfArgonRequest(request));
});

it('binds every purchase field and gateway domain to the native activity commitment', () => {
  const context = {
    chainId: BigInt(purchaseReceipt.chainId),
    gatewayAddress: purchaseReceipt.eventLog.address as Hex,
  };
  const { args: event } = decodeEventLog({
    abi: mainchain.EvmContracts.mintingGatewayAbi,
    eventName: 'TransferToArgonFromSwapStarted',
    topics: purchaseReceipt.eventLog.topics as [Hex, ...Hex[]],
    data: purchaseReceipt.eventLog.data as Hex,
  });
  const hash = mainchain.EvmContracts.hashMintingGatewayTransferToArgonFromSwapStartedActivity;
  const expected = hash(context, event);
  expect(
    mainchain.EvmContracts.appendMintingGatewayActivityRoot(
      purchaseReceipt.activityRootSeed as Hex,
      expected,
    ),
  ).toBe(purchaseReceipt.activityRoot);
  expect(hash(context, event)).toBe(expected);

  for (const [key, value] of Object.entries(event.purchase)) {
    let changed: string | bigint;
    if (typeof value === 'bigint') {
      changed = value + 1n;
    } else if (value === '0x') {
      changed = '0x01';
    } else {
      changed = `${value.slice(0, -1).toLowerCase()}${value.endsWith('0') ? '1' : '0'}`;
    }

    expect(hash(context, { ...event, purchase: { ...event.purchase, [key]: changed } })).not.toBe(
      expected,
    );
  }
  for (const [key, value] of Object.entries(event.gatewayState)) {
    expect(
      hash(context, { ...event, gatewayState: { ...event.gatewayState, [key]: value + 1n } }),
    ).not.toBe(expected);
  }
  expect(hash({ ...context, chainId: 1n }, event)).not.toBe(expected);
  expect(hash({ ...context, gatewayAddress: event.token }, event)).not.toBe(expected);
  expect(hash(context, { ...event, amount: 251n })).not.toBe(expected);
  expect(hash(context, { ...event, token: event.purchase.usdcToken })).not.toBe(expected);
});
