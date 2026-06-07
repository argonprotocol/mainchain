import { network } from 'hardhat';
import { afterAll, describe, expect, it } from 'vitest';
import { Wallet } from 'ethers';
import {
  appendMintingGatewayActivityRoot,
  encodeMintingGatewayGlobalIssuanceCouncilRotateTarget,
  encodeMintingGatewayMintingAuthorityActivationTarget,
  encodeMintingGatewayMintingAuthorityDeactivateTarget,
  hashMintingGatewayActivateMintingAuthorityApproval,
  hashMintingGatewayActivityBlockLocator,
  hashMintingGatewayGatewayUpdateApproval,
  hashMintingGatewayGlobalIssuanceCouncil,
  hashMintingGatewayMintingAuthorityActivatedActivity,
  hashMintingGatewayMintingAuthorization,
  hashMintingGatewayRotateGlobalIssuanceCouncilApproval,
  hashMintingGatewayTransferOutOfArgonFinalizedActivity,
  hashMintingGatewayTransferOutOfArgonRequest,
  hashMintingGatewayTransferToArgonStartedActivity,
  MINTING_GATEWAY_UPDATE_KINDS,
  type MintingGatewayActivityState,
  type MintingGatewayCouncilSnapshot,
  type MintingGatewayGlobalIssuanceCouncilRotateTarget,
  type MintingGatewayHashContext,
  type MintingGatewayMintingAuthorityActivationTarget,
  type MintingGatewayMintingAuthorityDeactivateTarget,
  type MintingGatewayTransferOutOfArgonRequest,
} from '../index.js';
import { expectCustomError } from './assertions.js';

const SCALE = 1_000_000_000_000n;
const MICROGONS_PER_ARGONOT = 1_000_000n;
const ERC1967_ADMIN_SLOT = '0xb53127684a568b3173ae13b9f8a6016e243e63b6e8ee1178d6a717850b5d6103';
const COUNCIL_WEIGHTS = [40n, 30n, 20n, 10n] as const;

const connection = await network.create();
const { ethers } = connection;

type SignerLike = {
  address: string;
  signMessage(message: Uint8Array): Promise<string>;
};

type TypedDataSignerLike = SignerLike & {
  signTypedData(
    domain: {
      name: string;
      version: string;
      chainId: bigint;
      verifyingContract: `0x${string}`;
    },
    types: {
      Permit: Array<{ name: string; type: string }>;
    },
    value: {
      owner: `0x${string}`;
      spender: `0x${string}`;
      value: bigint;
      nonce: bigint;
      deadline: bigint;
    },
  ): Promise<string>;
};

type Council = {
  wallets: SignerLike[];
  signers: `0x${string}`[];
  weights: bigint[];
  memberCount: bigint;
  totalWeight: bigint;
  hash: `0x${string}`;
  epochMicrogonsPerArgonot: bigint;
  snapshot: MintingGatewayCouncilSnapshot;
  quorumSigners: SignerLike[];
};

afterAll(async () => {
  await connection.close();
});

