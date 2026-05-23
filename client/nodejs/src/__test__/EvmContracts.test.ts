import { expect, it } from 'vitest';
import * as mainchain from '../index';

it('exports typed EVM contracts through the EvmContracts namespace', () => {
  expect(mainchain.EvmContracts.argonTokenArtifact.abi).toBeTruthy();
  expect(mainchain.EvmContracts.mintingGatewayAbi).toBeTruthy();
  expect(mainchain.EvmContracts.MINTING_GATEWAY_RUNTIME_TO_ERC20_SCALE).toBeGreaterThan(0n);
  expect('argonTokenArtifact' in mainchain).toBe(false);
});
