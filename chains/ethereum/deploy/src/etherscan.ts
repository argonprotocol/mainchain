import { createHash } from 'node:crypto';
import { mkdir, readFile, rename, writeFile } from 'node:fs/promises';
import { basename, dirname, join } from 'node:path';
import { AbiCoder, Interface, keccak256 } from 'ethers';
import type { ArtifactManager } from 'hardhat/types/artifacts';
import type { SolidityBuildInfo, SolidityBuildInfoOutput } from 'hardhat/types/solidity';

/** Shared fields in bootstrap, gateway-upgrade, and deterministic-pool manifests. */
export interface DeploymentManifest {
  network: { chainId: string };
  contracts: Record<
    string,
    | {
        address: string;
        constructorArgs?: unknown[];
        initCode?: string;
        /** Keccak-256 of creation bytecode including ABI-encoded constructor arguments. */
        creationCodeHash?: string;
        /** Keccak-256 of observed deployed code, used for the internally created ProxyAdmin. */
        runtimeCodeHash?: string;
        owner?: string;
      }
    | undefined
  >;
}

const contractNames: Record<string, string> = {
  mintingGatewayImplementationBootstrap: 'MintingGateway',
  mintingGatewayImplementationFinal: 'MintingGateway',
  argonBuyAndTransfer: 'ArgonBuyAndTransfer',
  stabilizationPoolHook: 'StabilizationPoolHook',
  stabilizationPoolSubscriber: 'StabilizationPoolSubscriber',
  uniswapV4PriceQuoter: 'UniswapV4PriceQuoter',
  argonToken: 'ArgonToken',
  argonotToken: 'ArgonotToken',
  mintingGatewayProxy: 'TransparentUpgradeableProxy',
  mintingGatewayProxyAdmin: 'ProxyAdmin',
};

interface VerificationRecord {
  /** Address on the report's chain; changed addresses cannot reuse an earlier submission. */
  address: string;
  /** Verified requires published source or an explicitly successful Etherscan job. */
  status: 'verified' | 'pending' | 'failed';
  message: string;
  /** SHA-256 of compiler input binds pending submissions to their source revision. */
  inputHash?: string;
  /** Job identifier saved before polling, allowing verification to resume after restart. */
  guid?: string;
}

interface VerificationReport {
  chainId: string;
  contracts: Record<string, VerificationRecord>;
}

/** Export only required deployment sources, preserving original names and compiler settings. */
export async function outputEtherscanFiles(manifestPath: string) {
  const manifest = JSON.parse(await readFile(manifestPath, 'utf8')) as DeploymentManifest;
  const { artifacts } = await import('hardhat');
  await mkdir(dirname(manifestPath), { recursive: true });
  for (const [key, name] of Object.entries(contractNames)) {
    const contract = manifest.contracts[key];
    if (!contract) continue;
    await prepareVerification(artifacts, name, contract, join(dirname(manifestPath), key));
  }
}

/** Verify a saved deployment without connecting a signer or sending blockchain transactions. */
export async function verifyEtherscanContracts(manifestPath: string, apiKey: string) {
  if (!apiKey.trim()) throw new Error('Set ETHERSCAN_API_KEY before verifying contracts');
  const manifest = JSON.parse(await readFile(manifestPath, 'utf8')) as DeploymentManifest;
  if (!/^\d+$/.test(manifest.network.chainId)) throw new Error('Manifest requires a chain ID');
  const reportPath = join(
    dirname(manifestPath),
    `${basename(manifestPath, '.json')}.etherscan-verification.json`,
  );
  let report: VerificationReport = { chainId: manifest.network.chainId, contracts: {} };
  try {
    report = JSON.parse(await readFile(reportPath, 'utf8')) as VerificationReport;
    if (report.chainId !== manifest.network.chainId) {
      throw new Error('Verification report and deployment manifest have different chain IDs');
    }
  } catch (error) {
    if ((error as NodeJS.ErrnoException).code !== 'ENOENT') throw error;
  }
  for (const [key, record] of Object.entries(report.contracts)) {
    const contract = manifest.contracts[key];
    if (!contract || !contractNames[key]) {
      delete report.contracts[key];
      continue;
    }
    if (record.address.toLowerCase() !== contract.address.toLowerCase()) {
      delete report.contracts[key];
    }
  }

  const { artifacts } = await import('hardhat');
  for (const [key, name] of Object.entries(contractNames)) {
    const contract = manifest.contracts[key];
    if (!contract) continue;
    const previous = report.contracts[key];
    let record: VerificationRecord;
    if (previous?.status === 'pending') {
      record = { ...previous };
    } else {
      record = {
        address: contract.address,
        status: 'failed',
        message: 'Verification has not completed',
      };
    }
    try {
      const existing = await etherscanRequest(apiKey, report.chainId, 'getsourcecode', {
        address: contract.address,
      });
      if (existing.status !== '1' || !Array.isArray(existing.result)) {
        throw new Error(`Cannot check published source: ${JSON.stringify(existing.result)}`);
      }
      if (existing.result[0]?.SourceCode) {
        record.status = 'verified';
        record.message = 'Source already verified';
      } else {
        const input = await prepareVerification(
          artifacts,
          name,
          contract,
          join(dirname(manifestPath), key),
        );
        record.inputHash = createHash('sha256').update(input.sourceCode).digest('hex');
        delete record.guid;
        record.status = 'failed';
        if (
          previous?.address.toLowerCase() === contract.address.toLowerCase() &&
          previous.status === 'pending' &&
          previous.inputHash === record.inputHash
        ) {
          record.guid = previous.guid;
        }
        if (!record.guid) {
          const submitted = await etherscanRequest(apiKey, report.chainId, 'verifysourcecode', {
            contractaddress: contract.address,
            ...input,
          });
          if (submitted.status !== '1') throw new Error(String(submitted.result));
          if (typeof submitted.result !== 'string' || !submitted.result) {
            throw new Error('Etherscan did not return a verification GUID');
          }
          record.guid = submitted.result;
        }
        record.status = 'pending';
        record.message = 'Verification pending';
        report.contracts[key] = record;
        await saveReport(reportPath, report);

        for (let attempt = 0; attempt < 20; attempt += 1) {
          await new Promise(resolve => setTimeout(resolve, 3000));
          const result = await etherscanRequest(apiKey, report.chainId, 'checkverifystatus', {
            guid: record.guid,
          });
          record.message = String(result.result).replaceAll(apiKey, '<REDACTED>');
          if (/pending|in progress/i.test(record.message)) continue;
          if (result.status === '1' && /pass.*verified|already verified/i.test(record.message)) {
            record.status = 'verified';
          } else if (/^fail\b/i.test(record.message)) {
            record.status = 'failed';
          }
          break;
        }
      }
    } catch (error) {
      record.message = String(error).replaceAll(apiKey, '<REDACTED>');
    }
    report.contracts[key] = record;
    await saveReport(reportPath, report);
    console.log(`${key} (${contract.address}): ${record.status} — ${record.message}`);
  }
  return report;
}