describe('MintingGateway', () => {
  async function getGatewayHashContext(gateway: {
    getAddress(): Promise<string>;
  }): Promise<MintingGatewayHashContext> {
    const { chainId } = await ethers.provider.getNetwork();

    return {
      chainId,
      gatewayAddress: (await gateway.getAddress()) as `0x${string}`,
    };
  }

  async function signPermit(
    signer: TypedDataSignerLike,
    token: {
      getAddress(): Promise<string>;
      getFunction(name: 'nonces'): (owner: string) => Promise<bigint>;
    },
    owner: `0x${string}`,
    spender: `0x${string}`,
    value: bigint,
    deadline: bigint,
    name: string,
  ) {
    const { chainId } = await ethers.provider.getNetwork();
    const nonce = await token.getFunction('nonces')(owner);
    const signature = await signer.signTypedData(
      {
        name,
        version: '1',
        chainId,
        verifyingContract: (await token.getAddress()) as `0x${string}`,
      },
      {
        Permit: [
          { name: 'owner', type: 'address' },
          { name: 'spender', type: 'address' },
          { name: 'value', type: 'uint256' },
          { name: 'nonce', type: 'uint256' },
          { name: 'deadline', type: 'uint256' },
        ],
      },
      {
        owner,
        spender,
        value,
        nonce,
        deadline,
      },
    );

    return ethers.Signature.from(signature);
  }

  function getQuorumCount(weights: bigint[]) {
    const totalWeight = weights.reduce((sum, weight) => sum + weight, 0n);
    let signerWeight = 0n;

    for (let index = 0; index < weights.length; ++index) {
      signerWeight += weights[index] ?? 0n;
      const signerCount = index + 1;
      const unsignedMemberCount = weights.length - signerCount;

      if (signerWeight * 100n >= totalWeight * 90n) return signerCount;
      if (unsignedMemberCount <= 2 && signerWeight * 100n >= totalWeight * 80n) {
        return signerCount;
      }
    }

    throw new Error('Unable to determine quorum count');
  }

  function createCouncil(
    signers: SignerLike[],
    epochMicrogonsPerArgonot = MICROGONS_PER_ARGONOT,
  ): Council {
    const wallets = [...signers].sort((left, right) =>
      left.address.toLowerCase().localeCompare(right.address.toLowerCase()),
    );
    const sortedSigners = wallets.map(wallet => wallet.address as `0x${string}`);
    const weights = wallets.map((_, index) => COUNCIL_WEIGHTS[index] ?? 10n);
    const snapshot = {
      signers: sortedSigners,
      weights,
    } satisfies MintingGatewayCouncilSnapshot;

    return {
      wallets,
      signers: sortedSigners,
      weights,
      memberCount: BigInt(sortedSigners.length),
      totalWeight: weights.reduce((sum, weight) => sum + weight, 0n),
      hash: hashMintingGatewayGlobalIssuanceCouncil({
        ...snapshot,
        epochMicrogonsPerArgonot,
      }),
      epochMicrogonsPerArgonot,
      snapshot,
      quorumSigners: wallets.slice(0, getQuorumCount(weights)),
    };
  }

  async function signApprovalHash(signers: SignerLike[], approvalHash: string) {
    return Promise.all(signers.map(signer => signer.signMessage(ethers.getBytes(approvalHash))));
  }

  async function parseGatewayEvent(
    action: Promise<{
      wait(): Promise<{ logs: Array<{ address: string; data: string; topics: string[] }> }>;
    }>,
    gateway: {
      interface: {
        parseLog(log: {
          address: string;
          data: string;
          topics: string[];
        }): { name: string; args: any } | null;
      };
      getAddress(): Promise<string>;
    },
    eventName: string,
  ) {
    const tx = await action;
    const receipt = await tx.wait();
    const gatewayAddress = (await gateway.getAddress()).toLowerCase();

    const parsedEvent = receipt.logs
      .filter(log => log.address.toLowerCase() === gatewayAddress)
      .flatMap(log => {
        const parsedLog = gateway.interface.parseLog(log);
        return parsedLog === null ? [] : [parsedLog];
      })
      .find(log => log.name === eventName);

    expect(parsedEvent).toBeDefined();
    return parsedEvent!;
  }

  async function parseGatewayEvents(
    action: Promise<{
      wait(): Promise<{ logs: Array<{ address: string; data: string; topics: string[] }> }>;
    }>,
    gateway: {
      interface: {
        parseLog(log: {
          address: string;
          data: string;
          topics: string[];
        }): { name: string; args: any } | null;
      };
      getAddress(): Promise<string>;
    },
    eventName: string,
  ) {
    const tx = await action;
    const receipt = await tx.wait();
    const gatewayAddress = (await gateway.getAddress()).toLowerCase();

    return receipt.logs
      .filter(log => log.address.toLowerCase() === gatewayAddress)
      .flatMap(log => {
        const parsedLog = gateway.interface.parseLog(log);
        return parsedLog === null ? [] : [parsedLog];
      })
      .filter(log => log.name === eventName);
  }

  function asGatewayActivityState(eventState: {
    [index: number]: bigint;
  }): MintingGatewayActivityState {
    return {
      gatewayActivityNonce: eventState[0],
      argonApprovalsNonce: eventState[1],
      argonCirculation: eventState[2],
      argonotCirculation: eventState[3],
    };
  }

  async function deployGatewayStack() {
    const [
      ,
      adminSafe,
      guardian,
      holder,
      recipient,
      councilA,
      councilB,
      councilC,
      councilD,
      outsider,
      mintingAuthoritySigner,
    ] = await ethers.getSigners();

    const gatewayFactory = await ethers.getContractFactory('MintingGateway');
    const gatewayBootstrapImplementation = await gatewayFactory.deploy(
      ethers.ZeroAddress,
      ethers.ZeroAddress,
    );
    await gatewayBootstrapImplementation.waitForDeployment();

    const council = createCouncil([
      councilA as unknown as SignerLike,
      councilB as unknown as SignerLike,
      councilC as unknown as SignerLike,
      councilD as unknown as SignerLike,
    ]);

    const initializeData = gatewayFactory.interface.encodeFunctionData('initialize', [
      adminSafe.address,
      guardian.address,
      council.hash,
      council.memberCount,
      council.totalWeight,
      council.epochMicrogonsPerArgonot,
    ]);

    const gatewayProxyFactory = await ethers.getContractFactory('TransparentUpgradeableProxy');
    const gatewayProxy = await gatewayProxyFactory.deploy(
      await gatewayBootstrapImplementation.getAddress(),
      adminSafe.address,
      initializeData,
    );
    await gatewayProxy.waitForDeployment();

    const gateway = (await ethers.getContractAt(
      'MintingGateway',
      await gatewayProxy.getAddress(),
    )) as any;

    const proxyAdminStorage = await ethers.provider.getStorage(
      await gatewayProxy.getAddress(),
      ERC1967_ADMIN_SLOT,
    );
    const proxyAdminAddress = ethers.getAddress(`0x${proxyAdminStorage.slice(-40)}`);
    const proxyAdmin = (await ethers.getContractAt('ProxyAdmin', proxyAdminAddress)) as any;

    const argonFactory = await ethers.getContractFactory('ArgonToken');
    const argonotFactory = await ethers.getContractFactory('ArgonotToken');

    const argon = (await argonFactory.deploy(await gatewayProxy.getAddress())) as any;
    const argonot = (await argonotFactory.deploy(await gatewayProxy.getAddress())) as any;
    await Promise.all([argon.waitForDeployment(), argonot.waitForDeployment()]);

    const gatewayFinalImplementation = (await gatewayFactory.deploy(
      await argon.getAddress(),
      await argonot.getAddress(),
    )) as any;
    await gatewayFinalImplementation.waitForDeployment();

    return {
      adminSafe,
      guardian,
      holder,
      recipient,
      outsider,
      mintingAuthoritySigner: mintingAuthoritySigner as unknown as SignerLike,
      council,
      gateway,
      proxyAdmin,
      gatewayFinalImplementation,
      argon,
      argonot,
      relayerArgonAccountId: ethers.encodeBytes32String('relayer-1'),
    };
  }

  async function deployFixture() {
    const stack = await deployGatewayStack();

    await stack.proxyAdmin
      .connect(stack.adminSafe)
      .upgradeAndCall(
        await stack.gateway.getAddress(),
        await stack.gatewayFinalImplementation.getAddress(),
        '0x',
      );

    await stack.gateway
      .connect(stack.adminSafe)
      .migrate(
        { recipients: [stack.holder.address], amounts: [1_000n * SCALE] },
        { recipients: [stack.holder.address], amounts: [2_000n * SCALE] },
      );

    return stack;
  }

  async function activateMintingAuthority(
    fixture: Awaited<ReturnType<typeof deployFixture>>,
    queueNonce = 1n,
    overrides: Partial<MintingGatewayMintingAuthorityActivationTarget> = {},
  ) {
    const target = {
      microgonCollateral: overrides.microgonCollateral ?? 1_000n,
      micronotCollateral: overrides.micronotCollateral ?? 200n,
      signingKey: overrides.signingKey ?? (fixture.mintingAuthoritySigner.address as `0x${string}`),
    } satisfies MintingGatewayMintingAuthorityActivationTarget;
    const previousUpdateHash = await fixture.gateway.getFunction('argonApprovalsHash')();
    const approvalHash = hashMintingGatewayActivateMintingAuthorityApproval(
      await getGatewayHashContext(fixture.gateway),
      {
        queueNonce,
        approvingCouncilHash: fixture.council.hash,
        previousUpdateHash,
        target,
      },
    );
    const signatures = await signApprovalHash(fixture.council.quorumSigners, approvalHash);
    const payload = encodeMintingGatewayMintingAuthorityActivationTarget(target);

    const event = await parseGatewayEvent(
      fixture.gateway.applyGatewayUpdates(
        fixture.council.snapshot,
        [
          {
            queueNonce,
            kind: MINTING_GATEWAY_UPDATE_KINDS.mintingAuthorityActivate,
            payload,
            signatures,
          },
        ],
        fixture.relayerArgonAccountId,
      ),
      fixture.gateway,
      'MintingAuthorityActivated',
    );
    expect(event.args[3]).to.equal(1n);
    expect(event.args[4]).to.equal(BigInt(fixture.council.quorumSigners.length));
    expect(event.args[5]).to.equal(approvalHash);

    return { event, target, approvalHash };
  }

  async function rotateCouncil(
    fixture: Awaited<ReturnType<typeof deployFixture>>,
    queueNonce: bigint,
    nextCouncil: Council,
  ) {
    const previousUpdateHash = await fixture.gateway.getFunction('argonApprovalsHash')();
    const target = {
      council: nextCouncil.snapshot,
      epochMicrogonsPerArgonot: nextCouncil.epochMicrogonsPerArgonot,
    } satisfies MintingGatewayGlobalIssuanceCouncilRotateTarget;
    const approvalHash = hashMintingGatewayRotateGlobalIssuanceCouncilApproval(
      await getGatewayHashContext(fixture.gateway),
      {
        queueNonce,
        approvingCouncilHash: fixture.council.hash,
        previousUpdateHash,
        target,
      },
    );
    const signatures = await signApprovalHash(fixture.council.quorumSigners, approvalHash);

    await fixture.gateway.applyGatewayUpdates(
      fixture.council.snapshot,
      [
        {
          queueNonce,
          kind: MINTING_GATEWAY_UPDATE_KINDS.globalIssuanceCouncilRotate,
          payload: encodeMintingGatewayGlobalIssuanceCouncilRotateTarget(target),
          signatures,
        },
      ],
      fixture.relayerArgonAccountId,
    );

    return approvalHash;
  }

  async function createTransferOutRequest(
    fixture: Awaited<ReturnType<typeof deployFixture>>,
    overrides: Partial<MintingGatewayTransferOutOfArgonRequest> = {},
  ): Promise<MintingGatewayTransferOutOfArgonRequest> {
    const chainId = BigInt((await ethers.provider.getNetwork()).chainId);
    const activeCouncil = await fixture.gateway.globalIssuanceCouncil();

    return {
      argonAccountId:
        overrides.argonAccountId ?? (ethers.encodeBytes32String('account-1') as `0x${string}`),
      argonTransferNonce: overrides.argonTransferNonce ?? 1n,
      chainId,
      microgonsPerArgonot: overrides.microgonsPerArgonot ?? activeCouncil.epochMicrogonsPerArgonot,
      recipient: overrides.recipient ?? (fixture.recipient.address as `0x${string}`),
      validUntilBlock: overrides.validUntilBlock ?? 1_000_000n,
      token: overrides.token ?? ((await fixture.argon.getAddress()) as `0x${string}`),
      amount: overrides.amount ?? 50n,
      mintingAuthorityTip: overrides.mintingAuthorityTip ?? 5n,
    };
  }

  it('rejects token-bearing transfer starts on the bootstrap implementation before canonical tokens exist', async () => {
    const { gateway, holder } = await deployGatewayStack();

    await expectCustomError(
      gateway
        .connect(holder)
        .startTransferToArgon(
          ethers.ZeroAddress,
          1n,
          ethers.encodeBytes32String('bootstrap'),
          0n,
          27,
          ethers.ZeroHash,
          ethers.ZeroHash,
        ),
      gateway,
      'UnsupportedToken',
      [ethers.ZeroAddress],
    );
  });

  it('starts a transfer to Argon in the ordered gateway activity stream', async () => {
    const { argon, argonot, gateway, holder } = await deployFixture();
    const argonAccountId = ethers.encodeBytes32String('argon-account-1');
    const hashContext = await getGatewayHashContext(gateway);
    const deadline = BigInt((await ethers.provider.getBlock('latest'))!.timestamp) + 3600n;
    const permit = await signPermit(
      holder as unknown as TypedDataSignerLike,
      argon,
      holder.address as `0x${string}`,
      (await gateway.getAddress()) as `0x${string}`,
      250n * SCALE,
      deadline,
      'Argon',
    );

    const event = await parseGatewayEvent(
      gateway
        .connect(holder)
        .startTransferToArgon(
          await argon.getAddress(),
          250n,
          argonAccountId,
          deadline,
          permit.v,
          permit.r,
          permit.s,
        ),
      gateway,
      'TransferToArgonStarted',
    );

    expect(event.args[0]).to.equal(holder.address);
    expect(event.args[1]).to.equal(await argon.getAddress());
    expect(event.args[2]).to.equal(250n);
    expect(event.args[3]).to.equal(argonAccountId);
    expect(event.args[4][0]).to.equal(1n);
    expect(event.args[4][1]).to.equal(0n);
    expect(event.args[4][2]).to.equal(750n);
    expect(event.args[4][3]).to.equal(2_000n);

    const locator = await gateway.activityBlockLocators(1n);
    expect(await gateway.gatewayActivityNonce()).to.equal(1n);
    expect(await gateway.latestActivityBlockLocatorIndex()).to.equal(1n);
    expect(locator.startGatewayActivityNonce).to.equal(1n);
    expect(locator.endGatewayActivityNonce).to.equal(1n);
    expect(locator.activityRoot).to.equal(
      appendMintingGatewayActivityRoot(
        ethers.ZeroHash as `0x${string}`,
        hashMintingGatewayTransferToArgonStartedActivity(hashContext, {
          from: holder.address as `0x${string}`,
          token: (await argon.getAddress()) as `0x${string}`,
          amount: 250n,
          argonAccountId: argonAccountId as `0x${string}`,
          gatewayState: asGatewayActivityState(event.args[4]),
        }),
      ),
    );

    expect(await argon.balanceOf(holder.address)).to.equal(750n * SCALE);
    expect(await argon.allowance(holder.address, await gateway.getAddress())).to.equal(0n);
    expect(await argonot.balanceOf(holder.address)).to.equal(2_000n * SCALE);
  });

  it('rejects zero runtime-unit transfer amounts', async () => {
    const { gateway, holder } = await deployFixture();
    const argonAccountId = ethers.encodeBytes32String('argon-account-1');

    await expectCustomError(
      gateway
        .connect(holder)
        .startTransferToArgon(
          await gateway.argonToken(),
          0n,
          argonAccountId,
          0n,
          27,
          ethers.ZeroHash,
          ethers.ZeroHash,
        ),
      gateway,
      'ZeroAmount',
    );
  });

  it('only lets the owner force-update the active council summary', async () => {
    const fixture = await deployFixture();
    const nextMicrogonsPerArgonot = 2n * MICROGONS_PER_ARGONOT;
    const nextCouncil = createCouncil(
      [
        Wallet.createRandom() as unknown as SignerLike,
        Wallet.createRandom() as unknown as SignerLike,
        Wallet.createRandom() as unknown as SignerLike,
        Wallet.createRandom() as unknown as SignerLike,
      ],
      nextMicrogonsPerArgonot,
    );

    await expectCustomError(
      fixture.gateway
        .connect(fixture.outsider)
        .forceUpdateActiveCouncil(nextCouncil.snapshot, nextCouncil.epochMicrogonsPerArgonot),
      fixture.gateway,
      'OwnableUnauthorizedAccount',
      [fixture.outsider.address],
    );

    const event = await parseGatewayEvent(
      fixture.gateway
        .connect(fixture.adminSafe)
        .forceUpdateActiveCouncil(nextCouncil.snapshot, nextCouncil.epochMicrogonsPerArgonot),
      fixture.gateway,
      'GlobalIssuanceCouncilForceUpdated',
    );
    const activeCouncil = await fixture.gateway.globalIssuanceCouncil();

    expect(event.args[0]).to.equal(fixture.council.hash);
    expect(event.args[1]).to.equal(nextCouncil.hash);
    expect(activeCouncil.councilHash).to.equal(nextCouncil.hash);
    expect(activeCouncil.epochMicrogonsPerArgonot).to.equal(nextMicrogonsPerArgonot);
    expect(await fixture.gateway.maxTransferOutMicrogonsPerArgonot()).to.equal(
      fixture.council.epochMicrogonsPerArgonot,
    );
    expect(activeCouncil.memberCount).to.equal(nextCouncil.memberCount);
    expect(activeCouncil.totalWeight).to.equal(nextCouncil.totalWeight);
    expect(await fixture.gateway.argonApprovalsNonce()).to.equal(0n);
    expect(await fixture.gateway.getFunction('argonApprovalsHash')()).to.equal(ethers.ZeroHash);
  });

  it('rejects gateway relay batches that exceed the update cap', async () => {
    const fixture = await deployFixture();

    await expectCustomError(
      fixture.gateway.applyGatewayUpdates(
        fixture.council.snapshot,
        Array.from({ length: 101 }, (_, index) => ({
          queueNonce: BigInt(index + 1),
          kind: MINTING_GATEWAY_UPDATE_KINDS.mintingAuthorityActivate,
          payload: '0x',
          signatures: [],
        })),
        fixture.relayerArgonAccountId,
      ),
      fixture.gateway,
      'TooManyGatewayUpdates',
      [100n, 101n],
    );
  });

  it('continues queue processing from the same approvals hash after a forced council update', async () => {
    const fixture = await deployFixture();
    const nextCouncil = createCouncil([
      Wallet.createRandom() as unknown as SignerLike,
      Wallet.createRandom() as unknown as SignerLike,
      Wallet.createRandom() as unknown as SignerLike,
      Wallet.createRandom() as unknown as SignerLike,
    ]);
    const target = {
      microgonCollateral: 1_500n,
      micronotCollateral: 250n,
      signingKey: Wallet.createRandom().address as `0x${string}`,
    } satisfies MintingGatewayMintingAuthorityActivationTarget;

    await fixture.gateway
      .connect(fixture.adminSafe)
      .forceUpdateActiveCouncil(nextCouncil.snapshot, nextCouncil.epochMicrogonsPerArgonot);

    const approvalHash = hashMintingGatewayActivateMintingAuthorityApproval(
      await getGatewayHashContext(fixture.gateway),
      {
        queueNonce: 1n,
        approvingCouncilHash: nextCouncil.hash,
        previousUpdateHash: await fixture.gateway.getFunction('argonApprovalsHash')(),
        target,
      },
    );
    const signatures = await signApprovalHash(nextCouncil.quorumSigners, approvalHash);

    await fixture.gateway.applyGatewayUpdates(
      nextCouncil.snapshot,
      [
        {
          queueNonce: 1n,
          kind: MINTING_GATEWAY_UPDATE_KINDS.mintingAuthorityActivate,
          payload: encodeMintingGatewayMintingAuthorityActivationTarget(target),
          signatures,
        },
      ],
      fixture.relayerArgonAccountId,
    );

    const mintingCollateral = await fixture.gateway.mintingAuthorityCollateralRemaining(
      target.signingKey,
    );
    expect(mintingCollateral.microgonCollateral).to.equal(target.microgonCollateral);
    expect(mintingCollateral.micronotCollateral).to.equal(target.micronotCollateral);
    expect(await fixture.gateway.argonApprovalsNonce()).to.equal(1n);
    expect(await fixture.gateway.getFunction('argonApprovalsHash')()).to.equal(approvalHash);
  });

  it('requires council signatures when a minting authority deactivation is the last relayed item', async () => {
    const fixture = await deployFixture();
    const { gateway, council, relayerArgonAccountId } = fixture;
    const { event: activationEvent, target } = await activateMintingAuthority(fixture);

    const deactivationTarget = {
      signingKey: target.signingKey,
    } satisfies MintingGatewayMintingAuthorityDeactivateTarget;
    const deactivationTargetPayloadHash = ethers.keccak256(
      encodeMintingGatewayMintingAuthorityDeactivateTarget(deactivationTarget),
    ) as `0x${string}`;
    const deactivationHash = hashMintingGatewayGatewayUpdateApproval(
      await getGatewayHashContext(gateway),
      {
        queueNonce: 2n,
        approvingCouncilHash: council.hash,
        kind: MINTING_GATEWAY_UPDATE_KINDS.mintingAuthorityDeactivate,
        targetId: `0x${deactivationTarget.signingKey.slice(2).padStart(64, '0').toLowerCase()}`,
        targetPayloadHash: deactivationTargetPayloadHash,
        previousUpdateHash: await gateway.getFunction('argonApprovalsHash')(),
      },
    );
    const wrongDeactivateSignature = await fixture.mintingAuthoritySigner.signMessage(
      ethers.getBytes(deactivationHash),
    );
    const deactivateSignatures = await signApprovalHash(council.quorumSigners, deactivationHash);

    expect(activationEvent.args[0]).to.equal(target.signingKey);

    await expectCustomError(
      gateway.applyGatewayUpdates(
        fixture.council.snapshot,
        [
          {
            queueNonce: 2n,
            kind: MINTING_GATEWAY_UPDATE_KINDS.mintingAuthorityDeactivate,
            payload: encodeMintingGatewayMintingAuthorityDeactivateTarget(deactivationTarget),
            signatures: [wrongDeactivateSignature],
          },
        ],
        relayerArgonAccountId,
      ),
      gateway,
      'InvalidGlobalIssuanceCouncilMember',
      [0n],
    );

    const tx = gateway.applyGatewayUpdates(
      fixture.council.snapshot,
      [
        {
          queueNonce: 2n,
          kind: MINTING_GATEWAY_UPDATE_KINDS.mintingAuthorityDeactivate,
          payload: encodeMintingGatewayMintingAuthorityDeactivateTarget(deactivationTarget),
          signatures: deactivateSignatures,
        },
      ],
      relayerArgonAccountId,
    );
    const deactivateEvent = await parseGatewayEvent(tx, gateway, 'MintingAuthorityDeactivated');

    const mintingCollateral = await gateway.mintingAuthorityCollateralRemaining(target.signingKey);
    expect(mintingCollateral.microgonCollateral).to.equal(0n);
    expect(mintingCollateral.micronotCollateral).to.equal(0n);
    expect(await gateway.argonApprovalsNonce()).to.equal(2n);
    expect(await gateway.gatewayActivityNonce()).to.equal(2n);

    expect(deactivateEvent.args[0]).to.equal(target.signingKey);
    expect(deactivateEvent.args[1]).to.equal(target.microgonCollateral);
    expect(deactivateEvent.args[2]).to.equal(target.micronotCollateral);
    expect(deactivateEvent.args[3]).to.equal(deactivationHash);
  });

  it('shares one final signature block across activations separated by a deactivation', async () => {
    const fixture = await deployFixture();
    const hashContext = await getGatewayHashContext(fixture.gateway);
    const { target: activeTarget } = await activateMintingAuthority(fixture);
    const firstTarget = {
      microgonCollateral: 1_000n,
      micronotCollateral: 100n,
      signingKey: Wallet.createRandom().address as `0x${string}`,
    } satisfies MintingGatewayMintingAuthorityActivationTarget;
    const secondTarget = {
      microgonCollateral: 2_000n,
      micronotCollateral: 150n,
      signingKey: Wallet.createRandom().address as `0x${string}`,
    } satisfies MintingGatewayMintingAuthorityActivationTarget;
    const activationOneHash = hashMintingGatewayActivateMintingAuthorityApproval(hashContext, {
      queueNonce: 2n,
      approvingCouncilHash: fixture.council.hash,
      previousUpdateHash: await fixture.gateway.getFunction('argonApprovalsHash')(),
      target: firstTarget,
    });
    const deactivationTarget = {
      signingKey: activeTarget.signingKey,
    } satisfies MintingGatewayMintingAuthorityDeactivateTarget;
    const deactivationHash = hashMintingGatewayGatewayUpdateApproval(hashContext, {
      queueNonce: 3n,
      approvingCouncilHash: fixture.council.hash,
      kind: MINTING_GATEWAY_UPDATE_KINDS.mintingAuthorityDeactivate,
      targetId: `0x${deactivationTarget.signingKey.slice(2).padStart(64, '0').toLowerCase()}`,
      targetPayloadHash: ethers.keccak256(
        encodeMintingGatewayMintingAuthorityDeactivateTarget(deactivationTarget),
      ) as `0x${string}`,
      previousUpdateHash: activationOneHash,
    });
    const activationTwoHash = hashMintingGatewayActivateMintingAuthorityApproval(hashContext, {
      queueNonce: 4n,
      approvingCouncilHash: fixture.council.hash,
      previousUpdateHash: deactivationHash,
      target: secondTarget,
    });
    const activationTwoSignatures = await signApprovalHash(
      fixture.council.quorumSigners,
      activationTwoHash,
    );

    const activationEvents = await parseGatewayEvents(
      fixture.gateway.applyGatewayUpdates(
        fixture.council.snapshot,
        [
          {
            queueNonce: 2n,
            kind: MINTING_GATEWAY_UPDATE_KINDS.mintingAuthorityActivate,
            payload: encodeMintingGatewayMintingAuthorityActivationTarget(firstTarget),
            signatures: [],
          },
          {
            queueNonce: 3n,
            kind: MINTING_GATEWAY_UPDATE_KINDS.mintingAuthorityDeactivate,
            payload: encodeMintingGatewayMintingAuthorityDeactivateTarget(deactivationTarget),
            signatures: [],
          },
          {
            queueNonce: 4n,
            kind: MINTING_GATEWAY_UPDATE_KINDS.mintingAuthorityActivate,
            payload: encodeMintingGatewayMintingAuthorityActivationTarget(secondTarget),
            signatures: activationTwoSignatures,
          },
        ],
        fixture.relayerArgonAccountId,
      ),
      fixture.gateway,
      'MintingAuthorityActivated',
    );

    expect(activationEvents).toHaveLength(2);
    expect(activationEvents[0].args[0]).to.equal(firstTarget.signingKey);
    expect(activationEvents[0].args[3]).to.equal(2n);
    expect(activationEvents[0].args[4]).to.equal(BigInt(activationTwoSignatures.length));
    expect(activationEvents[0].args[5]).to.equal(activationOneHash);
    expect(activationEvents[1].args[0]).to.equal(secondTarget.signingKey);
    expect(activationEvents[1].args[3]).to.equal(2n);
    expect(activationEvents[1].args[4]).to.equal(BigInt(activationTwoSignatures.length));
    expect(activationEvents[1].args[5]).to.equal(activationTwoHash);
    const deactivatedCollateral = await fixture.gateway.mintingAuthorityCollateralRemaining(
      activeTarget.signingKey,
    );
    expect(deactivatedCollateral.microgonCollateral).to.equal(0n);
    expect(deactivatedCollateral.micronotCollateral).to.equal(0n);
  });

  it('carries rotation signatures forward across a deactivation until the first later activation', async () => {
    const fixture = await deployFixture();
    const hashContext = await getGatewayHashContext(fixture.gateway);
    const { target: activeTarget } = await activateMintingAuthority(fixture);
    const nextCouncil = createCouncil([
      fixture.adminSafe,
      fixture.guardian,
      fixture.holder,
      fixture.outsider,
    ]);
    const activationTarget = {
      microgonCollateral: 2_000n,
      micronotCollateral: 150n,
      signingKey: Wallet.createRandom().address as `0x${string}`,
    } satisfies MintingGatewayMintingAuthorityActivationTarget;
    const rotationTarget = {
      council: nextCouncil.snapshot,
      epochMicrogonsPerArgonot: nextCouncil.epochMicrogonsPerArgonot,
    } satisfies MintingGatewayGlobalIssuanceCouncilRotateTarget;
    const rotationHash = hashMintingGatewayRotateGlobalIssuanceCouncilApproval(hashContext, {
      queueNonce: 2n,
      approvingCouncilHash: fixture.council.hash,
      previousUpdateHash: await fixture.gateway.getFunction('argonApprovalsHash')(),
      target: rotationTarget,
    });
    const deactivationTarget = {
      signingKey: activeTarget.signingKey,
    } satisfies MintingGatewayMintingAuthorityDeactivateTarget;
    const deactivationHash = hashMintingGatewayGatewayUpdateApproval(hashContext, {
      queueNonce: 3n,
      approvingCouncilHash: nextCouncil.hash,
      kind: MINTING_GATEWAY_UPDATE_KINDS.mintingAuthorityDeactivate,
      targetId: `0x${deactivationTarget.signingKey.slice(2).padStart(64, '0').toLowerCase()}`,
      targetPayloadHash: ethers.keccak256(
        encodeMintingGatewayMintingAuthorityDeactivateTarget(deactivationTarget),
      ) as `0x${string}`,
      previousUpdateHash: rotationHash,
    });
    const activationHash = hashMintingGatewayActivateMintingAuthorityApproval(hashContext, {
      queueNonce: 4n,
      approvingCouncilHash: nextCouncil.hash,
      previousUpdateHash: deactivationHash,
      target: activationTarget,
    });
    const rotationSignatures = await signApprovalHash(fixture.council.quorumSigners, rotationHash);
    const activationSignatures = await signApprovalHash(nextCouncil.quorumSigners, activationHash);

    const activationEvent = await parseGatewayEvent(
      fixture.gateway.applyGatewayUpdates(
        fixture.council.snapshot,
        [
          {
            queueNonce: 2n,
            kind: MINTING_GATEWAY_UPDATE_KINDS.globalIssuanceCouncilRotate,
            payload: encodeMintingGatewayGlobalIssuanceCouncilRotateTarget(rotationTarget),
            signatures: rotationSignatures,
          },
          {
            queueNonce: 3n,
            kind: MINTING_GATEWAY_UPDATE_KINDS.mintingAuthorityDeactivate,
            payload: encodeMintingGatewayMintingAuthorityDeactivateTarget(deactivationTarget),
            signatures: [],
          },
          {
            queueNonce: 4n,
            kind: MINTING_GATEWAY_UPDATE_KINDS.mintingAuthorityActivate,
            payload: encodeMintingGatewayMintingAuthorityActivationTarget(activationTarget),
            signatures: activationSignatures,
          },
        ],
        fixture.relayerArgonAccountId,
      ),
      fixture.gateway,
      'MintingAuthorityActivated',
    );

    expect(activationEvent.args[0]).to.equal(activationTarget.signingKey);
    expect(activationEvent.args[3]).to.equal(1n);
    expect(activationEvent.args[4]).to.equal(
      BigInt(rotationSignatures.length + activationSignatures.length),
    );
    expect(activationEvent.args[5]).to.equal(activationHash);
    const deactivatedCollateral = await fixture.gateway.mintingAuthorityCollateralRemaining(
      activeTarget.signingKey,
    );
    expect(deactivatedCollateral.microgonCollateral).to.equal(0n);
    expect(deactivatedCollateral.micronotCollateral).to.equal(0n);
  });

  it('only needs council signatures on rotation items and the last relayed item in a batch', async () => {
    const fixture = await deployFixture();
    const nextCouncil = createCouncil([
      fixture.adminSafe,
      fixture.guardian,
      fixture.holder,
      fixture.outsider,
    ]);
    const firstTarget = {
      microgonCollateral: 1_000n,
      micronotCollateral: 100n,
      signingKey: Wallet.createRandom().address as `0x${string}`,
    } satisfies MintingGatewayMintingAuthorityActivationTarget;
    const secondTarget = {
      microgonCollateral: 2_000n,
      micronotCollateral: 150n,
      signingKey: Wallet.createRandom().address as `0x${string}`,
    } satisfies MintingGatewayMintingAuthorityActivationTarget;
    const activationOneHash = hashMintingGatewayActivateMintingAuthorityApproval(
      await getGatewayHashContext(fixture.gateway),
      {
        queueNonce: 1n,
        approvingCouncilHash: fixture.council.hash,
        previousUpdateHash: ethers.ZeroHash as `0x${string}`,
        target: firstTarget,
      },
    );
    const rotationTarget = {
      council: nextCouncil.snapshot,
      epochMicrogonsPerArgonot: nextCouncil.epochMicrogonsPerArgonot,
    } satisfies MintingGatewayGlobalIssuanceCouncilRotateTarget;
    const rotationHash = hashMintingGatewayRotateGlobalIssuanceCouncilApproval(
      await getGatewayHashContext(fixture.gateway),
      {
        queueNonce: 2n,
        approvingCouncilHash: fixture.council.hash,
        previousUpdateHash: activationOneHash,
        target: rotationTarget,
      },
    );
    const activationTwoHash = hashMintingGatewayActivateMintingAuthorityApproval(
      await getGatewayHashContext(fixture.gateway),
      {
        queueNonce: 3n,
        approvingCouncilHash: nextCouncil.hash,
        previousUpdateHash: rotationHash,
        target: secondTarget,
      },
    );
    const rotationSignatures = await signApprovalHash(fixture.council.quorumSigners, rotationHash);
    const activationTwoSignatures = await signApprovalHash(
      nextCouncil.quorumSigners,
      activationTwoHash,
    );

    const activationEvents = await parseGatewayEvents(
      fixture.gateway.applyGatewayUpdates(
        fixture.council.snapshot,
        [
          {
            queueNonce: 1n,
            kind: MINTING_GATEWAY_UPDATE_KINDS.mintingAuthorityActivate,
            payload: encodeMintingGatewayMintingAuthorityActivationTarget(firstTarget),
            signatures: [],
          },
          {
            queueNonce: 2n,
            kind: MINTING_GATEWAY_UPDATE_KINDS.globalIssuanceCouncilRotate,
            payload: encodeMintingGatewayGlobalIssuanceCouncilRotateTarget(rotationTarget),
            signatures: rotationSignatures,
          },
          {
            queueNonce: 3n,
            kind: MINTING_GATEWAY_UPDATE_KINDS.mintingAuthorityActivate,
            payload: encodeMintingGatewayMintingAuthorityActivationTarget(secondTarget),
            signatures: activationTwoSignatures,
          },
        ],
        fixture.relayerArgonAccountId,
      ),
      fixture.gateway,
      'MintingAuthorityActivated',
    );

    const firstMintingCollateral = await fixture.gateway.mintingAuthorityCollateralRemaining(
      firstTarget.signingKey,
    );
    const secondMintingCollateral = await fixture.gateway.mintingAuthorityCollateralRemaining(
      secondTarget.signingKey,
    );
    const activeCouncil = await fixture.gateway.globalIssuanceCouncil();

    expect(firstMintingCollateral.microgonCollateral).to.equal(firstTarget.microgonCollateral);
    expect(secondMintingCollateral.microgonCollateral).to.equal(secondTarget.microgonCollateral);
    expect(await fixture.gateway.argonApprovalsNonce()).to.equal(3n);
    expect(await fixture.gateway.getFunction('argonApprovalsHash')()).to.equal(activationTwoHash);
    expect(activeCouncil.councilHash).to.equal(nextCouncil.hash);
    expect(activeCouncil.epochMicrogonsPerArgonot).to.equal(nextCouncil.epochMicrogonsPerArgonot);
    expect(activationEvents).toHaveLength(2);
    expect(activationEvents[0].args[0]).to.equal(firstTarget.signingKey);
    expect(activationEvents[0].args[3]).to.equal(1n);
    expect(activationEvents[0].args[4]).to.equal(BigInt(rotationSignatures.length));
    expect(activationEvents[0].args[5]).to.equal(activationOneHash);
    expect(activationEvents[1].args[0]).to.equal(secondTarget.signingKey);
    expect(activationEvents[1].args[3]).to.equal(1n);
    expect(activationEvents[1].args[4]).to.equal(BigInt(activationTwoSignatures.length));
    expect(activationEvents[1].args[5]).to.equal(activationTwoHash);
  });

  it('carries unresolved rotation signature cost forward to the first later activation in a batch', async () => {
    const fixture = await deployFixture();
    const hashContext = await getGatewayHashContext(fixture.gateway);
    const firstNextCouncil = createCouncil([
      Wallet.createRandom() as unknown as SignerLike,
      Wallet.createRandom() as unknown as SignerLike,
      Wallet.createRandom() as unknown as SignerLike,
      Wallet.createRandom() as unknown as SignerLike,
    ]);
    const secondNextCouncil = createCouncil([
      Wallet.createRandom() as unknown as SignerLike,
      Wallet.createRandom() as unknown as SignerLike,
      Wallet.createRandom() as unknown as SignerLike,
      Wallet.createRandom() as unknown as SignerLike,
    ]);
    const thirdNextCouncil = createCouncil([
      Wallet.createRandom() as unknown as SignerLike,
      Wallet.createRandom() as unknown as SignerLike,
      Wallet.createRandom() as unknown as SignerLike,
      Wallet.createRandom() as unknown as SignerLike,
    ]);
    const activationTarget = {
      microgonCollateral: 1_000n,
      micronotCollateral: 100n,
      signingKey: Wallet.createRandom().address as `0x${string}`,
    } satisfies MintingGatewayMintingAuthorityActivationTarget;
    const firstRotationTarget = {
      council: firstNextCouncil.snapshot,
      epochMicrogonsPerArgonot: firstNextCouncil.epochMicrogonsPerArgonot,
    } satisfies MintingGatewayGlobalIssuanceCouncilRotateTarget;
    const firstRotationHash = hashMintingGatewayRotateGlobalIssuanceCouncilApproval(hashContext, {
      queueNonce: 1n,
      approvingCouncilHash: fixture.council.hash,
      previousUpdateHash: ethers.ZeroHash as `0x${string}`,
      target: firstRotationTarget,
    });
    const secondRotationTarget = {
      council: secondNextCouncil.snapshot,
      epochMicrogonsPerArgonot: secondNextCouncil.epochMicrogonsPerArgonot,
    } satisfies MintingGatewayGlobalIssuanceCouncilRotateTarget;
    const secondRotationHash = hashMintingGatewayRotateGlobalIssuanceCouncilApproval(hashContext, {
      queueNonce: 2n,
      approvingCouncilHash: firstNextCouncil.hash,
      previousUpdateHash: firstRotationHash,
      target: secondRotationTarget,
    });
    const thirdRotationTarget = {
      council: thirdNextCouncil.snapshot,
      epochMicrogonsPerArgonot: thirdNextCouncil.epochMicrogonsPerArgonot,
    } satisfies MintingGatewayGlobalIssuanceCouncilRotateTarget;
    const thirdRotationHash = hashMintingGatewayRotateGlobalIssuanceCouncilApproval(hashContext, {
      queueNonce: 3n,
      approvingCouncilHash: secondNextCouncil.hash,
      previousUpdateHash: secondRotationHash,
      target: thirdRotationTarget,
    });
    const activationHash = hashMintingGatewayActivateMintingAuthorityApproval(hashContext, {
      queueNonce: 4n,
      approvingCouncilHash: thirdNextCouncil.hash,
      previousUpdateHash: thirdRotationHash,
      target: activationTarget,
    });
    const firstRotationSignatures = await signApprovalHash(
      fixture.council.quorumSigners,
      firstRotationHash,
    );
    const secondRotationSignatures = await signApprovalHash(
      firstNextCouncil.quorumSigners,
      secondRotationHash,
    );
    const thirdRotationSignatures = await signApprovalHash(
      secondNextCouncil.quorumSigners,
      thirdRotationHash,
    );
    const activationSignatures = await signApprovalHash(
      thirdNextCouncil.quorumSigners,
      activationHash,
    );

    const activationEvent = await parseGatewayEvent(
      fixture.gateway.applyGatewayUpdates(
        fixture.council.snapshot,
        [
          {
            queueNonce: 1n,
            kind: MINTING_GATEWAY_UPDATE_KINDS.globalIssuanceCouncilRotate,
            payload: encodeMintingGatewayGlobalIssuanceCouncilRotateTarget(firstRotationTarget),
            signatures: firstRotationSignatures,
          },
          {
            queueNonce: 2n,
            kind: MINTING_GATEWAY_UPDATE_KINDS.globalIssuanceCouncilRotate,
            payload: encodeMintingGatewayGlobalIssuanceCouncilRotateTarget(secondRotationTarget),
            signatures: secondRotationSignatures,
          },
          {
            queueNonce: 3n,
            kind: MINTING_GATEWAY_UPDATE_KINDS.globalIssuanceCouncilRotate,
            payload: encodeMintingGatewayGlobalIssuanceCouncilRotateTarget(thirdRotationTarget),
            signatures: thirdRotationSignatures,
          },
          {
            queueNonce: 4n,
            kind: MINTING_GATEWAY_UPDATE_KINDS.mintingAuthorityActivate,
            payload: encodeMintingGatewayMintingAuthorityActivationTarget(activationTarget),
            signatures: activationSignatures,
          },
        ],
        fixture.relayerArgonAccountId,
      ),
      fixture.gateway,
      'MintingAuthorityActivated',
    );

    expect(activationEvent.args[0]).to.equal(activationTarget.signingKey);
    expect(activationEvent.args[3]).to.equal(1n);
    expect(activationEvent.args[4]).to.equal(
      BigInt(
        firstRotationSignatures.length +
          secondRotationSignatures.length +
          thirdRotationSignatures.length +
          activationSignatures.length,
      ),
    );
    expect(activationEvent.args[5]).to.equal(activationHash);
  });

  it('shares the realized signature cost across co-activations in the same signed head', async () => {
    const fixture = await deployFixture();
    const hashContext = await getGatewayHashContext(fixture.gateway);
    const firstTarget = {
      microgonCollateral: 1_000n,
      micronotCollateral: 100n,
      signingKey: Wallet.createRandom().address as `0x${string}`,
    } satisfies MintingGatewayMintingAuthorityActivationTarget;
    const secondTarget = {
      microgonCollateral: 2_000n,
      micronotCollateral: 150n,
      signingKey: Wallet.createRandom().address as `0x${string}`,
    } satisfies MintingGatewayMintingAuthorityActivationTarget;
    const firstActivationHash = hashMintingGatewayActivateMintingAuthorityApproval(hashContext, {
      queueNonce: 1n,
      approvingCouncilHash: fixture.council.hash,
      previousUpdateHash: ethers.ZeroHash as `0x${string}`,
      target: firstTarget,
    });
    const secondActivationHash = hashMintingGatewayActivateMintingAuthorityApproval(hashContext, {
      queueNonce: 2n,
      approvingCouncilHash: fixture.council.hash,
      previousUpdateHash: firstActivationHash,
      target: secondTarget,
    });
    const secondActivationSignatures = await signApprovalHash(
      fixture.council.quorumSigners,
      secondActivationHash,
    );
    const activationEvents = await parseGatewayEvents(
      fixture.gateway.applyGatewayUpdates(
        fixture.council.snapshot,
        [
          {
            queueNonce: 1n,
            kind: MINTING_GATEWAY_UPDATE_KINDS.mintingAuthorityActivate,
            payload: encodeMintingGatewayMintingAuthorityActivationTarget(firstTarget),
            signatures: [],
          },
          {
            queueNonce: 2n,
            kind: MINTING_GATEWAY_UPDATE_KINDS.mintingAuthorityActivate,
            payload: encodeMintingGatewayMintingAuthorityActivationTarget(secondTarget),
            signatures: secondActivationSignatures,
          },
        ],
        fixture.relayerArgonAccountId,
      ),
      fixture.gateway,
      'MintingAuthorityActivated',
    );

    expect(activationEvents).toHaveLength(2);
    expect(activationEvents[0].args[0]).to.equal(firstTarget.signingKey);
    expect(activationEvents[0].args[3]).to.equal(2n);
    expect(activationEvents[0].args[4]).to.equal(BigInt(secondActivationSignatures.length));
    expect(activationEvents[0].args[5]).to.equal(firstActivationHash);
    expect(activationEvents[1].args[0]).to.equal(secondTarget.signingKey);
    expect(activationEvents[1].args[3]).to.equal(2n);
    expect(activationEvents[1].args[4]).to.equal(BigInt(secondActivationSignatures.length));
    expect(activationEvents[1].args[5]).to.equal(secondActivationHash);
    expect((await fixture.gateway.activityBlockLocators(1n)).activityRoot).to.equal(
      appendMintingGatewayActivityRoot(
        appendMintingGatewayActivityRoot(
          ethers.ZeroHash as `0x${string}`,
          hashMintingGatewayMintingAuthorityActivatedActivity(hashContext, {
            signingKey: activationEvents[0].args[0] as `0x${string}`,
            microgonCollateral: activationEvents[0].args[1],
            micronotCollateral: activationEvents[0].args[2],
            coactivationCount: activationEvents[0].args[3],
            sharedSignatureCount: activationEvents[0].args[4],
            approvalHash: activationEvents[0].args[5] as `0x${string}`,
            relayerArgonAccountId: activationEvents[0].args[6] as `0x${string}`,
            gatewayState: asGatewayActivityState(activationEvents[0].args[7]),
          }),
        ),
        hashMintingGatewayMintingAuthorityActivatedActivity(hashContext, {
          signingKey: activationEvents[1].args[0] as `0x${string}`,
          microgonCollateral: activationEvents[1].args[1],
          micronotCollateral: activationEvents[1].args[2],
          coactivationCount: activationEvents[1].args[3],
          sharedSignatureCount: activationEvents[1].args[4],
          approvalHash: activationEvents[1].args[5] as `0x${string}`,
          relayerArgonAccountId: activationEvents[1].args[6] as `0x${string}`,
          gatewayState: asGatewayActivityState(activationEvents[1].args[7]),
        }),
      ),
    );
  });

  it('lets anyone finalize a transfer out of Argon to the signed recipient once authorizations are valid', async () => {
    const fixture = await deployFixture();
    const { argon, gateway, outsider, recipient } = fixture;
    const hashContext = await getGatewayHashContext(gateway);
    const { target } = await activateMintingAuthority(fixture);
    const request = await createTransferOutRequest(fixture, {
      recipient: recipient.address as `0x${string}`,
      token: (await argon.getAddress()) as `0x${string}`,
      amount: 50n,
    });
    const signature = await fixture.mintingAuthoritySigner.signMessage(
      ethers.getBytes(
        hashMintingGatewayMintingAuthorization(hashContext, {
          request,
          microgonCollateral: 80n,
          micronotCollateral: 0n,
        }),
      ),
    );
    const transferId = hashMintingGatewayTransferOutOfArgonRequest(request);

    const event = await parseGatewayEvent(
      gateway.connect(outsider).finalizeTransferOutOfArgon(request, {
        authorizations: [{ microgonCollateral: 80n, micronotCollateral: 0n, signature }],
      }),
      gateway,
      'TransferOutOfArgonFinalized',
    );

    const mintingCollateral = await gateway.mintingAuthorityCollateralRemaining(target.signingKey);
    expect(await argon.balanceOf(recipient.address)).to.equal(50n * SCALE);
    expect(await gateway.gatewayActivityNonce()).to.equal(2n);
    expect(await gateway.finalizedTransferOutOfArgonIds(transferId)).to.equal(true);
    expect(mintingCollateral.microgonCollateral).to.equal(920n);
    expect(mintingCollateral.micronotCollateral).to.equal(200n);

    expect(event.args[0]).to.equal(transferId);
    expect(event.args[1]).to.equal(await argon.getAddress());
    expect(event.args[2]).to.equal(50n);
    expect(event.args[3]).to.have.length(1);
    expect(event.args[3][0].signingKey).to.equal(target.signingKey);
    expect(event.args[3][0].microgonCollateral).to.equal(80n);
    expect(event.args[3][0].micronotCollateral).to.equal(0n);
    expect(event.args[4][0]).to.equal(2n);
    expect(event.args[4][1]).to.equal(1n);
    const activationLocator = await gateway.activityBlockLocators(1n);
    const locatorIndex = await gateway.latestActivityBlockLocatorIndex();
    const finalizationLocator = await gateway.activityBlockLocators(locatorIndex);
    const activationLocatorHash = hashMintingGatewayActivityBlockLocator({
      blockNumber: activationLocator.blockNumber,
      startGatewayActivityNonce: activationLocator.startGatewayActivityNonce,
      endGatewayActivityNonce: activationLocator.endGatewayActivityNonce,
      activityRoot: activationLocator.activityRoot as `0x${string}`,
    });
    expect(finalizationLocator.activityRoot).to.equal(
      appendMintingGatewayActivityRoot(
        activationLocatorHash,
        hashMintingGatewayTransferOutOfArgonFinalizedActivity(hashContext, {
          transferId,
          token: (await argon.getAddress()) as `0x${string}`,
          amount: 50n,
          mintingCollateral: [
            {
              signingKey: event.args[3][0].signingKey as `0x${string}`,
              microgonCollateral: event.args[3][0].microgonCollateral,
              micronotCollateral: event.args[3][0].micronotCollateral,
            },
          ],
          gatewayState: asGatewayActivityState(event.args[4]),
        }),
      ),
    );

    await expectCustomError(
      gateway.connect(outsider).finalizeTransferOutOfArgon(request, {
        authorizations: [{ microgonCollateral: 80n, micronotCollateral: 0n, signature }],
      }),
      gateway,
      'TransferOutOfArgonAlreadyFinalized',
      [transferId],
    );
  });

  it('rejects transfer-out proofs that exceed the authorization cap', async () => {
    const fixture = await deployFixture();
    const request = await createTransferOutRequest(fixture, { amount: 1n });

    await expectCustomError(
      fixture.gateway.connect(fixture.outsider).finalizeTransferOutOfArgon(request, {
        authorizations: Array.from({ length: 26 }, () => ({
          microgonCollateral: 1n,
          micronotCollateral: 0n,
          signature: '0x',
        })),
      }),
      fixture.gateway,
      'TooManyMintingAuthorizations',
      [25n, 26n],
    );
  });

  it('rejects zero-amount transfer-out finalization requests', async () => {
    const fixture = await deployFixture();
    await activateMintingAuthority(fixture);
    const request = await createTransferOutRequest(fixture, { amount: 0n });

    await expectCustomError(
      fixture.gateway.connect(fixture.outsider).finalizeTransferOutOfArgon(request, {
        authorizations: [],
      }),
      fixture.gateway,
      'ZeroAmount',
    );
  });

  it('accepts lower-quoted Argon payouts after the active floor increases', async () => {
    const fixture = await deployFixture();
    const signingWallet = Wallet.createRandom() as unknown as SignerLike;
    const { target } = await activateMintingAuthority(fixture, 1n, {
      microgonCollateral: 0n,
      micronotCollateral: 50n,
      signingKey: signingWallet.address as `0x${string}`,
    });
    const nextCouncil = createCouncil(
      [fixture.adminSafe, fixture.guardian, fixture.holder, fixture.outsider],
      2n * MICROGONS_PER_ARGONOT,
    );

    await rotateCouncil(fixture, 2n, nextCouncil);

    const request = await createTransferOutRequest(fixture, {
      microgonsPerArgonot: MICROGONS_PER_ARGONOT,
      amount: 50n,
    });
    const signature = await signingWallet.signMessage(
      ethers.getBytes(
        hashMintingGatewayMintingAuthorization(await getGatewayHashContext(fixture.gateway), {
          request,
          microgonCollateral: 0n,
          micronotCollateral: 50n,
        }),
      ),
    );

    await fixture.gateway.connect(fixture.outsider).finalizeTransferOutOfArgon(request, {
      authorizations: [{ microgonCollateral: 0n, micronotCollateral: 50n, signature }],
    });

    const mintingCollateral = await fixture.gateway.mintingAuthorityCollateralRemaining(
      target.signingKey,
    );
    expect(mintingCollateral.microgonCollateral).to.equal(0n);
    expect(mintingCollateral.micronotCollateral).to.equal(0n);
  });

  it('rejects transfer-out requests quoted above the active floor', async () => {
    const fixture = await deployFixture();
    const signingWallet = Wallet.createRandom() as unknown as SignerLike;
    await activateMintingAuthority(fixture, 1n, {
      microgonCollateral: 0n,
      micronotCollateral: 50n,
      signingKey: signingWallet.address as `0x${string}`,
    });
    const nextCouncil = createCouncil(
      [fixture.adminSafe, fixture.guardian, fixture.holder, fixture.outsider],
      500_000n,
    );

    await rotateCouncil(fixture, 2n, nextCouncil);

    const request = await createTransferOutRequest(fixture, {
      microgonsPerArgonot: MICROGONS_PER_ARGONOT,
      amount: 50n,
    });
    const signature = await signingWallet.signMessage(
      ethers.getBytes(
        hashMintingGatewayMintingAuthorization(await getGatewayHashContext(fixture.gateway), {
          request,
          microgonCollateral: 0n,
          micronotCollateral: 50n,
        }),
      ),
    );

    await expectCustomError(
      fixture.gateway.connect(fixture.outsider).finalizeTransferOutOfArgon(request, {
        authorizations: [{ microgonCollateral: 0n, micronotCollateral: 50n, signature }],
      }),
      fixture.gateway,
      'InvalidTransferOutRate',
      [500_000n, MICROGONS_PER_ARGONOT],
    );
  });

  it('allows mixed Argon and Argonot collateral on Argon payouts', async () => {
    const fixture = await deployFixture();
    await activateMintingAuthority(fixture);
    const request = await createTransferOutRequest(fixture, { amount: 50n });
    const signature = await fixture.mintingAuthoritySigner.signMessage(
      ethers.getBytes(
        hashMintingGatewayMintingAuthorization(await getGatewayHashContext(fixture.gateway), {
          request,
          microgonCollateral: 25n,
          micronotCollateral: 25n,
        }),
      ),
    );

    await fixture.gateway.connect(fixture.outsider).finalizeTransferOutOfArgon(request, {
      authorizations: [{ microgonCollateral: 25n, micronotCollateral: 25n, signature }],
    });
  });

  it('does not let Argon collateral pay for Argonot payouts', async () => {
    const fixture = await deployFixture();
    await activateMintingAuthority(fixture);
    const request = await createTransferOutRequest(fixture, {
      token: (await fixture.argonot.getAddress()) as `0x${string}`,
      amount: 50n,
    });
    const signature = await fixture.mintingAuthoritySigner.signMessage(
      ethers.getBytes(
        hashMintingGatewayMintingAuthorization(await getGatewayHashContext(fixture.gateway), {
          request,
          microgonCollateral: 1n,
          micronotCollateral: 50n,
        }),
      ),
    );

    await expectCustomError(
      fixture.gateway.connect(fixture.outsider).finalizeTransferOutOfArgon(request, {
        authorizations: [{ microgonCollateral: 1n, micronotCollateral: 50n, signature }],
      }),
      fixture.gateway,
      'InvalidMicrogonCollateralForArgonotPayout',
    );
  });

  it('lets anyone cancel a transfer out of Argon after expiry and marks the transfer id finalized', async () => {
    const fixture = await deployFixture();
    const currentBlock = BigInt(await ethers.provider.getBlockNumber());
    const notExpiredRequest = await createTransferOutRequest(fixture, {
      argonAccountId: ethers.encodeBytes32String('account-2') as `0x${string}`,
      argonTransferNonce: 7n,
      validUntilBlock: currentBlock + 100n,
      amount: 25n,
      mintingAuthorityTip: 1n,
    });
    const expiredRequest = {
      ...notExpiredRequest,
      argonTransferNonce: 8n,
      validUntilBlock: 0n,
    } satisfies MintingGatewayTransferOutOfArgonRequest;
    const expiredTransferId = hashMintingGatewayTransferOutOfArgonRequest(expiredRequest);

    await expectCustomError(
      fixture.gateway.connect(fixture.outsider).cancelTransferOutOfArgon(notExpiredRequest),
      fixture.gateway,
      'TransferOutOfArgonNotExpired',
      [currentBlock + 1n, currentBlock + 100n],
    );

    const event = await parseGatewayEvent(
      fixture.gateway.connect(fixture.outsider).cancelTransferOutOfArgon(expiredRequest),
      fixture.gateway,
      'TransferOutOfArgonCanceled',
    );

    expect(await fixture.gateway.gatewayActivityNonce()).to.equal(1n);
    expect(await fixture.gateway.finalizedTransferOutOfArgonIds(expiredTransferId)).to.equal(true);
    expect(event.args[0]).to.equal(expiredTransferId);
    expect(event.args[1][0]).to.equal(1n);
    expect(event.args[1][1]).to.equal(0n);

    await expectCustomError(
      fixture.gateway.connect(fixture.outsider).cancelTransferOutOfArgon(expiredRequest),
      fixture.gateway,
      'TransferOutOfArgonAlreadyFinalized',
      [expiredTransferId],
    );
  });

  it('rejects zero-amount transfer-out cancellations', async () => {
    const fixture = await deployFixture();
    const expiredRequest = await createTransferOutRequest(fixture, {
      validUntilBlock: 0n,
      amount: 0n,
      mintingAuthorityTip: 0n,
    });

    await expectCustomError(
      fixture.gateway.connect(fixture.outsider).cancelTransferOutOfArgon(expiredRequest),
      fixture.gateway,
      'ZeroAmount',
    );
  });
});
