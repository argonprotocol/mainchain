import { expect, it } from 'vitest';
import { once } from 'node:events';
import { WebSocketServer } from 'ws';
import { WsProvider } from '@polkadot/rpc-provider';
import { Metadata, TypeRegistry } from '@polkadot/types';
import type { HexString } from '@polkadot/util/types';
import { getClient } from '../index';
import { withMiningBidDefaults } from '../miningBid';
import Genesis from '../../genesis.json';
import metadata from '../../metadata.json';
import runtimeVersion from '../../runtime_version.json';

it.each(Object.entries(Genesis))('uses %s cached metadata', async (_network, genesisHash) => {
  const registry = new TypeRegistry();
  const header = registry.createType('Header', { number: 9, parentHash: genesisHash });
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
          result = Number(params[0]) === 0 ? genesisHash : header.hash.toHex();
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

    // Registration must be available through the known-network metadata cache used by deployment.
    const poolManager = `0x${'33'.repeat(20)}`;
    const registration = client.tx.stabilizationPools.registerPool(poolManager, {
      positionManager: `0x${'22'.repeat(20)}`,
      subscriber: `0x${'11'.repeat(20)}`,
      key: {
        currency0: `0x${'44'.repeat(20)}`,
        currency1: `0x${'55'.repeat(20)}`,
        fee: 0x800000,
        tickSpacing: 10,
        hooks: `0x${'66'.repeat(20)}`,
      },
    });

    // Runtime pallet 39, call 0, followed by PoolManager and the SCALE-encoded PoolConfig.
    const registrationCallHex = [
      '0x2700',
      '33'.repeat(20),
      '22'.repeat(20),
      '11'.repeat(20),
      '44'.repeat(20),
      '55'.repeat(20),
      '00008000', // uint32 dynamic-fee flag
      '0a000000', // int32 tick spacing
      '66'.repeat(20),
    ].join('');
    expect(registration.method.toHex()).toBe(registrationCallHex);
    expect(client.tx.sudo.sudo(registration).method.args[0].toHex()).toBe(registrationCallHex);

    const keys = new Uint8Array(64);
    const bid = client.tx.miningSlot.bid;
    const explicit = bid(10_000n, keys, null, null, null);
    const short = bid(10_000n, keys, null);
    expect(short.method.toHex()).toBe(explicit.method.toHex());
    expect(bid(10_000n, keys, null, undefined, undefined).method.toHex()).toBe(
      explicit.method.toHex(),
    );
    expect(short.method.args[3].toJSON()).toBeNull();
    expect(short.method.args[4].toJSON()).toBeNull();
    expect(typeof short.signAndSend).toBe('function');
    expect(typeof short.paymentInfo).toBe('function');
    expect(bid.is(short.method)).toBe(true);
    expect(bid.callIndex).toEqual(short.method.callIndex);
    expect(bid.meta.fields).toHaveLength(5);
    expect(client.tx.utility.batchAll([short]).method.args[0].toJSON()).toEqual([
      short.method.toJSON(),
    ]);

    const frameOnly = bid(10_000n, keys, null, 2n);
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
    // @ts-expect-error The SDK still requires the original three arguments.
    expect(() => bid(10_000n, keys)).toThrow('expects 5 arguments');
    // @ts-expect-error The SDK must not silently ignore extra arguments.
    expect(() => bid(10_000n, keys, null, null, null, null)).toThrow('expects 5 arguments');
    expect(withMiningBidDefaults(client)).toBe(client);
    expect(client.tx.miningSlot.bid).toBe(bid);

    // Exercise the same metadata replacement and decorated event used on runtime upgrades.
    client.injectMetadata(new Metadata(registry, metadata.result as HexString), true, registry);
    expect(client.tx.miningSlot.bid).not.toBe(bid);
    expect(client.tx.miningSlot.bid(10_000n, keys, null).method.toHex()).toBe(
      explicit.method.toHex(),
    );
    expect(connections).toBe(1);
    expect(metadataRequests).toBe(0);
  } finally {
    await (client ?? provider).disconnect();
    for (const socket of server.clients) socket.terminate();
    await new Promise<void>(resolve => server.close(() => resolve()));
  }
});
