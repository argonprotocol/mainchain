import { network } from 'hardhat';
import { afterAll, describe, expect, it } from 'vitest';
import { Wallet } from 'ethers';
import {
  encodeMintingGatewayV2GlobalIssuanceCouncilRotateTarget,
  encodeMintingGatewayV2MintingAuthorityActivationTarget,
  encodeMintingGatewayV2MintingAuthorityDeactivateTarget,
  hashMintingGatewayV2ActivateMintingAuthorityApproval,
  hashMintingGatewayV2GlobalIssuanceCouncil,
  hashMintingGatewayV2MintingAuthorityDeactivation,
  hashMintingGatewayV2MintingAuthorization,
  hashMintingGatewayV2RotateGlobalIssuanceCouncilApproval,
  hashMintingGatewayV2TransferOutOfArgonRequest,
  MINTING_GATEWAY_V2_UPDATE_KINDS,
  type MintingGatewayV2CouncilSnapshot,
  type MintingGatewayV2GlobalIssuanceCouncilRotateTarget,
  type MintingGatewayV2HashContext,
  type MintingGatewayV2MintingAuthorityActivationTarget,
  type MintingGatewayV2MintingAuthorityDeactivateTarget,
  type MintingGatewayV2TransferOutOfArgonRequest,
} from '../index.js';
import { expectCustomError } from './assertions.js';

const SCALE = 1_000_000_000_000n;
const MICROGONS_PER_ARGONOT = 1_000_000n;
const ERC1967_ADMIN_SLOT = '0xb53127684a568b3173ae13b9f8a6016e243e63b6e8ee1178d6a717850b5d6103';
const COUNCIL_WEIGHTS = [40n, 30n, 20n, 10n] as const;

const connection = await network.create();
const { ethers } = connection;

type SignerLike = {
  address: `0x${string}`;
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
  microgonsPerArgonot: bigint;
  snapshot: MintingGatewayV2CouncilSnapshot;
  quorumSigners: SignerLike[];
};

afterAll(async () => {
  await connection.close();
});

