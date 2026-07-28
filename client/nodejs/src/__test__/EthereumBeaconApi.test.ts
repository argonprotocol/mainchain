import { afterEach, expect, it } from 'vitest';
import { getBeaconJson } from '../EthereumBeaconApi';

const originalFetch = globalThis.fetch;

afterEach(() => {
  globalThis.fetch = originalFetch;
});

it('resolves Beacon routes against a root base URL', async () => {
  globalThis.fetch = createUrlEchoFetch();

  const response = await getBeaconJson<{ url: string }>(
    'https://ethereum-beacon-api.publicnode.com',
    '/eth/v1/config/spec',
  );

  expect(response.url).toBe('https://ethereum-beacon-api.publicnode.com/eth/v1/config/spec');
});

it('preserves the base pathname when resolving Beacon routes', async () => {
  globalThis.fetch = createUrlEchoFetch();

  const response = await getBeaconJson<{ url: string }>(
    'https://eth-mainnetbeacon.g.alchemy.com/v2/example-key',
    '/eth/v1/config/spec',
  );

  expect(response.url).toBe(
    'https://eth-mainnetbeacon.g.alchemy.com/v2/example-key/eth/v1/config/spec',
  );
});

function createUrlEchoFetch() {
  return async (input: string | URL | Request) => {
    const url =
      typeof input === 'string' ? input : input instanceof URL ? input.toString() : input.url;

    return {
      ok: true,
      json: async () => ({ url }),
      status: 200,
      statusText: 'OK',
    } as Response;
  };
}
