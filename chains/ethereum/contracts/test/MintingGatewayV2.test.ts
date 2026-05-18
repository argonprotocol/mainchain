import { network } from 'hardhat';
import { afterAll, describe, expect, it } from 'vitest';
import { Wallet } from 'ethers';
import { expectCustomError } from './assertions.js';

const SCALE = 1_000_000_000_000n;
const ERC1967_ADMIN_SLOT = '0xb53127684a568b3173ae13b9f8a6016e243e63b6e8ee1178d6a717850b5d6103';
const COUNCIL_WEIGHTS = [40n, 30n, 20n, 10n] as const;

const connection = await network.create();
const { ethers } = connection;
const abiCoder = ethers.AbiCoder.defaultAbiCoder();
const MINTING_AUTHORITY_ACTIVATION_TAG = ethers.id('ARGON_MINTING_AUTHORITY_ACTIVATION');
const MINTING_AUTHORITY_DEACTIVATION_TAG = ethers.id('ARGON_MINTING_AUTHORITY_DEACTIVATION');
const GATEWAY_UPDATE_APPROVAL_TAG = ethers.id('ARGON_GATEWAY_UPDATE');
const TRANSFER_OUT_OF_ARGON_AUTHORIZATION_TAG =
  ethers.id('ARGON_TRANSFER_OUT_OF_ARGON_AUTHORIZATION');

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
      verifyingContract: string;
    },
    types: {
      Permit: Array<{ name: string; type: string }>;
    },
    value: {
      owner: string;
      spender: string;
      value: bigint;
      nonce: bigint;
      deadline: bigint;
    },
  ): Promise<string>;
};

afterAll(async () => {
  await connection.close();
});

