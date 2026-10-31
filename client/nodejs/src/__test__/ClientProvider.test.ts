import { expect, expectTypeOf, it } from 'vitest';
import { once } from 'node:events';
import { WebSocketServer } from 'ws';
import { WsProvider } from '@polkadot/rpc-provider';
import { Metadata, TypeRegistry } from '@polkadot/types';
import type { HexString } from '@polkadot/util/types';
import { getClient, type ArgonClient } from '../index';
import type { ApiPromise } from '@polkadot/api';
import Genesis from '../../genesis.json';
import metadata from '../../metadata.json';
import runtimeVersion from '../../runtime_version.json';

it('uses one supplied connection and bundled metadata for historical reads', async () => {
  expectTypeOf<ApiPromise>().toEqualTypeOf<ArgonClient>();
  expectTypeOf<Awaited<ReturnType<typeof getClient>>>().toEqualTypeOf<ApiPromise>();

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
    expect(client.consts.miningSlot.bidRevealBlocks.toNumber()).toBe(5);

    const keys = new Uint8Array(64);
    const bid = client.tx.miningSlot.bid;
    const explicit = bid(10_000n, keys, null, null, null);
    expect(explicit.method.args[3].toJSON()).toBeNull();
    expect(explicit.method.args[4].toJSON()).toBeNull();
    // @ts-expect-error Optional runtime fields still require explicit argument positions.
    expect(() => bid(10_000n, keys, null)).toThrow('expects 5 arguments');

    const frameOnly = bid(10_000n, keys, null, 2n, null);
    expect(frameOnly.method.args[3].toJSON()).toBe(2);
    expect(frameOnly.method.args[4].toJSON()).toBeNull();
    const hash = '0x' + '11'.repeat(32);
    const commitment = bid(10_000n, keys, null, 2n, hash);
    expect(commitment.method.args[3].toJSON()).toBe(2);
    expect(commitment.method.args[4].toHex()).toBe(hash);
    const bidMax = client.tx.miningSlot.bidMax;
    const reveal = bidMax(20_000n, hash, null, 2n);
    expect(reveal.method.args[3].toU8a()).toEqual(new Uint8Array([2, 0, 0, 0, 0, 0, 0, 0]));
    expect(bidMax.meta.fields[3].name.unwrap().toString()).toBe('target_frame_id');
    expect(client.tx.utility.batchAll([reveal]).method.args[0].toJSON()).toEqual([
      reveal.method.toJSON(),
    ]);
    // @ts-expect-error A maximum reveal must explicitly target its original auction frame.
    expect(() => bidMax(20_000n, hash, null)).toThrow('expects 4 arguments');
    // @ts-expect-error The SDK must not silently ignore extra arguments.
    expect(() => bid(10_000n, keys, null, null, null, null)).toThrow('expects 5 arguments');

    // Native argument requirements remain intact after a runtime metadata refresh.
    client.injectMetadata(new Metadata(registry, metadata.result as HexString), true, registry);
    const refreshedBid = client.tx.miningSlot.bid;
    expect(refreshedBid).not.toBe(bid);
    expect(refreshedBid(10_000n, keys, null, null, null).method.toHex()).toBe(
      explicit.method.toHex(),
    );
    // @ts-expect-error Metadata refresh must not install argument defaults.
    expect(() => refreshedBid(10_000n, keys, null)).toThrow('expects 5 arguments');
    expect(connections).toBe(1);
    expect(metadataRequests).toBe(0);
  } finally {
    await (client ?? provider).disconnect();
    for (const socket of server.clients) socket.terminate();
    await new Promise<void>(resolve => server.close(() => resolve()));
  }
});
