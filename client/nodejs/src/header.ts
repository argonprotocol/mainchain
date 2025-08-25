import { ArgonClient, Header } from './index';

export function getTickFromHeader(client: ArgonClient, header: Header): number | undefined {
  for (const x of header.digest.logs) {
    if (x.isPreRuntime) {
      const [engineId, data] = x.asPreRuntime;
      if (engineId.toString() === 'aura') {
        return client.createType('u64', data).toNumber();
      }
    }
  }
  return undefined;
}

export function getAuthorFromHeader(client: ArgonClient, header: Header): string | undefined {
  for (const x of header.digest.logs) {
    if (x.isPreRuntime) {
      const [engineId, data] = x.asPreRuntime;
      if (engineId.toString() === 'pow_') {
        return client.createType('AccountId32', data).toHuman();
      }
    }
  }
  return undefined;
}