describe('MintingGatewayV2', () => {
  function hashGlobalIssuanceCouncil(signers: string[], weights: bigint[]) {
    return ethers.keccak256(abiCoder.encode(['address[]', 'uint256[]'], [signers, weights]));
  }

  function hashMintingAuthority(
    mintingAuthorityId: string,
    microgonCollateral: bigint,
    micronotCollateral: bigint,
    signingKey: string,
  ) {
    return ethers.keccak256(
      abiCoder.encode(
        ['bytes32', 'uint64', 'uint64', 'address'],
        [mintingAuthorityId, microgonCollateral, micronotCollateral, signingKey],
      ),
    );
  }

  async function hashActivateMintingAuthorityApproval(
    gateway: { getAddress(): Promise<string> },
    queueNonce: bigint,
    approvingCouncilHash: string,
    previousUpdateHash: string,
    target: {
      mintingAuthorityId: string;
      microgonCollateral: bigint;
      micronotCollateral: bigint;
      signingKey: string;
    },
  ) {
    const { chainId } = await ethers.provider.getNetwork();
    const gatewayAddress = await gateway.getAddress();
    const activationHash = ethers.keccak256(
      abiCoder.encode(
        ['bytes32', 'uint256', 'address', 'bytes32'],
        [
          MINTING_AUTHORITY_ACTIVATION_TAG,
          chainId,
          gatewayAddress,
          hashMintingAuthority(
            target.mintingAuthorityId,
            target.microgonCollateral,
            target.micronotCollateral,
            target.signingKey,
          ),
        ],
      ),
    );

    return ethers.keccak256(
      abiCoder.encode(
        ['bytes32', 'uint256', 'address', 'uint64', 'bytes32', 'uint8', 'bytes32', 'bytes32', 'bytes32'],
        [
          GATEWAY_UPDATE_APPROVAL_TAG,
          chainId,
          gatewayAddress,
          queueNonce,
          approvingCouncilHash,
          1,
          target.mintingAuthorityId,
          activationHash,
          previousUpdateHash,
        ],
      ),
    );
  }

  async function hashRotateGlobalIssuanceCouncilApproval(
    gateway: { getAddress(): Promise<string> },
    queueNonce: bigint,
    approvingCouncilHash: string,
    previousUpdateHash: string,
    nextSigners: string[],
    nextWeights: bigint[],
  ) {
    const { chainId } = await ethers.provider.getNetwork();
    const gatewayAddress = await gateway.getAddress();
    const nextCouncilHash = hashGlobalIssuanceCouncil(nextSigners, nextWeights);
    const rotationHash = ethers.keccak256(
      abiCoder.encode(
        ['bytes32', 'uint256', 'address', 'bytes32'],
        [
          ethers.id('ARGON_GLOBAL_ISSUANCE_COUNCIL_ROTATION'),
          chainId,
          gatewayAddress,
          nextCouncilHash,
        ],
      ),
    );

    return ethers.keccak256(
      abiCoder.encode(
        ['bytes32', 'uint256', 'address', 'uint64', 'bytes32', 'uint8', 'bytes32', 'bytes32', 'bytes32'],
        [
          GATEWAY_UPDATE_APPROVAL_TAG,
          chainId,
          gatewayAddress,
          queueNonce,
          approvingCouncilHash,
          0,
          nextCouncilHash,
          rotationHash,
          previousUpdateHash,
        ],
      ),
    );
  }

  async function hashMintingAuthorityDeactivation(
    gateway: { getAddress(): Promise<string> },
    queueNonce: bigint,
    target: { mintingAuthorityId: string; signingKey: string },
    previousUpdateHash: string,
  ) {
    const { chainId } = await ethers.provider.getNetwork();
    const gatewayAddress = await gateway.getAddress();
    return ethers.keccak256(
      abiCoder.encode(
        ['bytes32', 'uint256', 'address', 'uint64', 'bytes32', 'address', 'bytes32'],
        [
          MINTING_AUTHORITY_DEACTIVATION_TAG,
          chainId,
          gatewayAddress,
          queueNonce,
          target.mintingAuthorityId,
          target.signingKey,
          previousUpdateHash,
        ],
      ),
    );
  }

  function hashTransferOutOfArgonRequest(request: readonly unknown[]) {
    return ethers.keccak256(
      abiCoder.encode(
        [
          'bytes32',
          'uint64',
          'uint64',
          'address',
          'uint64',
          'address',
          'uint64',
          'uint256',
          'uint64',
          'uint64',
        ],
        request,
      ),
    );
  }

  async function hashMintingAuthorization(
    gateway: { getAddress(): Promise<string> },
    request: readonly unknown[],
    microgonCollateral: bigint,
    micronotCollateral: bigint,
  ) {
    const { chainId } = await ethers.provider.getNetwork();

    return ethers.keccak256(
      abiCoder.encode(
        ['bytes32', 'uint256', 'address', 'bytes32', 'uint64', 'uint64'],
        [
          TRANSFER_OUT_OF_ARGON_AUTHORIZATION_TAG,
          chainId,
          await gateway.getAddress(),
          hashTransferOutOfArgonRequest(request),
          microgonCollateral,
          micronotCollateral,
        ],
      ),
    );
  }

  async function signPermit(
    signer: TypedDataSignerLike,
    token: { getAddress(): Promise<string>; getFunction(name: 'nonces'): (owner: string) => Promise<bigint> },
    owner: string,
    spender: string,
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
        verifyingContract: await token.getAddress(),
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

  function createCouncil(signers: SignerLike[]) {
    const wallets = [...signers].sort((left, right) =>
      left.address.toLowerCase().localeCompare(right.address.toLowerCase()),
    );
    const sortedSigners = wallets.map(wallet => wallet.address);
    const weights = wallets.map((_, index) => COUNCIL_WEIGHTS[index] ?? 10n);

    return {
      wallets,
      signers: sortedSigners,
      weights,
      memberCount: BigInt(sortedSigners.length),
      totalWeight: weights.reduce((sum, weight) => sum + weight, 0n),
      hash: hashGlobalIssuanceCouncil(sortedSigners, weights),
      snapshot: [sortedSigners, weights] as const,
    };
  }

  async function signApprovalHash(signers: SignerLike[], approvalHash: string) {
    const sortedSigners = [...signers].sort((left, right) =>
      left.address.toLowerCase().localeCompare(right.address.toLowerCase()),
    );

    return Promise.all(
      sortedSigners.map(signer => signer.signMessage(ethers.getBytes(approvalHash))),
    );
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

  async function deployGatewayV2Stack() {
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

    const gatewayFactory = await ethers.getContractFactory('MintingGatewayV2');
    const gatewayBootstrapImplementation = (await gatewayFactory.deploy(
      ethers.ZeroAddress,
      ethers.ZeroAddress,
    )) as any;
    await gatewayBootstrapImplementation.waitForDeployment();

    const council = createCouncil([councilA, councilB, councilC, councilD]);

    const initializeData = gatewayFactory.interface.encodeFunctionData('initialize', [
      adminSafe.address,
      guardian.address,
      council.hash,
      council.memberCount,
      council.totalWeight,
    ]);

    const gatewayProxyFactory = await ethers.getContractFactory('TransparentUpgradeableProxy');
    const gatewayProxy = (await gatewayProxyFactory.deploy(
      await gatewayBootstrapImplementation.getAddress(),
      adminSafe.address,
      initializeData,
    )) as any;
    await gatewayProxy.waitForDeployment();

    const gateway = (await ethers.getContractAt(
      'MintingGatewayV2',
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
      mintingAuthoritySigner,
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
    const stack = await deployGatewayV2Stack();

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
        [[stack.holder.address], [1_000n * SCALE]],
        [[stack.holder.address], [2_000n * SCALE]],
      );

    return stack;
  }

  async function activateMintingAuthority(fixture: Awaited<ReturnType<typeof deployFixture>>) {
    const mintingAuthorityId = ethers.id('minting-authority-1');
    const target = {
      mintingAuthorityId,
      microgonCollateral: 1_000n,
      micronotCollateral: 200n,
      signingKey: fixture.mintingAuthoritySigner.address,
    };
    const previousUpdateHash =
      await fixture.gateway.getFunction('argonApprovalsHash')();
    const approvalHash = await hashActivateMintingAuthorityApproval(
      fixture.gateway,
      1n,
      fixture.council.hash,
      previousUpdateHash,
      target,
    );
    const signatures = await signApprovalHash(
      [fixture.council.wallets[0], fixture.council.wallets[1], fixture.council.wallets[2]],
      approvalHash,
    );
    const payload = abiCoder.encode(
      [
        'tuple(bytes32 mintingAuthorityId,uint64 microgonCollateral,uint64 micronotCollateral,address signingKey)',
      ],
      [[
        target.mintingAuthorityId,
        target.microgonCollateral,
        target.micronotCollateral,
        target.signingKey,
      ]],
    );

    const event = await parseGatewayEvent(
      fixture.gateway.applyGatewayUpdates(
        fixture.council.snapshot,
        [
          [1n, 1, payload, signatures],
        ],
        fixture.relayerArgonAccountId,
      ),
      fixture.gateway,
      'MintingAuthorityActivated',
    );

    return { event, mintingAuthorityId, target };
  }

  it('starts a transfer to Argon in the ordered gateway activity stream', async () => {
    const { argon, argonot, gateway, holder } = await deployFixture();
    const argonAccountId = ethers.encodeBytes32String('argon-account-1');
    const deadline = BigInt((await ethers.provider.getBlock('latest'))!.timestamp) + 3600n;
    const permit = await signPermit(
      holder as TypedDataSignerLike,
      argon,
      holder.address,
      await gateway.getAddress(),
      250n * SCALE,
      deadline,
      'Argon',
    );

    const event = await parseGatewayEvent(
      gateway.connect(holder).startTransferToArgon(
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

    expect(await argon.balanceOf(holder.address)).to.equal(750n * SCALE);
    expect(await argon.allowance(holder.address, await gateway.getAddress())).to.equal(0n);
    expect(await argonot.balanceOf(holder.address)).to.equal(2_000n * SCALE);
  });

  it('rejects zero runtime-unit transfer amounts', async () => {
    const { gateway, holder } = await deployFixture();
    const argonAccountId = ethers.encodeBytes32String('argon-account-1');

    await expectCustomError(
      gateway.connect(holder).startTransferToArgon(
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
    const nextCouncil = createCouncil([
      Wallet.createRandom(),
      Wallet.createRandom(),
      Wallet.createRandom(),
      Wallet.createRandom(),
    ]);

    await expectCustomError(
      fixture.gateway.connect(fixture.outsider).forceUpdateActiveCouncil(nextCouncil.snapshot),
      fixture.gateway,
      'OwnableUnauthorizedAccount',
      [fixture.outsider.address],
    );

    const event = await parseGatewayEvent(
      fixture.gateway.connect(fixture.adminSafe).forceUpdateActiveCouncil(nextCouncil.snapshot),
      fixture.gateway,
      'GlobalIssuanceCouncilForceUpdated',
    );
    const activeCouncil = await fixture.gateway.globalIssuanceCouncil();

    expect(event.args[0]).to.equal(fixture.council.hash);
    expect(event.args[1]).to.equal(nextCouncil.hash);
    expect(activeCouncil.councilHash).to.equal(nextCouncil.hash);
    expect(activeCouncil.memberCount).to.equal(nextCouncil.memberCount);
    expect(activeCouncil.totalWeight).to.equal(nextCouncil.totalWeight);
    expect(await fixture.gateway.argonApprovalsNonce()).to.equal(0n);
    expect(await fixture.gateway.getFunction('argonApprovalsHash')()).to.equal(ethers.ZeroHash);
  });

  it('continues queue processing from the same approvals hash after a forced council update', async () => {
    const fixture = await deployFixture();
    const nextCouncil = createCouncil([
      Wallet.createRandom(),
      Wallet.createRandom(),
      Wallet.createRandom(),
      Wallet.createRandom(),
    ]);
    const target = {
      mintingAuthorityId: ethers.id('minting-authority-force-update'),
      microgonCollateral: 1_500n,
      micronotCollateral: 250n,
      signingKey: Wallet.createRandom().address,
    };

    await fixture.gateway.connect(fixture.adminSafe).forceUpdateActiveCouncil(nextCouncil.snapshot);

    const approvalHash = await hashActivateMintingAuthorityApproval(
      fixture.gateway,
      1n,
      nextCouncil.hash,
      await fixture.gateway.getFunction('argonApprovalsHash')(),
      target,
    );
    const signatures = await signApprovalHash(
      [nextCouncil.wallets[0], nextCouncil.wallets[1], nextCouncil.wallets[2]],
      approvalHash,
    );
    const payload = abiCoder.encode(
      [
        'tuple(bytes32 mintingAuthorityId,uint64 microgonCollateral,uint64 micronotCollateral,address signingKey)',
      ],
      [[
        target.mintingAuthorityId,
        target.microgonCollateral,
        target.micronotCollateral,
        target.signingKey,
      ]],
    );

    await fixture.gateway.applyGatewayUpdates(
      nextCouncil.snapshot,
      [
        [1n, 1, payload, signatures],
      ],
      fixture.relayerArgonAccountId,
    );

    const mintingCollateral =
      await fixture.gateway.mintingAuthorityCollateralRemaining(target.signingKey);
    expect(mintingCollateral.microgonCollateral).to.equal(target.microgonCollateral);
    expect(mintingCollateral.micronotCollateral).to.equal(target.micronotCollateral);
    expect(await fixture.gateway.argonApprovalsNonce()).to.equal(1n);
    expect(await fixture.gateway.getFunction('argonApprovalsHash')()).to.equal(approvalHash);
  });

  it('applies queued minting authority activation and deactivation updates in order', async () => {
    const fixture = await deployFixture();
    const { gateway, council, relayerArgonAccountId } = fixture;
    const { event: activationEvent, mintingAuthorityId, target } =
      await activateMintingAuthority(fixture);

    expect(activationEvent.args[0]).to.equal(mintingAuthorityId);
    expect(activationEvent.args[4]).to.equal(relayerArgonAccountId);
    expect(activationEvent.args[5][0]).to.equal(1n);
    expect(activationEvent.args[5][1]).to.equal(1n);

    const deactivationHash = await hashMintingAuthorityDeactivation(
      gateway,
      2n,
      { mintingAuthorityId, signingKey: target.signingKey },
      await gateway.getFunction('argonApprovalsHash')(),
    );
    const wrongDeactivateSignature = await council.wallets[0].signMessage(
      ethers.getBytes(deactivationHash),
    );
    const deactivatePayload =
      abiCoder.encode(
        ['tuple(bytes32 mintingAuthorityId,address signingKey)'],
        [{ mintingAuthorityId, signingKey: target.signingKey }],
      );

    await expectCustomError(
      gateway.applyGatewayUpdates(
        fixture.council.snapshot,
        [
          [2n, 2, deactivatePayload, [wrongDeactivateSignature]],
        ],
        relayerArgonAccountId,
      ),
      gateway,
      'InvalidMintingAuthorityDeactivationSigner',
      [target.signingKey, council.wallets[0].address],
    );

    const deactivateSignature = await fixture.mintingAuthoritySigner.signMessage(
      ethers.getBytes(deactivationHash),
    );
    const deactivateEvent = await parseGatewayEvent(
      gateway.applyGatewayUpdates(
        fixture.council.snapshot,
        [
          [2n, 2, deactivatePayload, [deactivateSignature]],
        ],
        relayerArgonAccountId,
      ),
      gateway,
      'MintingAuthorityDeactivated',
    );

    const mintingCollateral =
      await gateway.mintingAuthorityCollateralRemaining(target.signingKey);
    expect(mintingCollateral.microgonCollateral).to.equal(0n);
    expect(mintingCollateral.micronotCollateral).to.equal(0n);
    expect(await gateway.argonApprovalsNonce()).to.equal(2n);
    expect(await gateway.gatewayActivityNonce()).to.equal(2n);

    expect(deactivateEvent.args[0]).to.equal(mintingAuthorityId);
    expect(deactivateEvent.args[1]).to.equal(target.microgonCollateral);
    expect(deactivateEvent.args[2]).to.equal(target.micronotCollateral);
    expect(deactivateEvent.args[3]).to.equal(relayerArgonAccountId);
    expect(deactivateEvent.args[4][0]).to.equal(2n);
    expect(deactivateEvent.args[4][1]).to.equal(2n);
  });

  it('only needs council signatures on rotation items and the last council-approved item in a batch', async () => {
    const fixture = await deployFixture();
    const nextCouncil = createCouncil([
      Wallet.createRandom(),
      Wallet.createRandom(),
      Wallet.createRandom(),
      Wallet.createRandom(),
    ]);
    const firstTarget = {
      mintingAuthorityId: ethers.id('minting-authority-batch-1'),
      microgonCollateral: 1_000n,
      micronotCollateral: 100n,
      signingKey: Wallet.createRandom().address,
    };
    const secondTarget = {
      mintingAuthorityId: ethers.id('minting-authority-batch-2'),
      microgonCollateral: 2_000n,
      micronotCollateral: 150n,
      signingKey: Wallet.createRandom().address,
    };
    const rotationPayload = abiCoder.encode(
      ['tuple(address[] signers,uint256[] weights)'],
      [[nextCouncil.signers, nextCouncil.weights]],
    );
    const firstPayload = abiCoder.encode(
      [
        'tuple(bytes32 mintingAuthorityId,uint64 microgonCollateral,uint64 micronotCollateral,address signingKey)',
      ],
      [[
        firstTarget.mintingAuthorityId,
        firstTarget.microgonCollateral,
        firstTarget.micronotCollateral,
        firstTarget.signingKey,
      ]],
    );
    const secondPayload = abiCoder.encode(
      [
        'tuple(bytes32 mintingAuthorityId,uint64 microgonCollateral,uint64 micronotCollateral,address signingKey)',
      ],
      [[
        secondTarget.mintingAuthorityId,
        secondTarget.microgonCollateral,
        secondTarget.micronotCollateral,
        secondTarget.signingKey,
      ]],
    );
    const activationOneHash = await hashActivateMintingAuthorityApproval(
      fixture.gateway,
      1n,
      fixture.council.hash,
      ethers.ZeroHash,
      firstTarget,
    );
    const rotationHash = await hashRotateGlobalIssuanceCouncilApproval(
      fixture.gateway,
      2n,
      fixture.council.hash,
      activationOneHash,
      nextCouncil.signers,
      nextCouncil.weights,
    );
    const activationTwoHash = await hashActivateMintingAuthorityApproval(
      fixture.gateway,
      3n,
      nextCouncil.hash,
      rotationHash,
      secondTarget,
    );
    const rotationSignatures = await signApprovalHash(
      [fixture.council.wallets[0], fixture.council.wallets[1], fixture.council.wallets[2]],
      rotationHash,
    );
    const activationTwoSignatures = await signApprovalHash(
      [nextCouncil.wallets[0], nextCouncil.wallets[1], nextCouncil.wallets[2]],
      activationTwoHash,
    );

    await fixture.gateway.applyGatewayUpdates(
      fixture.council.snapshot,
      [
        [1n, 1, firstPayload, []],
        [2n, 0, rotationPayload, rotationSignatures],
        [3n, 1, secondPayload, activationTwoSignatures],
      ],
      fixture.relayerArgonAccountId,
    );

    const firstMintingCollateral = await fixture.gateway.mintingAuthorityCollateralRemaining(
      firstTarget.signingKey,
    );
    const secondMintingCollateral = await fixture.gateway.mintingAuthorityCollateralRemaining(
      secondTarget.signingKey,
    );

    expect(firstMintingCollateral.microgonCollateral).to.equal(firstTarget.microgonCollateral);
    expect(secondMintingCollateral.microgonCollateral).to.equal(secondTarget.microgonCollateral);
    expect(await fixture.gateway.argonApprovalsNonce()).to.equal(3n);
    expect(
      await fixture.gateway.getFunction('argonApprovalsHash')(),
    ).to.equal(activationTwoHash);

    const activeCouncil = await fixture.gateway.globalIssuanceCouncil();
    expect(activeCouncil.councilHash).to.equal(nextCouncil.hash);
    expect(activeCouncil.memberCount).to.equal(nextCouncil.memberCount);
    expect(activeCouncil.totalWeight).to.equal(nextCouncil.totalWeight);
  });

  it('lets anyone finalize a transfer out of Argon to the signed recipient once authorizations are valid', async () => {
    const fixture = await deployFixture();
    const { argon, gateway, mintingAuthoritySigner, outsider, recipient } = fixture;
    const { target } = await activateMintingAuthority(fixture);
    const chainId = BigInt((await ethers.provider.getNetwork()).chainId);
    const request = [
      ethers.encodeBytes32String('account-1'),
      1n,
      chainId,
      recipient.address,
      1_000_000n,
      await argon.getAddress(),
      50n,
      0n,
      80n,
      5n,
    ] as const;
    const mintingAuthorizationHash = await hashMintingAuthorization(
      gateway,
      request,
      80n,
      0n,
    );
    const signature = await mintingAuthoritySigner.signMessage(
      ethers.getBytes(mintingAuthorizationHash),
    );
    const transferId = hashTransferOutOfArgonRequest(request);

    const event = await parseGatewayEvent(
      gateway.connect(outsider).finalizeTransferOutOfArgon(request, [
        [[80n, 0n, signature]],
      ]),
      gateway,
      'TransferOutOfArgonFinalized',
    );

    const mintingCollateral =
      await gateway.mintingAuthorityCollateralRemaining(target.signingKey);
    expect(await argon.balanceOf(recipient.address)).to.equal(50n * SCALE);
    expect(await gateway.gatewayActivityNonce()).to.equal(2n);
    expect(
      await gateway.getFunction('finalizedTransferOutOfArgonIds')(transferId),
    ).to.equal(true);
    expect(mintingCollateral.microgonCollateral).to.equal(920n);
    expect(mintingCollateral.micronotCollateral).to.equal(200n);

    expect(event.args[0]).to.equal(transferId);
    expect(event.args[1]).to.have.length(1);
    expect(event.args[1][0].signingKey).to.equal(target.signingKey);
    expect(event.args[1][0].microgonCollateral).to.equal(80n);
    expect(event.args[1][0].micronotCollateral).to.equal(0n);
    expect(event.args[2][0]).to.equal(2n);
    expect(event.args[2][1]).to.equal(1n);

    await expectCustomError(
      gateway.connect(outsider).finalizeTransferOutOfArgon(request, [
        [[80n, 0n, signature]],
      ]),
      gateway,
      'TransferOutOfArgonAlreadyFinalized',
      [transferId],
    );
  });

  it('lets anyone cancel a transfer out of Argon after expiry and marks the transfer id finalized', async () => {
    const { argon, gateway, outsider, recipient } = await deployFixture();
    const chainId = BigInt((await ethers.provider.getNetwork()).chainId);
    const currentBlock = BigInt(await ethers.provider.getBlockNumber());
    const notExpiredRequest = [
      ethers.encodeBytes32String('account-2'),
      7n,
      chainId,
      recipient.address,
      currentBlock + 100n,
      await argon.getAddress(),
      25n,
      0n,
      25n,
      1n,
    ] as const;
    const expiredRequest = [
      notExpiredRequest[0],
      8n,
      notExpiredRequest[2],
      notExpiredRequest[3],
      0n,
      notExpiredRequest[5],
      notExpiredRequest[6],
      notExpiredRequest[7],
      notExpiredRequest[8],
      notExpiredRequest[9],
    ] as const;
    const expiredTransferId = hashTransferOutOfArgonRequest(expiredRequest);

    await expectCustomError(
      gateway.connect(outsider).cancelTransferOutOfArgon(notExpiredRequest),
      gateway,
      'TransferOutOfArgonNotExpired',
      [currentBlock + 1n, currentBlock + 100n],
    );

    const event = await parseGatewayEvent(
      gateway.connect(outsider).cancelTransferOutOfArgon(expiredRequest),
      gateway,
      'TransferOutOfArgonCanceled',
    );

    expect(await gateway.gatewayActivityNonce()).to.equal(1n);
    expect(
      await gateway.getFunction('finalizedTransferOutOfArgonIds')(expiredTransferId),
    ).to.equal(true);
    expect(event.args[0]).to.equal(expiredTransferId);
    expect(event.args[1][0]).to.equal(1n);
    expect(event.args[1][1]).to.equal(0n);

    await expectCustomError(
      gateway.connect(outsider).cancelTransferOutOfArgon(expiredRequest),
      gateway,
      'TransferOutOfArgonAlreadyFinalized',
      [expiredTransferId],
    );
  });
});