describe('MintingGatewayV2', () => {
  async function getGatewayHashContext(gateway: {
    getAddress(): Promise<string>;
  }): Promise<MintingGatewayV2HashContext> {
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
    microgonsPerArgonot = MICROGONS_PER_ARGONOT,
  ): Council {
    const wallets = [...signers].sort((left, right) =>
      left.address.toLowerCase().localeCompare(right.address.toLowerCase()),
    );
    const sortedSigners = wallets.map(wallet => wallet.address);
    const weights = wallets.map((_, index) => COUNCIL_WEIGHTS[index] ?? 10n);
    const snapshot = {
      signers: sortedSigners,
      weights,
    } satisfies MintingGatewayV2CouncilSnapshot;

    return {
      wallets,
      signers: sortedSigners,
      weights,
      memberCount: BigInt(sortedSigners.length),
      totalWeight: weights.reduce((sum, weight) => sum + weight, 0n),
      hash: hashMintingGatewayV2GlobalIssuanceCouncil(snapshot),
      microgonsPerArgonot,
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
      council.microgonsPerArgonot,
    ]);

    const gatewayProxyFactory = await ethers.getContractFactory('TransparentUpgradeableProxy');
    const gatewayProxy = await gatewayProxyFactory.deploy(
      await gatewayBootstrapImplementation.getAddress(),
      adminSafe.address,
      initializeData,
    );
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
        { recipients: [stack.holder.address], amounts: [1_000n * SCALE] },
        { recipients: [stack.holder.address], amounts: [2_000n * SCALE] },
      );

    return stack;
  }

  async function activateMintingAuthority(
    fixture: Awaited<ReturnType<typeof deployFixture>>,
    queueNonce = 1n,
    overrides: Partial<MintingGatewayV2MintingAuthorityActivationTarget> = {},
  ) {
    const target = {
      mintingAuthorityId:
        overrides.mintingAuthorityId ?? (ethers.id(`minting-authority-${queueNonce}`) as `0x${string}`),
      microgonCollateral: overrides.microgonCollateral ?? 1_000n,
      micronotCollateral: overrides.micronotCollateral ?? 200n,
      signingKey: overrides.signingKey ?? fixture.mintingAuthoritySigner.address,
    } satisfies MintingGatewayV2MintingAuthorityActivationTarget;
    const previousUpdateHash = await fixture.gateway.getFunction('argonApprovalsHash')();
    const approvalHash = hashMintingGatewayV2ActivateMintingAuthorityApproval(
      await getGatewayHashContext(fixture.gateway),
      {
        queueNonce,
        approvingCouncilHash: fixture.council.hash,
        previousUpdateHash,
        target,
      },
    );
    const signatures = await signApprovalHash(fixture.council.quorumSigners, approvalHash);
    const payload = encodeMintingGatewayV2MintingAuthorityActivationTarget(target);

    const event = await parseGatewayEvent(
      fixture.gateway.applyGatewayUpdates(
        fixture.council.snapshot,
        [
          {
            queueNonce,
            kind: MINTING_GATEWAY_V2_UPDATE_KINDS.mintingAuthorityActivate,
            payload,
            signatures,
          },
        ],
        fixture.relayerArgonAccountId,
      ),
      fixture.gateway,
      'MintingAuthorityActivated',
    );

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
      microgonsPerArgonot: nextCouncil.microgonsPerArgonot,
    } satisfies MintingGatewayV2GlobalIssuanceCouncilRotateTarget;
    const approvalHash = hashMintingGatewayV2RotateGlobalIssuanceCouncilApproval(
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
          kind: MINTING_GATEWAY_V2_UPDATE_KINDS.globalIssuanceCouncilRotate,
          payload: encodeMintingGatewayV2GlobalIssuanceCouncilRotateTarget(target),
          signatures,
        },
      ],
      fixture.relayerArgonAccountId,
    );

    return approvalHash;
  }

  async function createTransferOutRequest(
    fixture: Awaited<ReturnType<typeof deployFixture>>,
    overrides: Partial<MintingGatewayV2TransferOutOfArgonRequest> = {},
  ): Promise<MintingGatewayV2TransferOutOfArgonRequest> {
    const chainId = BigInt((await ethers.provider.getNetwork()).chainId);
    const activeCouncil = await fixture.gateway.globalIssuanceCouncil();

    return {
      argonAccountId:
        overrides.argonAccountId ?? (ethers.encodeBytes32String('account-1') as `0x${string}`),
      argonTransferNonce: overrides.argonTransferNonce ?? 1n,
      chainId,
      councilNumber: overrides.councilNumber ?? activeCouncil.councilNumber,
      recipient: overrides.recipient ?? (fixture.recipient.address as `0x${string}`),
      validUntilBlock: overrides.validUntilBlock ?? 1_000_000n,
      token: overrides.token ?? ((await fixture.argon.getAddress()) as `0x${string}`),
      amount: overrides.amount ?? 50n,
      finalizationTip: overrides.finalizationTip ?? 5n,
    };
  }

  it('rejects token-bearing transfer starts on the bootstrap implementation before canonical tokens exist', async () => {
    const { gateway, holder } = await deployGatewayV2Stack();

    await expectCustomError(
      gateway.connect(holder).startTransferToArgon(
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

  it('rejects migration on the bootstrap implementation before canonical tokens exist', async () => {
    const { adminSafe, gateway, holder } = await deployGatewayV2Stack();

    await expectCustomError(
      gateway
        .connect(adminSafe)
        .migrate(
          { recipients: [holder.address], amounts: [1n * SCALE] },
          { recipients: [holder.address], amounts: [2n * SCALE] },
        ),
      gateway,
      'UnsupportedToken',
      [ethers.ZeroAddress],
    );
  });

  it('starts a transfer to Argon in the ordered gateway activity stream', async () => {
    const { argon, argonot, gateway, holder } = await deployFixture();
    const argonAccountId = ethers.encodeBytes32String('argon-account-1');
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
      Wallet.createRandom() as unknown as SignerLike,
      Wallet.createRandom() as unknown as SignerLike,
      Wallet.createRandom() as unknown as SignerLike,
      Wallet.createRandom() as unknown as SignerLike,
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
    expect(activeCouncil.councilNumber).to.equal(2n);
    expect(activeCouncil.memberCount).to.equal(nextCouncil.memberCount);
    expect(activeCouncil.totalWeight).to.equal(nextCouncil.totalWeight);
    expect(await fixture.gateway.microgonsPerArgonot()).to.equal(
      MICROGONS_PER_ARGONOT,
    );
    expect(await fixture.gateway.previousMicrogonsPerArgonot()).to.equal(
      MICROGONS_PER_ARGONOT,
    );
    expect(await fixture.gateway.argonApprovalsNonce()).to.equal(0n);
    expect(await fixture.gateway.getFunction('argonApprovalsHash')()).to.equal(ethers.ZeroHash);
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
      mintingAuthorityId: ethers.id('minting-authority-force-update') as `0x${string}`,
      microgonCollateral: 1_500n,
      micronotCollateral: 250n,
      signingKey: Wallet.createRandom().address as `0x${string}`,
    } satisfies MintingGatewayV2MintingAuthorityActivationTarget;

    await fixture.gateway.connect(fixture.adminSafe).forceUpdateActiveCouncil(nextCouncil.snapshot);

    const approvalHash = hashMintingGatewayV2ActivateMintingAuthorityApproval(
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
          kind: MINTING_GATEWAY_V2_UPDATE_KINDS.mintingAuthorityActivate,
          payload: encodeMintingGatewayV2MintingAuthorityActivationTarget(target),
          signatures,
        },
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

  it('requires a later queue item to anchor minting authority deactivations', async () => {
    const fixture = await deployFixture();
    const { gateway, council, relayerArgonAccountId } = fixture;
    const { event: activationEvent, target } = await activateMintingAuthority(fixture);

    const deactivationTarget = {
      signingKey: target.signingKey,
    } satisfies MintingGatewayV2MintingAuthorityDeactivateTarget;
    const deactivationHash = hashMintingGatewayV2MintingAuthorityDeactivation(
      await getGatewayHashContext(gateway),
      {
        queueNonce: 2n,
        target: deactivationTarget,
        previousUpdateHash: await gateway.getFunction('argonApprovalsHash')(),
      },
    );
    const wrongDeactivateSignature = await council.wallets[0].signMessage(
      ethers.getBytes(deactivationHash),
    );
    const nextTarget = {
      mintingAuthorityId: ethers.id('minting-authority-2') as `0x${string}`,
      microgonCollateral: 500n,
      micronotCollateral: 50n,
      signingKey: Wallet.createRandom().address as `0x${string}`,
    } satisfies MintingGatewayV2MintingAuthorityActivationTarget;
    const nextActivationHash = hashMintingGatewayV2ActivateMintingAuthorityApproval(
      await getGatewayHashContext(gateway),
      {
        queueNonce: 3n,
        approvingCouncilHash: council.hash,
        previousUpdateHash: deactivationHash,
        target: nextTarget,
      },
    );
    const nextActivationSignatures = await signApprovalHash(council.quorumSigners, nextActivationHash);

    expect(activationEvent.args[0]).to.equal(target.mintingAuthorityId);

    await expectCustomError(
      gateway.applyGatewayUpdates(
        fixture.council.snapshot,
        [
          {
            queueNonce: 2n,
            kind: MINTING_GATEWAY_V2_UPDATE_KINDS.mintingAuthorityDeactivate,
            payload: encodeMintingGatewayV2MintingAuthorityDeactivateTarget(deactivationTarget),
            signatures: [wrongDeactivateSignature],
          },
          {
            queueNonce: 3n,
            kind: MINTING_GATEWAY_V2_UPDATE_KINDS.mintingAuthorityActivate,
            payload: encodeMintingGatewayV2MintingAuthorityActivationTarget(nextTarget),
            signatures: nextActivationSignatures,
          },
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
    await expectCustomError(
      gateway.applyGatewayUpdates(
        fixture.council.snapshot,
        [
          {
            queueNonce: 2n,
            kind: MINTING_GATEWAY_V2_UPDATE_KINDS.mintingAuthorityDeactivate,
            payload: encodeMintingGatewayV2MintingAuthorityDeactivateTarget(deactivationTarget),
            signatures: [deactivateSignature],
          },
        ],
        relayerArgonAccountId,
      ),
      gateway,
      'LatestGatewayUpdateCannotDeactivate',
    );

    const tx = gateway.applyGatewayUpdates(
      fixture.council.snapshot,
      [
        {
          queueNonce: 2n,
          kind: MINTING_GATEWAY_V2_UPDATE_KINDS.mintingAuthorityDeactivate,
          payload: encodeMintingGatewayV2MintingAuthorityDeactivateTarget(deactivationTarget),
          signatures: [deactivateSignature],
        },
        {
          queueNonce: 3n,
          kind: MINTING_GATEWAY_V2_UPDATE_KINDS.mintingAuthorityActivate,
          payload: encodeMintingGatewayV2MintingAuthorityActivationTarget(nextTarget),
          signatures: nextActivationSignatures,
        },
      ],
      relayerArgonAccountId,
    );
    const deactivateEvent = await parseGatewayEvent(tx, gateway, 'MintingAuthorityDeactivated');
    const activateEvent = await parseGatewayEvent(tx, gateway, 'MintingAuthorityActivated');

    const mintingCollateral =
      await gateway.mintingAuthorityCollateralRemaining(target.signingKey);
    const nextMintingCollateral =
      await gateway.mintingAuthorityCollateralRemaining(nextTarget.signingKey);
    expect(mintingCollateral.microgonCollateral).to.equal(0n);
    expect(mintingCollateral.micronotCollateral).to.equal(0n);
    expect(nextMintingCollateral.microgonCollateral).to.equal(nextTarget.microgonCollateral);
    expect(nextMintingCollateral.micronotCollateral).to.equal(nextTarget.micronotCollateral);
    expect(await gateway.argonApprovalsNonce()).to.equal(3n);
    expect(await gateway.gatewayActivityNonce()).to.equal(3n);

    expect(deactivateEvent.args[0]).to.equal(target.signingKey);
    expect(deactivateEvent.args[1]).to.equal(target.microgonCollateral);
    expect(deactivateEvent.args[2]).to.equal(target.micronotCollateral);
    expect(activateEvent.args[0]).to.equal(nextTarget.mintingAuthorityId);
  });

  it('only needs council signatures on rotation items and the last council-approved item in a batch', async () => {
    const fixture = await deployFixture();
    const nextCouncil = createCouncil([
      Wallet.createRandom() as unknown as SignerLike,
      Wallet.createRandom() as unknown as SignerLike,
      Wallet.createRandom() as unknown as SignerLike,
      Wallet.createRandom() as unknown as SignerLike,
    ]);
    const firstTarget = {
      mintingAuthorityId: ethers.id('minting-authority-batch-1') as `0x${string}`,
      microgonCollateral: 1_000n,
      micronotCollateral: 100n,
      signingKey: Wallet.createRandom().address as `0x${string}`,
    } satisfies MintingGatewayV2MintingAuthorityActivationTarget;
    const secondTarget = {
      mintingAuthorityId: ethers.id('minting-authority-batch-2') as `0x${string}`,
      microgonCollateral: 2_000n,
      micronotCollateral: 150n,
      signingKey: Wallet.createRandom().address as `0x${string}`,
    } satisfies MintingGatewayV2MintingAuthorityActivationTarget;
    const activationOneHash = hashMintingGatewayV2ActivateMintingAuthorityApproval(
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
      microgonsPerArgonot: nextCouncil.microgonsPerArgonot,
    } satisfies MintingGatewayV2GlobalIssuanceCouncilRotateTarget;
    const rotationHash = hashMintingGatewayV2RotateGlobalIssuanceCouncilApproval(
      await getGatewayHashContext(fixture.gateway),
      {
        queueNonce: 2n,
        approvingCouncilHash: fixture.council.hash,
        previousUpdateHash: activationOneHash,
        target: rotationTarget,
      },
    );
    const activationTwoHash = hashMintingGatewayV2ActivateMintingAuthorityApproval(
      await getGatewayHashContext(fixture.gateway),
      {
        queueNonce: 3n,
        approvingCouncilHash: nextCouncil.hash,
        previousUpdateHash: rotationHash,
        target: secondTarget,
      },
    );
    const rotationSignatures = await signApprovalHash(fixture.council.quorumSigners, rotationHash);
    const activationTwoSignatures = await signApprovalHash(nextCouncil.quorumSigners, activationTwoHash);

    await fixture.gateway.applyGatewayUpdates(
      fixture.council.snapshot,
      [
        {
          queueNonce: 1n,
          kind: MINTING_GATEWAY_V2_UPDATE_KINDS.mintingAuthorityActivate,
          payload: encodeMintingGatewayV2MintingAuthorityActivationTarget(firstTarget),
          signatures: [],
        },
        {
          queueNonce: 2n,
          kind: MINTING_GATEWAY_V2_UPDATE_KINDS.globalIssuanceCouncilRotate,
          payload: encodeMintingGatewayV2GlobalIssuanceCouncilRotateTarget(rotationTarget),
          signatures: rotationSignatures,
        },
        {
          queueNonce: 3n,
          kind: MINTING_GATEWAY_V2_UPDATE_KINDS.mintingAuthorityActivate,
          payload: encodeMintingGatewayV2MintingAuthorityActivationTarget(secondTarget),
          signatures: activationTwoSignatures,
        },
      ],
      fixture.relayerArgonAccountId,
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
    expect(activeCouncil.councilNumber).to.equal(2n);
    expect(await fixture.gateway.microgonsPerArgonot()).to.equal(
      nextCouncil.microgonsPerArgonot,
    );
    expect(await fixture.gateway.previousMicrogonsPerArgonot()).to.equal(
      MICROGONS_PER_ARGONOT,
    );
  });

  it('lets anyone finalize a transfer out of Argon to the signed recipient once authorizations are valid', async () => {
    const fixture = await deployFixture();
    const { argon, gateway, outsider, recipient } = fixture;
    const { target } = await activateMintingAuthority(fixture);
    const request = await createTransferOutRequest(fixture, {
      recipient: recipient.address as `0x${string}`,
      token: (await argon.getAddress()) as `0x${string}`,
      amount: 50n,
    });
    const signature = await fixture.mintingAuthoritySigner.signMessage(
      ethers.getBytes(
        hashMintingGatewayV2MintingAuthorization(
          await getGatewayHashContext(gateway),
          { request, microgonCollateral: 80n, micronotCollateral: 0n },
        ),
      ),
    );
    const transferId = hashMintingGatewayV2TransferOutOfArgonRequest(request);

    const event = await parseGatewayEvent(
      gateway.connect(outsider).finalizeTransferOutOfArgon(
        request,
        { authorizations: [{ microgonCollateral: 80n, micronotCollateral: 0n, signature }] },
      ),
      gateway,
      'TransferOutOfArgonFinalized',
    );

    const mintingCollateral =
      await gateway.mintingAuthorityCollateralRemaining(target.signingKey);
    expect(await argon.balanceOf(recipient.address)).to.equal(50n * SCALE);
    expect(await gateway.gatewayActivityNonce()).to.equal(2n);
    expect(await gateway.finalizedTransferOutOfArgonIds(transferId)).to.equal(true);
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
      gateway.connect(outsider).finalizeTransferOutOfArgon(
        request,
        { authorizations: [{ microgonCollateral: 80n, micronotCollateral: 0n, signature }] },
      ),
      gateway,
      'TransferOutOfArgonAlreadyFinalized',
      [transferId],
    );
  });

  it('uses the previous council floor for in-flight Argon payouts', async () => {
    const fixture = await deployFixture();
    const signingWallet = Wallet.createRandom() as unknown as SignerLike;
    const { target } = await activateMintingAuthority(fixture, 1n, {
      microgonCollateral: 0n,
      micronotCollateral: 50n,
      signingKey: signingWallet.address,
    });
    const nextCouncil = createCouncil(
      [
        Wallet.createRandom() as unknown as SignerLike,
        Wallet.createRandom() as unknown as SignerLike,
        Wallet.createRandom() as unknown as SignerLike,
        Wallet.createRandom() as unknown as SignerLike,
      ],
      500_000n,
    );

    await rotateCouncil(fixture, 2n, nextCouncil);

    const request = await createTransferOutRequest(fixture, {
      councilNumber: 1n,
      amount: 50n,
    });
    const signature = await signingWallet.signMessage(
      ethers.getBytes(
        hashMintingGatewayV2MintingAuthorization(
          await getGatewayHashContext(fixture.gateway),
          { request, microgonCollateral: 0n, micronotCollateral: 50n },
        ),
      ),
    );

    await fixture.gateway.connect(fixture.outsider).finalizeTransferOutOfArgon(
      request,
      { authorizations: [{ microgonCollateral: 0n, micronotCollateral: 50n, signature }] },
    );

    const mintingCollateral =
      await fixture.gateway.mintingAuthorityCollateralRemaining(target.signingKey);
    expect(mintingCollateral.microgonCollateral).to.equal(0n);
    expect(mintingCollateral.micronotCollateral).to.equal(0n);
  });

  it('allows mixed Argon and Argonot collateral on Argon payouts', async () => {
    const fixture = await deployFixture();
    await activateMintingAuthority(fixture);
    const request = await createTransferOutRequest(fixture, { amount: 50n });
    const signature = await fixture.mintingAuthoritySigner.signMessage(
      ethers.getBytes(
        hashMintingGatewayV2MintingAuthorization(
          await getGatewayHashContext(fixture.gateway),
          { request, microgonCollateral: 25n, micronotCollateral: 25n },
        ),
      ),
    );

    await fixture.gateway.connect(fixture.outsider).finalizeTransferOutOfArgon(
      request,
      { authorizations: [{ microgonCollateral: 25n, micronotCollateral: 25n, signature }] },
    );
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
        hashMintingGatewayV2MintingAuthorization(
          await getGatewayHashContext(fixture.gateway),
          { request, microgonCollateral: 1n, micronotCollateral: 50n },
        ),
      ),
    );

    await expectCustomError(
      fixture.gateway.connect(fixture.outsider).finalizeTransferOutOfArgon(
        request,
        { authorizations: [{ microgonCollateral: 1n, micronotCollateral: 50n, signature }] },
      ),
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
      finalizationTip: 1n,
    });
    const expiredRequest = {
      ...notExpiredRequest,
      argonTransferNonce: 8n,
      validUntilBlock: 0n,
    } satisfies MintingGatewayV2TransferOutOfArgonRequest;
    const expiredTransferId = hashMintingGatewayV2TransferOutOfArgonRequest(expiredRequest);

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
});
