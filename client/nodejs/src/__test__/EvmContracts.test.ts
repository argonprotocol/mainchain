import { expect, it } from 'vitest';
import { encodeFunctionData } from 'viem';
import * as mainchain from '../index';

it('exports typed EVM contracts through the EvmContracts namespace', () => {
  expect(mainchain.EvmContracts.argonTokenArtifact.abi).toBeTruthy();
  expect(mainchain.EvmContracts.mintingGatewayAbi).toBeTruthy();
  expect(mainchain.EvmContracts.MINTING_GATEWAY_RUNTIME_TO_ERC20_SCALE).toBeGreaterThan(0n);
  expect('argonTokenArtifact' in mainchain).toBe(false);
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
  expect(mainchain.EvmContracts.hashMintingGatewayMintingAuthorization(gateway, authorization)).toBe(
    '0x94fb1ef1202e2f7b0e2943219453e28153d2a97069bdb48ce28a695a6cf3bbb8',
  );
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
