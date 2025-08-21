import * as docker from 'docker-compose';
import { runOnTeardown } from './index';
import * as Path from 'node:path';

export async function startNetwork(
  testName: string,
  options?: {
    shouldLog: boolean;
    dockerEnv?: Record<string, string>;
  },
): Promise<{ archiveUrl: string; notaryUrl: string }> {
  const config = Path.join(__dirname, `docker-compose.yml`);
  const env = {
    VERSION: 'dev',
    ARGON_CHAIN: 'dev-docker',
    BITCOIN_BLOCK_SECS: '20',
    PATH: `${process.env.PATH}:/opt/homebrew/bin:/usr/local/bin`,
    COMPOSE_PROJECT_NAME: `argon-test-${testName}`,
    ...(options?.dockerEnv ?? {}),
  };
  runOnTeardown(async () => {
    await docker.downAll({
      log: options?.shouldLog ?? false,
      commandOptions: [`--volumes`],
      env,
      config,
    });
  });
  await docker.upAll({
    log: options?.shouldLog ?? false,
    commandOptions: [`--force-recreate`, `--remove-orphans`],
    config,
    env,
  });
  const portResult = await docker.port('archive-node', '9944', { config, env });
  const notaryPortResult = await docker.port('notary', '9925', { config, env });
  const port = portResult.data.port;
  return {
    archiveUrl: `ws://localhost:${port}`,
    notaryUrl: `ws://localhost:${notaryPortResult.data.port}`,
  };
}
