import { mkdtemp, readFile, rm, writeFile } from 'node:fs/promises';
import { createRequire } from 'node:module';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { AbiCoder, keccak256 } from 'ethers';
import { afterEach, beforeEach, expect, it, vi } from 'vitest';
import {
  outputEtherscanFiles,
  verifyDeploymentSources,
  verifyEtherscanContracts,
} from '../../deploy/src/etherscan.js';

// Exercise the same Hardhat instance that the deployment exporter resolves.
const deploymentRequire = createRequire(new URL('../../deploy/package.json', import.meta.url));
const { artifacts, network } = (await import(
  deploymentRequire.resolve('hardhat')
)) as typeof import('hardhat');

let directory: string;
let quoterInitCode: string;
const address = '0x1111111111111111111111111111111111111111';
const manager = '0x2222222222222222222222222222222222222222';
const apiKey = 'test-api-key-never-persist';

beforeEach(async () => {
  directory = await mkdtemp(join(tmpdir(), 'etherscan-verification-'));
  const artifact = await artifacts.readArtifact('UniswapV4PriceQuoter');
  quoterInitCode = `${artifact.bytecode}${AbiCoder.defaultAbiCoder().encode(['address'], [manager]).slice(2)}`;
  vi.spyOn(console, 'log').mockImplementation(() => undefined);
});

afterEach(async () => {
  vi.restoreAllMocks();
  vi.unstubAllGlobals();
  await rm(directory, { recursive: true, force: true });
});

it('exports only the target import tree while reproducing the original deployment bytecode', async () => {
  const artifact = await artifacts.readArtifact('UniswapV4PriceQuoter');
  const manifestPath = join(directory, 'test-deployment.json');
  await writeFile(
    manifestPath,
    JSON.stringify({
      network: { chainId: '11155111' },
      contracts: {
        uniswapV4PriceQuoter: {
          address,
          constructorArgs: [manager],
          creationCodeHash: keccak256(quoterInitCode),
        },
      },
    }),
  );

  await outputEtherscanFiles(manifestPath);

  const input = JSON.parse(
    await readFile(join(directory, 'uniswapV4PriceQuoter.standard-input.json'), 'utf8'),
  );
  const original = JSON.parse(
    await readFile((await artifacts.getBuildInfoPath(artifact.buildInfoId!))!, 'utf8'),
  );
  const sourceNames = Object.keys(input.sources);
  expect(sourceNames).toContain('project/contracts/UniswapV4PriceQuoter.sol');
  expect(sourceNames.some(path => path.endsWith('/base/BaseV4Quoter.sol'))).toBe(true);
  expect(sourceNames.some(path => path.includes('MintingGateway.sol'))).toBe(false);
  expect(sourceNames.some(path => path.includes('MintingGatewayInvariant.sol'))).toBe(false);
  expect(sourceNames.length).toBeLessThan(Object.keys(original.input.sources).length);
  expect(input.settings).toEqual(original.input.settings);
});

it.each(['creation', 'runtime'] as const)(
  'refuses an upload when reduced sources change %s bytecode',
  async kind => {
    const artifact = await artifacts.readArtifact('UniswapV4PriceQuoter');
    const changed = { ...artifact };
    if (kind === 'creation') changed.bytecode = `${artifact.bytecode}00`;
    else changed.deployedBytecode = `${artifact.deployedBytecode}00`;
    const manifestPath = join(directory, 'test-deployment.json');
    await writeFile(
      manifestPath,
      JSON.stringify({
        network: { chainId: '11155111' },
        contracts: {
          uniswapV4PriceQuoter: {
            address,
            constructorArgs: [manager],
            creationCodeHash: keccak256(
              `${changed.bytecode}${AbiCoder.defaultAbiCoder().encode(['address'], [manager]).slice(2)}`,
            ),
          },
        },
      }),
    );
    vi.spyOn(artifacts, 'readArtifact').mockResolvedValueOnce(changed);

    await expect(outputEtherscanFiles(manifestPath)).rejects.toThrow(
      `sources change ${kind} bytecode`,
    );
    await expect(
      readFile(join(directory, 'uniswapV4PriceQuoter.standard-input.json')),
    ).rejects.toMatchObject({ code: 'ENOENT' });
  },
);