/** A verification outage must not turn a saved, successful deployment into a deployment failure. */
export async function verifyDeploymentSources(manifestPath: string, apiKey?: string) {
  const retry = `yarn workspace @argonprotocol/ethereum-deploy etherscan:verify --deployment-manifest ${JSON.stringify(manifestPath)}`;
  try {
    const manifest = JSON.parse(await readFile(manifestPath, 'utf8')) as DeploymentManifest;
    if (['31337', '1337'].includes(manifest.network.chainId)) return;
    if (!apiKey?.trim()) {
      await outputEtherscanFiles(manifestPath);
      console.warn(
        `Deployment saved. Source files exported; set ETHERSCAN_API_KEY and run: ${retry}`,
      );
      return;
    }
    const report = await verifyEtherscanContracts(manifestPath, apiKey);
    if (Object.values(report.contracts).some(record => record.status !== 'verified')) {
      console.warn(`Deployment saved. Source verification remains incomplete; retry: ${retry}`);
    }
  } catch (error) {
    let message = String(error);
    if (apiKey) message = message.replaceAll(apiKey, '<REDACTED>');
    console.warn(`Deployment saved. Source verification failed: ${message}. Retry: ${retry}`);
  }
}

async function prepareVerification(
  artifacts: ArtifactManager,
  name: string,
  contract: NonNullable<DeploymentManifest['contracts'][string]>,
  outputPrefix: string,
) {
  // TransparentUpgradeableProxy creates OpenZeppelin's ProxyAdmin internally, not our local wrapper.
  const artifact = await artifacts.readArtifact(
    name === 'ProxyAdmin' ? 'TransparentUpgradeableProxy' : name,
  );
  const buildInfoPath =
    artifact.buildInfoId && (await artifacts.getBuildInfoPath(artifact.buildInfoId));
  if (!buildInfoPath || !artifact.inputSourceName) {
    throw new Error(
      `Missing build info for ${name}; compile the original deployment sources first`,
    );
  }
  const buildInfo = JSON.parse(await readFile(buildInfoPath, 'utf8')) as SolidityBuildInfo;
  const outputPath = await artifacts.getBuildInfoOutputPath(buildInfo.id);
  if (!outputPath) throw new Error(`Missing original build output for ${name}`);
  const buildOutput = JSON.parse(await readFile(outputPath, 'utf8')) as SolidityBuildInfoOutput;
  let abi = artifact.abi;
  let bytecode = artifact.bytecode;
  let runtimeBytecode = artifact.deployedBytecode;
  let sourceName = artifact.inputSourceName;
  if (name === 'ProxyAdmin') {
    const adminSource = Object.keys(buildInfo.input.sources).find(path =>
      path.endsWith('/proxy/transparent/ProxyAdmin.sol'),
    );
    if (!adminSource) throw new Error('Missing original OpenZeppelin ProxyAdmin build output');
    const compiled = buildOutput.output.contracts?.[adminSource]?.ProxyAdmin;
    if (!compiled?.abi || !compiled.evm?.bytecode?.object || !compiled.evm.deployedBytecode?.object)
      throw new Error('Missing compiled OpenZeppelin ProxyAdmin');
    abi = compiled.abi;
    bytecode = `0x${compiled.evm.bytecode.object}`;
    runtimeBytecode = `0x${compiled.evm.deployedBytecode.object}`;
    sourceName = adminSource;
  }
  let constructorArgs = contract.constructorArgs;
  if (!constructorArgs && name === 'ProxyAdmin' && contract.owner)
    constructorArgs = [contract.owner];
  if (!constructorArgs) throw new Error(`Missing constructor arguments for ${name}`);
  const constructor = new Interface(abi).deploy;
  if (constructor.inputs.length !== constructorArgs.length) {
    throw new Error(
      `${name} constructor revision differs; use the deployment's original Git revision`,
    );
  }
  const encoded = AbiCoder.defaultAbiCoder().encode(constructor.inputs, constructorArgs).slice(2);
  const initCode = `${bytecode}${encoded}`;
  if (!contract.initCode && !contract.creationCodeHash && !contract.runtimeCodeHash) {
    throw new Error(
      `Missing deployment bytecode fingerprint for ${name}; recover it from the original deployment transaction before uploading`,
    );
  }
  if (contract.initCode && contract.initCode !== initCode) {
    throw new Error(
      `${name} bytecode differs from the prepared deployment; use its original Git revision`,
    );
  }
  if (contract.creationCodeHash && contract.creationCodeHash !== keccak256(initCode)) {
    throw new Error(
      `${name} bytecode differs from its deployment receipt; use its original Git revision`,
    );
  }
  if (contract.runtimeCodeHash && contract.runtimeCodeHash !== keccak256(runtimeBytecode)) {
    throw new Error(
      `${name} runtime differs from its deployment receipt; use its original Git revision`,
    );
  }
  const requiredSources = new Set<string>();
  const pendingSources = [sourceName];
  while (pendingSources.length) {
    const currentSource = pendingSources.pop()!;
    if (requiredSources.has(currentSource)) continue;
    const ast = buildOutput.output.sources[currentSource]?.ast;
    if (!ast) throw new Error(`Missing original import graph for ${currentSource}`);
    requiredSources.add(currentSource);
    for (const node of ast.nodes) {
      if (node.nodeType === 'ImportDirective') pendingSources.push(node.absolutePath);
    }
  }

  const input = {
    ...buildInfo.input,
    sources: Object.fromEntries(
      Object.entries(buildInfo.input.sources).filter(([path]) => requiredSources.has(path)),
    ),
  };
  const { solidity } = await import('hardhat');
  const output = await solidity.compileBuildInfo({ ...buildInfo, input }, { quiet: true });
  const errors = output.errors?.filter(error => error.severity === 'error');
  if (errors?.length) {
    throw new Error(
      `Cannot compile reduced ${name} sources: ${errors.map(error => error.message).join('\n')}`,
    );
  }
  const compiled = output.contracts?.[sourceName]?.[name];
  if (`0x${compiled?.evm?.bytecode?.object}` !== bytecode) {
    throw new Error(`Reduced ${name} sources change creation bytecode; refusing to publish`);
  }
  if (`0x${compiled?.evm?.deployedBytecode?.object}` !== runtimeBytecode) {
    throw new Error(`Reduced ${name} sources change runtime bytecode; refusing to publish`);
  }

  const sourceCode = `${JSON.stringify(input, null, 2)}\n`;
  await writeFile(`${outputPrefix}.standard-input.json`, sourceCode);
  await writeFile(`${outputPrefix}.constructor-args.txt`, `${encoded}\n`);
  return {
    sourceCode,
    contractname: `${sourceName}:${name}`,
    compilerversion: `v${buildInfo.solcLongVersion}`,
    codeformat: 'solidity-standard-json-input',
    constructorArguments: encoded,
    licenseType: '3',
  };
}

