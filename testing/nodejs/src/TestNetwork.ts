import * as docker from 'docker-compose';
import { runOnTeardown } from './index';

export async function startNetwork(options?: {
  shouldLog: boolean;
  dockerEnv?: Record<string, string>;
}): Promise<{ archiveUrl: string; notaryUrl: string }> {
  const env = {
    VERSION: 'dev',
    ARGON_CHAIN: 'dev-docker',
    BITCOIN_BLOCK_SECS: '20',
    PATH: `${process.env.PATH}:/opt/homebrew/bin:/usr/local/bin`,
    ...(options?.dockerEnv ?? {}),
  };
  await docker.upAll({
    log: options?.shouldLog ?? false,
    commandOptions: [`--force-recreate`, `--remove-orphans`],
    env,
  });
  const portResult = await docker.port('archive-node', '9944');
  const notaryPortResult = await docker.port('notary', '9925');
  const port = portResult.data.port;
  runOnTeardown(async () => {
    await docker.downAll({
      log: options?.shouldLog ?? false,
      commandOptions: [`--volumes`],
    });
  });
  return {
    archiveUrl: `ws://localhost:${port}`,
    notaryUrl: `ws://localhost:${notaryPortResult.data.port}`,
  };
}
