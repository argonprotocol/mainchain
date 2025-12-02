import { ArgonPrimitivesDigestsFrameInfo, getOfflineRegistry, Header } from './index';

export function getTickFromHeader(header: Header): number | undefined {
  for (const x of header.digest.logs) {
    if (x.isPreRuntime) {
      const [engineId, data] = x.asPreRuntime;
      if (engineId.toString() === 'aura') {
        return getOfflineRegistry().createType('u64', data).toNumber();
      }
    }
  }
  return undefined;
}

export function getAuthorFromHeader(header: Header): string | undefined {
  for (const x of header.digest.logs) {
    if (x.isPreRuntime) {
      const [engineId, data] = x.asPreRuntime;
      if (engineId.toString() === 'pow_') {
        return getOfflineRegistry().createType('AccountId32', data).toHuman();
      }
    }
  }
  return undefined;
}

export function getFrameInfoFromHeader(
  header: Header,
): { isNewFrame: boolean; frameId: number; frameRewardTicksRemaining: number } | undefined {
  for (const x of header.digest.logs) {
    if (x.isConsensus) {
      const [engineId, data] = x.asConsensus;
      if (engineId.toString() === 'fram') {
        const decoded: ArgonPrimitivesDigestsFrameInfo = getOfflineRegistry().createType(
          'ArgonPrimitivesDigestsFrameInfo',
          data,
        );
        return {
          isNewFrame: decoded.isNewFrame.toPrimitive(),
          frameId: decoded.frameId.toNumber(),
          frameRewardTicksRemaining: decoded.frameRewardTicksRemaining.toNumber(),
        };
      }
    }
  }
  return undefined;
}