async function etherscanRequest(
  apiKey: string,
  chainId: string,
  action: string,
  parameters: Record<string, string>,
) {
  // Free Etherscan keys allow three requests per second, including already-verified checks.
  await new Promise(resolve => setTimeout(resolve, 400));
  const url = new URL('https://api.etherscan.io/v2/api');
  url.search = new URLSearchParams({
    apikey: apiKey,
    chainid: chainId,
    module: 'contract',
    action,
  }).toString();
  let response: Response;
  if (action === 'verifysourcecode') {
    response = await fetch(url, {
      method: 'POST',
      headers: { 'content-type': 'application/x-www-form-urlencoded' },
      body: new URLSearchParams(parameters),
      redirect: 'error',
      signal: AbortSignal.timeout(30_000),
    });
  } else {
    for (const [key, value] of Object.entries(parameters)) url.searchParams.set(key, value);
    response = await fetch(url, { redirect: 'error', signal: AbortSignal.timeout(30_000) });
  }
  if (!response.ok) throw new Error(`Etherscan ${action} returned HTTP ${response.status}`);
  return (await response.json()) as { status: string; result: string | { SourceCode: string }[] };
}

async function saveReport(path: string, report: VerificationReport) {
  const temporaryPath = `${path}.${process.pid}.tmp`;
  await writeFile(temporaryPath, `${JSON.stringify(report, null, 2)}\n`);
  await rename(temporaryPath, path);
}
