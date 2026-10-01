import { expect, it } from 'vitest';
import { once } from 'node:events';
import { WebSocketServer } from 'ws';
import { WsProvider } from '@polkadot/rpc-provider';
import { TypeRegistry } from '@polkadot/types';
import { getClient } from '../index';
import Genesis from '../../genesis.json';
import runtimeVersion from '../../runtime_version.json';

it('uses one supplied connection and bundled metadata for historical reads', async () => {
  const registry = new TypeRegistry();
  const header = registry.createType('Header', { number: 9, parentHash: Genesis.mainnet });
  const server = new WebSocketServer({ host: '127.0.0.1', port: 0 });
  await once(server, 'listening');
  const address = server.address();
  if (typeof address === 'string' || !address) throw new Error('Missing RPC address');
  const url = `ws://127.0.0.1:${address.port}`;
  let connections = 0;
  let metadataRequests = 0;
  server.on('connection', socket => {
    connections += 1;
    socket.on('message', bytes => {
      const { id, method, params } = JSON.parse((bytes as Buffer).toString());
      let result: unknown;
      switch (method) {
        case 'chain_getBlockHash':
          result = Number(params[0]) === 0 ? Genesis.mainnet : header.hash.toHex();
          break;
        case 'chain_getHeader':
          result = header.toJSON();
          break;
        case 'state_getRuntimeVersion':
          result = runtimeVersion;
          break;
        case 'system_chain':
          result = 'Argon';
          break;
        case 'system_properties':
          result = { ss58Format: 42, tokenDecimals: 6, tokenSymbol: 'ARGN' };
          break;
        case 'rpc_methods':
          result = {
            version: 1,
            methods: [
              'chain_getBlockHash',
              'chain_getHeader',
              'state_getRuntimeVersion',
              'state_getMetadata',
              'state_getStorage',
              'state_subscribeRuntimeVersion',
              'state_unsubscribeRuntimeVersion',
              'system_chain',
              'system_properties',
              'rpc_methods',
            ],
          };
          break;
        case 'state_getMetadata':
          metadataRequests += 1;
          socket.send(
            JSON.stringify({
              jsonrpc: '2.0',
              id,
              error: { code: -32000, message: 'Metadata unavailable' },
            }),
          );
          return;
        case 'state_getStorage':
          result = '0x09000000';
          break;
        case 'state_subscribeRuntimeVersion':
          result = '1';
          break;
        case 'state_unsubscribeRuntimeVersion':
          result = true;
          break;
        default:
          socket.send(
            JSON.stringify({
              jsonrpc: '2.0',
              id,
              error: { code: -32601, message: `Unexpected ${method}` },
            }),
          );
          return;
      }
      socket.send(JSON.stringify({ jsonrpc: '2.0', id, result }));
    });
  });

  const provider = new WsProvider(url);
  let client: Awaited<ReturnType<typeof getClient>> | undefined;
  try {
    client = await getClient(url, { provider, registry, throwOnConnect: true });
    const historical = await client.at(header.hash);
    expect((await historical.query.system.number()).toNumber()).toBe(9);
    expect(client.registry).toBe(registry);
    expect(connections).toBe(1);
    expect(metadataRequests).toBe(0);
  } finally {
    await (client ?? provider).disconnect();
    for (const socket of server.clients) socket.terminate();
    await new Promise<void>(resolve => server.close(() => resolve()));
  }
});