it('resumes a submission after polling fails without resubmitting or changing the deployment', async () => {
  const manifestPath = join(directory, 'deployment.json');
  const manifest = JSON.stringify({
    network: { chainId: '11155111' },
    contracts: {
      uniswapV4PriceQuoter: { address, constructorArgs: [manager], initCode: quoterInitCode },
    },
  });
  await writeFile(manifestPath, manifest);
  let submissions = 0;
  let pollingFails = true;
  let sourceCheckFails = false;
  let pollingRateLimited = false;
  const request = vi.fn(async (url: URL, init?: RequestInit) => {
    expect(url.searchParams.get('chainid')).toBe('11155111');
    const action = url.searchParams.get('action');
    if (action === 'getsourcecode') {
      if (sourceCheckFails) return new Response('Unavailable', { status: 503 });
      return Response.json({ status: '1', result: [{ SourceCode: '' }] });
    }
    if (action === 'verifysourcecode') {
      submissions += 1;
      const body = init?.body as URLSearchParams;
      expect(body.get('contractname')).toBe(
        'project/contracts/UniswapV4PriceQuoter.sol:UniswapV4PriceQuoter',
      );
      expect(body.get('compilerversion')).toBe('v0.8.35+commit.47b9dedd');
      const input = JSON.parse(body.get('sourceCode')!);
      expect(input.sources['project/contracts/UniswapV4PriceQuoter.sol'].content).toContain(
        'contract UniswapV4PriceQuoter',
      );
      expect(input.settings.remappings.length).toBeGreaterThan(0);
      return Response.json({ status: '1', result: 'saved-guid' });
    }
    expect(action).toBe('checkverifystatus');
    expect(url.searchParams.get('guid')).toBe('saved-guid');
    const saved = JSON.parse(
      await readFile(join(directory, 'deployment.etherscan-verification.json'), 'utf8'),
    );
    expect(saved.contracts.uniswapV4PriceQuoter.status).toBe('pending');
    if (pollingFails) throw new Error(`Temporary transport failure ${apiKey}`);
    if (pollingRateLimited) return Response.json({ status: '0', result: 'Max rate limit reached' });
    return Response.json({ status: '1', result: 'Pass - Verified' });
  });
  vi.stubGlobal('fetch', request);

  const first = await verifyEtherscanContracts(manifestPath, apiKey);
  expect(first.contracts.uniswapV4PriceQuoter.status).toBe('pending');
  expect(first.contracts.uniswapV4PriceQuoter.guid).toBe('saved-guid');
  expect(JSON.stringify(first)).not.toContain(apiKey);

  sourceCheckFails = true;
  const lookupFailed = await verifyEtherscanContracts(manifestPath, apiKey);
  expect(lookupFailed.contracts.uniswapV4PriceQuoter.status).toBe('pending');
  expect(lookupFailed.contracts.uniswapV4PriceQuoter.guid).toBe('saved-guid');

  sourceCheckFails = false;
  pollingFails = false;
  pollingRateLimited = true;
  const rateLimited = await verifyEtherscanContracts(manifestPath, apiKey);
  expect(rateLimited.contracts.uniswapV4PriceQuoter.status).toBe('pending');
  expect(rateLimited.contracts.uniswapV4PriceQuoter.guid).toBe('saved-guid');

  pollingRateLimited = false;
  const resumed = await verifyEtherscanContracts(manifestPath, apiKey);
  expect(resumed.contracts.uniswapV4PriceQuoter.status).toBe('verified');
  expect(submissions).toBe(1);
  expect(await readFile(manifestPath, 'utf8')).toBe(manifest);
  expect(
    await readFile(join(directory, 'deployment.etherscan-verification.json'), 'utf8'),
  ).not.toContain(apiKey);
});

