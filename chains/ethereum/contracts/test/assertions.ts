import { expect } from 'vitest';

export async function expectCustomError(
  action: Promise<unknown>,
  contract: { interface: { parseError(data: string): { name: string; args: Iterable<unknown> } | null } },
  errorName: string,
  expectedArgs: unknown[] = [],
) {
  try {
    await action;
  } catch (error) {
    const data = getErrorData(error);
    const decodedError = contract.interface.parseError(data);

    expect(decodedError?.name).to.equal(errorName);
    expect(decodedError ? Array.from(decodedError.args) : []).to.deep.equal(expectedArgs);
    return;
  }

  throw new Error(`Expected transaction to revert with ${errorName}`);
}

export async function expectEvent(
  action: Promise<{ wait(): Promise<{ logs: Array<{ address: string; data: string; topics: string[] }> }> }>,
  contract: {
    interface: {
      parseLog(log: { address: string; data: string; topics: string[] }):
        | { name: string; args: Iterable<unknown> }
        | null;
    };
    getAddress(): Promise<string>;
  },
  eventName: string,
  expectedArgs: unknown[],
) {
  const tx = await action;
  const receipt = await tx.wait();
  const contractAddress = (await contract.getAddress()).toLowerCase();

  const parsedLogs = receipt.logs
    .filter(log => log.address.toLowerCase() === contractAddress)
    .flatMap(log => {
      const parsedLog = contract.interface.parseLog(log);
      return parsedLog === null ? [] : [parsedLog];
    })
    .filter(log => log.name === eventName);

  expect(parsedLogs.length).to.be.greaterThan(0);
  expect(Array.from(parsedLogs[0].args)).to.deep.equal(expectedArgs);
}

function getErrorData(error: unknown) {
  if (
    typeof error === 'object' &&
    error !== null &&
    'data' in error &&
    typeof error.data === 'string'
  ) {
    return error.data;
  }

  throw error;
}