it('reports an unsupported historical constructor while still verifying the remaining contracts', async () => {
  const manifestPath = join(directory, 'upgrade.json');
  await writeFile(
    manifestPath,
    JSON.stringify({
      network: { chainId: '11155111' },
      contracts: {
        mintingGatewayImplementationFinal: {
          address: manager,
          constructorArgs: [address, manager],
        },
        uniswapV4PriceQuoter: { address, constructorArgs: [manager], initCode: quoterInitCode },
      },
    }),
  );
  vi.stubGlobal(
    'fetch',
    vi.fn(async (url: URL, init?: RequestInit) => {
      const action = url.searchParams.get('action');
      if (action === 'getsourcecode')
        return Response.json({ status: '1', result: [{ SourceCode: '' }] });
      if (action === 'verifysourcecode') {
        expect((init?.body as URLSearchParams).get('contractaddress')).toBe(address);
        return Response.json({ status: '1', result: 'quoter-guid' });
      }
      return Response.json({ status: '1', result: 'Pass - Verified' });
    }),
  );

  const report = await verifyEtherscanContracts(manifestPath, apiKey);
  expect(report.contracts.mintingGatewayImplementationFinal.status).toBe('failed');
  expect(report.contracts.mintingGatewayImplementationFinal.message).toContain(
    'original Git revision',
  );
  expect(report.contracts.uniswapV4PriceQuoter.status).toBe('verified');
});

it('recognizes already verified historical source without requiring current constructor compatibility', async () => {
  const manifestPath = join(directory, 'historical.json');
  await writeFile(
    manifestPath,
    JSON.stringify({
      network: { chainId: '11155111' },
      contracts: {
        mintingGatewayImplementationFinal: { address, constructorArgs: [address, manager] },
      },
    }),
  );
  const request = vi.fn(async (url: URL) => {
    expect(url.searchParams.get('action')).toBe('getsourcecode');
    return Response.json({ status: '1', result: [{ SourceCode: 'verified historical source' }] });
  });
  vi.stubGlobal('fetch', request);
  const report = await verifyEtherscanContracts(manifestPath, apiKey);
  expect(report.contracts.mintingGatewayImplementationFinal.status).toBe('verified');
  expect(request).toHaveBeenCalledTimes(1);
});

it('keeps a successful deployment saved and returns retry instructions when Etherscan is unavailable', async () => {
  const manifestPath = join(directory, 'saved-deployment.json');
  const manifest = JSON.stringify({
    network: { chainId: '11155111' },
    contracts: {
      uniswapV4PriceQuoter: { address, constructorArgs: [manager], initCode: quoterInitCode },
    },
  });
  await writeFile(manifestPath, manifest);
  vi.stubGlobal(
    'fetch',
    vi.fn(async () => new Response('Unavailable', { status: 503 })),
  );
  const warning = vi.spyOn(console, 'warn').mockImplementation(() => undefined);

  await expect(verifyDeploymentSources(manifestPath, apiKey)).resolves.toBeUndefined();
  expect(await readFile(manifestPath, 'utf8')).toBe(manifest);
  const report = JSON.parse(
    await readFile(join(directory, 'saved-deployment.etherscan-verification.json'), 'utf8'),
  );
  expect(report.contracts.uniswapV4PriceQuoter.status).toBe('failed');
  expect(report.contracts.uniswapV4PriceQuoter.message).toContain('503');
  expect(warning).toHaveBeenCalledWith(expect.stringContaining('etherscan:verify'));
});

it('drops a failed purchase from the report when a later upgrade reuses an existing purchase contract', async () => {
  const manifestPath = join(directory, 'upgrade.json');
  const implementation = { address: manager, constructorArgs: [address, manager, address] };
  await writeFile(
    manifestPath,
    JSON.stringify({
      network: { chainId: '11155111' },
      contracts: {
        argonBuyAndTransfer: { address, constructorArgs: [] },
        mintingGatewayImplementationFinal: implementation,
      },
    }),
  );
  vi.stubGlobal(
    'fetch',
    vi.fn(async (url: URL) => {
      if (url.searchParams.get('address') === address)
        return new Response('Unavailable', { status: 503 });
      return Response.json({ status: '1', result: [{ SourceCode: 'verified implementation' }] });
    }),
  );
  const first = await verifyEtherscanContracts(manifestPath, apiKey);
  expect(first.contracts.argonBuyAndTransfer.status).toBe('failed');

  await writeFile(
    manifestPath,
    JSON.stringify({
      network: { chainId: '11155111' },
      contracts: { mintingGatewayImplementationFinal: implementation },
    }),
  );
  const next = await verifyEtherscanContracts(manifestPath, apiKey);
  expect(Object.keys(next.contracts)).toEqual(['mintingGatewayImplementationFinal']);
  expect(next.contracts.mintingGatewayImplementationFinal.status).toBe('verified');
});

it('refuses an unverified same-arity historical deployment without a bytecode fingerprint', async () => {
  const manifestPath = join(directory, 'historical.json');
  await writeFile(
    manifestPath,
    JSON.stringify({
      network: { chainId: '11155111' },
      contracts: { uniswapV4PriceQuoter: { address, constructorArgs: [manager] } },
    }),
  );
  const request = vi.fn(async (url: URL) => {
    expect(url.searchParams.get('action')).toBe('getsourcecode');
    return Response.json({ status: '1', result: [{ SourceCode: '' }] });
  });
  vi.stubGlobal('fetch', request);
  const report = await verifyEtherscanContracts(manifestPath, apiKey);
  expect(report.contracts.uniswapV4PriceQuoter.status).toBe('failed');
  expect(report.contracts.uniswapV4PriceQuoter.message).toContain(
    'original deployment transaction',
  );
  expect(request).toHaveBeenCalledTimes(1);
});

it('verifies the upstream ProxyAdmin created inside a real transparent proxy deployment', async () => {
  const connection = await network.create();
  try {
    const { ethers } = connection;
    const [owner] = await ethers.getSigners();
    const implementation = await ethers.deployContract('TestArgon');
    const proxy = await ethers.deployContract('TransparentUpgradeableProxy', [
      await implementation.getAddress(),
      owner.address,
      implementation.interface.encodeFunctionData('approve', [owner.address, 0n]),
    ]);
    const adminSlot = '0xb53127684a568b3173ae13b9f8a6016e243e63b6e8ee1178d6a717850b5d6103';
    const admin = `0x${(await ethers.provider.getStorage(await proxy.getAddress(), adminSlot)).slice(-40)}`;
    const runtimeCodeHash = keccak256(await ethers.provider.getCode(admin));
    const manifestPath = join(directory, 'proxy.json');
    await writeFile(
      manifestPath,
      JSON.stringify({
        network: { chainId: '11155111' },
        contracts: {
          mintingGatewayProxyAdmin: { address: admin, owner: owner.address, runtimeCodeHash },
        },
      }),
    );
    vi.stubGlobal(
      'fetch',
      vi.fn(async (url: URL, init?: RequestInit) => {
        const action = url.searchParams.get('action');
        if (action === 'getsourcecode')
          return Response.json({ status: '1', result: [{ SourceCode: '' }] });
        if (action === 'verifysourcecode') {
          expect((init?.body as URLSearchParams).get('contractname')).toMatch(
            /^npm\/.+\/proxy\/transparent\/ProxyAdmin.sol:ProxyAdmin$/,
          );
          return Response.json({ status: '1', result: 'admin-guid' });
        }
        return Response.json({ status: '1', result: 'Pass - Verified' });
      }),
    );
    const report = await verifyEtherscanContracts(manifestPath, apiKey);
    expect(report.contracts.mintingGatewayProxyAdmin.status).toBe('verified');
  } finally {
    await connection.close();
  }
});
