// SPDX-License-Identifier: MIT
pragma solidity ^0.8.24;

import {ICanonicalToken} from "./interfaces/ICanonicalToken.sol";
import {OwnableUpgradeable} from "@openzeppelin/contracts-upgradeable/access/OwnableUpgradeable.sol";
import {PausableUpgradeable} from "@openzeppelin/contracts-upgradeable/utils/PausableUpgradeable.sol";
import {Initializable} from "@openzeppelin/contracts-upgradeable/proxy/utils/Initializable.sol";
import {ECDSA} from "@openzeppelin/contracts/utils/cryptography/ECDSA.sol";
import {MessageHashUtils} from "@openzeppelin/contracts/utils/cryptography/MessageHashUtils.sol";

/// @title MintingGateway
/// @author Argon Protocol
/// @notice Gateway for transfers between this chain and Argon, plus council-managed minting authority updates.
contract MintingGateway is Initializable, OwnableUpgradeable, PausableUpgradeable {
	using MessageHashUtils for bytes32;

	/// @notice Decimal precision used by Argon runtime balances.
	uint8 public constant RUNTIME_DECIMALS = 6;
	/// @notice Decimal precision used by the canonical ERC-20 tokens.
	uint8 public constant TOKEN_DECIMALS = 18;
	/// @notice Conversion factor from runtime units into ERC-20 base units.
	uint256 public constant RUNTIME_TO_ERC20_SCALE = 10 ** (TOKEN_DECIMALS - RUNTIME_DECIMALS);

	bytes32 private constant _GLOBAL_ISSUANCE_COUNCIL_ROTATION_TAG =
		keccak256("ARGON_GLOBAL_ISSUANCE_COUNCIL_ROTATION");
	bytes32 private constant _MINTING_AUTHORITY_ACTIVATION_TAG =
		keccak256("ARGON_MINTING_AUTHORITY_ACTIVATION");
	bytes32 private constant _MINTING_AUTHORITY_DEACTIVATION_TAG =
		keccak256("ARGON_MINTING_AUTHORITY_DEACTIVATION");
	bytes32 private constant _GATEWAY_UPDATE_APPROVAL_TAG =
		keccak256("ARGON_GATEWAY_UPDATE_APPROVAL");
	bytes32 private constant _TRANSFER_OUT_OF_ARGON_AUTHORIZATION_TAG =
		keccak256("ARGON_TRANSFER_OUT_OF_ARGON_AUTHORIZATION");

	/// @notice Canonical Argon token managed by this gateway.
	address public immutable argonToken;
	/// @notice Canonical Argonot token managed by this gateway.
	address public immutable argonotToken;
	/// @notice Address that can pause the gateway without being the owner.
	address public guardian;

	/// @notice Running activity number for gateway events emitted on this chain.
	uint64 public gatewayActivityNonce;
	/// @notice Latest Argon queue item that has been applied on this chain.
	uint64 public argonApprovalsNonce;
	/// @notice Chained signed hash of the latest Argon queue item applied on this chain.
	bytes32 public argonApprovalsHash;
	/// @notice Latest block-locator entry written into activityBlockLocators.
	uint64 public latestActivityBlockLocatorIndex;
	/// @notice Tracks whether the one-time migration balances have already been applied.
	bool public migrationCompleted;

	/// @notice Current council snapshot.
	GlobalIssuanceCouncil public globalIssuanceCouncil;
	/// @notice Prior active council hash kept for in-flight transfer windows.
	bytes32 public previousGlobalIssuanceCouncilHash;
	/// @notice Previous council-approved floor kept for in-flight transfer windows.
	uint128 public previousMicrogonsPerArgonot;
	/// @notice Minting-authority collateral keyed by the signing key for that bucket.
	mapping(address signingKey => MintingCollateral collateralRemaining)
		public mintingAuthorityCollateralRemaining;
	/// @notice Permanent replay marker for finalized transfer-out requests.
	mapping(bytes32 transferId => bool isFinalized) public finalizedTransferOutOfArgonIds;
	/// @notice Per-block activity ranges used to locate gateway events later.
	mapping(uint64 locatorIndex => ActivityBlockLocator locator) public activityBlockLocators;

	/// @notice Activity-number range emitted in one block on this chain.
	struct ActivityBlockLocator {
		uint64 blockNumber;
		uint64 startGatewayActivityNonce;
		uint64 endGatewayActivityNonce;
	}

	/// @notice Shared gateway snapshot appended to every activity event.
	struct GatewayActivityState {
		uint64 gatewayActivityNonce;
		uint64 argonApprovalsNonce;
		uint128 argonCirculation;
		uint128 argonotCirculation;
	}

	/// @notice Current council snapshot used to verify queued updates.
	struct GlobalIssuanceCouncil {
		uint256 totalWeight;
		uint256 memberCount;
		bytes32 councilHash;
		uint128 epochMicrogonsPerArgonot;
	}

	/// @notice One council member snapshot used to verify council-approved items.
	struct CouncilSnapshot {
		address[] signers;
		uint256[] weights;
	}

	/// @notice Decoded payload for a council rotation queue item.
	struct GlobalIssuanceCouncilRotateTarget {
		CouncilSnapshot council;
		uint128 epochMicrogonsPerArgonot;
	}

	/// @notice In-memory active council state used while applying one contiguous queue batch.
	struct ActiveCouncilSnapshot {
		address[] signers;
		uint256[] weights;
		bytes32 councilHash;
		uint256 totalWeight;
		uint256 memberCount;
	}

	/// @notice Minting-authority collateral tracked on this chain for one signing key.
	struct MintingCollateral {
		uint128 microgonCollateral;
		uint128 micronotCollateral;
	}

	/// @notice Collateral used by one minting authority in a finalized transfer-out.
	struct MintingAuthorityCollateral {
		address signingKey;
		uint128 microgonCollateral;
		uint128 micronotCollateral;
	}

	/// @notice Transfer-out request that minting authorities sign.
	struct TransferOutOfArgonRequest {
		bytes32 argonAccountId;
		uint64 argonTransferNonce;
		uint64 chainId;
		bytes32 councilHash;
		address recipient;
		uint64 validUntilBlock;
		address token;
		uint128 amount;
		uint128 finalizationTip;
	}

	/// @notice One minting authority's signed authorization for a transfer-out.
	struct MintingAuthorization {
		uint128 microgonCollateral;
		uint128 micronotCollateral;
		bytes signature;
	}

	/// @notice All minting-authority authorizations needed to finalize a transfer-out.
	struct TransferOutOfArgonProof {
		MintingAuthorization[] authorizations;
	}

	/// @notice One prior-contract migration distribution using exact token base units.
	struct MigrationAssetDistribution {
		address[] recipients;
		uint256[] amounts;
	}

	/// @notice Decoded payload for a minting-authority activation queue item.
	struct MintingAuthorityActivationTarget {
		uint128 microgonCollateral;
		uint128 micronotCollateral;
		address signingKey;
	}

	/// @notice Decoded payload for a minting-authority deactivation queue item.
	struct MintingAuthorityDeactivateTarget {
		/// @notice Signing key that must authorize the deactivation on this chain.
		address signingKey;
	}

	/// @notice Queue update kinds accepted by applyGatewayUpdates(...).
	enum GatewayUpdateKind {
		GlobalIssuanceCouncilRotate,
		MintingAuthorityActivate,
		MintingAuthorityDeactivate
	}

	/// @notice One queued Argon update with kind-specific payload and signatures.
	struct GatewayUpdate {
		uint64 queueNonce;
		GatewayUpdateKind kind;
		/// @notice Encoded target payload for the queue item kind.
		bytes payload;
		/// @notice Sorted council signatures for council-segment tips, or one authority signature for deactivation.
		bytes[] signatures;
	}

	error ArrayLengthMismatch();
	error GlobalIssuanceCouncilNotBootstrapped();
	error GlobalIssuanceCouncilQuorumNotMet();
	error InvalidChainId(uint64 expected, uint64 provided);
	error InvalidCurrentCouncilSnapshot(bytes32 expected, bytes32 provided);
	error InvalidGlobalIssuanceCouncilMember(uint256 index);
	error InvalidMintingAuthority(address signingKey);
	error InvalidMintingAuthorityDeactivationSigner(address expectedSigner, address recoveredSigner);
	error InvalidMicrogonCollateralForArgonotPayout();
	error InvalidQueueNonce(uint64 expected, uint64 provided);
	error InvalidSignatureCount(uint256 expected, uint256 provided);
	error InvalidSignatureOrder();
	error InvalidUpdateKind(uint8 kind);
	error LatestGatewayUpdateCannotDeactivate();
	error InsufficientAuthorizedCollateral(uint256 authorized, uint256 required);
	error MigrationAlreadyCompleted();
	error MintingAuthorityAlreadyActive(address signingKey);
	error MintingAuthorityTransferCapacityExceeded(address signingKey);
	error NotGuardianOrOwner(address caller);
	error TransferOutOfArgonAlreadyFinalized(bytes32 transferId);
	error TransferOutOfArgonExpired(uint256 currentBlock, uint256 validUntilBlock);
	error TransferOutOfArgonNotExpired(uint256 currentBlock, uint256 validUntilBlock);
	error UnsupportedToken(address token);
	error RuntimeAmountOverflow(uint256 amount);
	error UnknownCouncilHash(bytes32 councilHash);
	error ZeroAdminSafe();
	error ZeroAmount();
	error ZeroGuardian();
	error ZeroMicrogonsPerArgonot();
	error ZeroRecipient(uint256 index);
	error ZeroSigningKey();

	/// @notice Emitted when the first council snapshot is set.
	/// @param councilHash Hash of the bootstrapped council membership and weights.
	event GlobalIssuanceCouncilBootstrapped(bytes32 indexed councilHash);
	/// @notice Emitted when the owner force-replaces the active council summary.
	/// @param previousCouncilHash Hash of the council membership and weights before the recovery update.
	/// @param councilHash Hash of the replacement council membership and weights.
	event GlobalIssuanceCouncilForceUpdated(
		bytes32 indexed previousCouncilHash,
		bytes32 indexed councilHash
	);
	/// @notice Emitted when one-time migration balances from the prior contracts are applied.
	/// @param argonRecipientCount Number of Argon recipients included in the migration.
	/// @param argonTotalAmount Total Argon base units minted during the migration.
	/// @param argonotRecipientCount Number of Argonot recipients included in the migration.
	/// @param argonotTotalAmount Total Argonot base units minted during the migration.
	event MigrationCompleted(
		uint256 argonRecipientCount,
		uint256 argonTotalAmount,
		uint256 argonotRecipientCount,
		uint256 argonotTotalAmount
	);
	/// @notice Emitted when a queued council rotation is applied.
	/// @param councilHash Hash of the new council membership and weights.
	/// @param relayerArgonAccountId Argon account that submitted the update relay.
	/// @param gatewayState Shared gateway state snapshot after the update lands.
	event GlobalIssuanceCouncilRotated(
		bytes32 councilHash,
		bytes32 relayerArgonAccountId,
		GatewayActivityState gatewayState
	);
	/// @notice Emitted when the owner changes the guardian.
	/// @param previousGuardian Guardian address before the update.
	/// @param newGuardian Guardian address after the update.
	event GuardianUpdated(address indexed previousGuardian, address indexed newGuardian);
	/// @notice Emitted when a queued minting-authority activation is applied.
	/// @param microgonCollateral Argon collateral activated on this chain.
	/// @param micronotCollateral Argonot collateral activated on this chain.
	/// @param signingKey Signing key that can authorize transfer finalizations.
	/// @param coactivationCount Number of minting-authority activations sharing this realized
	/// signature tranche.
	/// @param sharedSignatureCount Number of council signatures actually supplied across that
	/// tranche, including queued council rotations ahead of the tranche and the final signed head
	/// update.
	/// @param relayerArgonAccountId Argon account that submitted the update relay.
	/// @param gatewayState Shared gateway state snapshot after the update lands.
	event MintingAuthorityActivated(
		address indexed signingKey,
		uint128 microgonCollateral,
		uint128 micronotCollateral,
		uint32 coactivationCount,
		uint32 sharedSignatureCount,
		bytes32 relayerArgonAccountId,
		GatewayActivityState gatewayState
	);
	/// @notice Emitted when a queued minting-authority deactivation is applied.
	/// @param signingKey Signing key for the minting authority being deactivated.
	/// @param microgonCollateral Remaining Argon collateral returned by the deactivation.
	/// @param micronotCollateral Remaining Argonot collateral returned by the deactivation.
	/// @param relayerArgonAccountId Argon account that submitted the update relay.
	/// @param gatewayState Shared gateway state snapshot after the update lands.
	event MintingAuthorityDeactivated(
		address indexed signingKey,
		uint128 microgonCollateral,
		uint128 micronotCollateral,
		bytes32 relayerArgonAccountId,
		GatewayActivityState gatewayState
	);
	/// @notice Emitted when tokens are burned here to start a transfer to Argon.
	/// @param from Account that burned the canonical tokens.
	/// @param token Canonical token that was burned.
	/// @param amount Runtime-unit amount sent to Argon.
	/// @param argonAccountId Argon account that should receive the transfer.
	/// @param gatewayState Shared gateway state snapshot after the burn.
	event TransferToArgonStarted(
		address indexed from,
		address indexed token,
		uint128 amount,
		bytes32 argonAccountId,
		GatewayActivityState gatewayState
	);
	/// @notice Emitted when an expired transfer out of Argon is canceled on this chain.
	/// @param transferId Canonical identifier for the canceled transfer-out request.
	/// @param gatewayState Shared gateway state snapshot after the cancel lands.
	event TransferOutOfArgonCanceled(
		bytes32 transferId,
		GatewayActivityState gatewayState
	);
	/// @notice Emitted when a transfer out of Argon is finalized on this chain.
	/// @param transferId Canonical identifier for the finalized transfer-out request.
	/// @param token Canonical token minted to the recipient.
	/// @param amount Runtime-decimal amount minted to the recipient.
	/// @param mintingCollateral Collateral committed by each active minting authority key.
	/// @param gatewayState Shared gateway state snapshot after the finalization lands.
	event TransferOutOfArgonFinalized(
		bytes32 transferId,
		address token,
		uint128 amount,
		MintingAuthorityCollateral[] mintingCollateral,
		GatewayActivityState gatewayState
	);

	/// @custom:oz-upgrades-unsafe-allow constructor
	/// @dev The implementation constructor stores immutable token addresses and disables direct initialization.
	constructor(address argonTokenAddress, address argonotTokenAddress) {
		argonToken = argonTokenAddress;
		argonotToken = argonotTokenAddress;
		_disableInitializers();
	}

	/// @notice Initializes the gateway proxy and stores the first council summary.
	/// @param adminSafe Owner address for administration and upgrades.
	/// @param guardianAddress Emergency pause role address.
	/// @param councilHash Hash of the first council membership snapshot.
	/// @param councilMemberCount Number of members in the first council snapshot.
	/// @param councilTotalWeight Total signer weight in the first council snapshot.
	/// @param initialMicrogonsPerArgonot Initial council-approved floor, measured in microgons per 1 whole Argonot.
	function initialize(
		address adminSafe,
		address guardianAddress,
		bytes32 councilHash,
		uint256 councilMemberCount,
		uint256 councilTotalWeight,
		uint128 initialMicrogonsPerArgonot
	) external initializer {
		if (adminSafe == address(0)) revert ZeroAdminSafe();
		if (guardianAddress == address(0)) revert ZeroGuardian();
		if (councilHash == bytes32(0) || councilMemberCount == 0 || councilTotalWeight == 0) {
			revert InvalidGlobalIssuanceCouncilMember(0);
		}
		if (initialMicrogonsPerArgonot == 0) revert ZeroMicrogonsPerArgonot();

		__Ownable_init(adminSafe);
		__Pausable_init();
		guardian = guardianAddress;

		_storeGlobalIssuanceCouncil(
			councilHash,
			councilTotalWeight,
			councilMemberCount,
			initialMicrogonsPerArgonot
		);
		previousMicrogonsPerArgonot = initialMicrogonsPerArgonot;

		emit GlobalIssuanceCouncilBootstrapped(councilHash);
	}

	/// @notice Loads one-time migration balances from the prior contracts.
	/// @param argonMigration Argon migration recipients and exact base-unit amounts.
	/// @param argonotMigration Argonot migration recipients and exact base-unit amounts.
	function migrate(
		MigrationAssetDistribution calldata argonMigration,
		MigrationAssetDistribution calldata argonotMigration
	) external onlyOwner whenNotPaused {
		if (migrationCompleted) revert MigrationAlreadyCompleted();

		uint256 argonTotalAmount =
			_mintMigrationBalances(argonToken, argonMigration.recipients, argonMigration.amounts);
		uint256 argonotTotalAmount =
			_mintMigrationBalances(argonotToken, argonotMigration.recipients, argonotMigration.amounts);

		migrationCompleted = true;

		emit MigrationCompleted(
			argonMigration.recipients.length,
			argonTotalAmount,
			argonotMigration.recipients.length,
			argonotTotalAmount
		);
	}

	/// @notice Emergency owner seam to replace the active council summary without consuming queue items.
	/// @dev Leaves argonApprovalsNonce and argonApprovalsHash unchanged so later queue items still chain
	/// from the last applied Argon update while using the replacement council for future approvals.
	/// @param nextCouncil Replacement active council snapshot.
	function forceUpdateActiveCouncil(
		CouncilSnapshot calldata nextCouncil
	) external onlyOwner {
		(bytes32 nextCouncilHash, uint256 nextCouncilTotalWeight, uint256 nextCouncilMemberCount) =
			_validateCouncilSnapshot(
				nextCouncil.signers,
				nextCouncil.weights,
				globalIssuanceCouncil.epochMicrogonsPerArgonot
			);
		GlobalIssuanceCouncil memory activeCouncil = globalIssuanceCouncil;

		_rollMicrogonsPerArgonot();
		_storeGlobalIssuanceCouncil(
			nextCouncilHash,
			nextCouncilTotalWeight,
			nextCouncilMemberCount,
			activeCouncil.epochMicrogonsPerArgonot
		);

		emit GlobalIssuanceCouncilForceUpdated(activeCouncil.councilHash, nextCouncilHash);
	}

	/// @notice Applies queued updates from Argon after verifying the required signatures for each item kind.
	/// @dev Updates must start at argonApprovalsNonce + 1 and continue without gaps.
	/// @param currentCouncil Current active council snapshot in force for the first council-approved item in the batch.
	/// @param updates Ordered queue updates to apply.
	/// @param relayerArgonAccountId Argon-side relayer identifier included in the resulting activity events.
	function applyGatewayUpdates(
		CouncilSnapshot calldata currentCouncil,
		GatewayUpdate[] calldata updates,
		bytes32 relayerArgonAccountId
	) external whenNotPaused {
		if (
			updates.length != 0
				&& updates[updates.length - 1].kind == GatewayUpdateKind.MintingAuthorityDeactivate
		) {
			revert LatestGatewayUpdateCannotDeactivate();
		}

		if (globalIssuanceCouncil.councilHash == bytes32(0)) {
			revert GlobalIssuanceCouncilNotBootstrapped();
		}

		ActiveCouncilSnapshot memory activeCouncil = _currentActiveCouncilSnapshot(currentCouncil);
		bytes32 previousUpdateHash = argonApprovalsHash;
		uint32[] memory activationCohortCounts = new uint32[](updates.length);
		uint32[] memory activationSharedSignatureCounts = new uint32[](updates.length);
		uint256 lastSignedHeadIndex = _collectActivationCohorts(
			updates,
			activationCohortCounts,
			activationSharedSignatureCounts
		);

		for (uint256 index = 0; index < updates.length; ++index) {
			GatewayUpdate calldata update = updates[index];

			uint64 expectedQueueNonce = argonApprovalsNonce + 1;
			if (update.queueNonce != expectedQueueNonce) {
				revert InvalidQueueNonce(expectedQueueNonce, update.queueNonce);
			}

			if (update.kind == GatewayUpdateKind.MintingAuthorityDeactivate) {
				previousUpdateHash =
					_applyMintingAuthorityDeactivate(update, previousUpdateHash, relayerArgonAccountId);
			} else if (update.kind == GatewayUpdateKind.GlobalIssuanceCouncilRotate) {
				(activeCouncil, previousUpdateHash) = _applyGlobalIssuanceCouncilUpdate(
					update,
					activeCouncil,
					previousUpdateHash,
					relayerArgonAccountId
				);
			} else if (update.kind == GatewayUpdateKind.MintingAuthorityActivate){
				previousUpdateHash = _applyMintingAuthorityActivation(
					update,
					activeCouncil,
					previousUpdateHash,
					index == lastSignedHeadIndex,
					activationCohortCounts[index],
					activationSharedSignatureCounts[index],
					relayerArgonAccountId
				);
			} else {
				revert InvalidUpdateKind(uint8(update.kind));
			}
		}
	}

	function _currentActiveCouncilSnapshot(
		CouncilSnapshot calldata currentCouncil
	) private view returns (ActiveCouncilSnapshot memory activeCouncil) {
		(bytes32 activeCouncilHash, uint256 activeCouncilTotalWeight, uint256 activeCouncilMemberCount) =
			_validateCouncilSnapshot(
				currentCouncil.signers,
				currentCouncil.weights,
				globalIssuanceCouncil.epochMicrogonsPerArgonot
			);
		if (activeCouncilHash != globalIssuanceCouncil.councilHash
			|| activeCouncilTotalWeight != globalIssuanceCouncil.totalWeight
			|| activeCouncilMemberCount != globalIssuanceCouncil.memberCount) {
			revert InvalidCurrentCouncilSnapshot(
				globalIssuanceCouncil.councilHash, activeCouncilHash
			);
		}

		return ActiveCouncilSnapshot({
			signers: currentCouncil.signers,
			weights: currentCouncil.weights,
			councilHash: activeCouncilHash,
			totalWeight: activeCouncilTotalWeight,
			memberCount: activeCouncilMemberCount
		});
	}

	function _collectActivationCohorts(
		GatewayUpdate[] calldata updates,
		uint32[] memory activationCohortCounts,
		uint32[] memory activationSharedSignatureCounts
	)
		private
		pure
		returns (uint256 lastSignedHeadIndex)
	{
		// `applyGatewayUpdates(...)` rejects batches whose final item is a deactivation, so this
		// scan finds the last non-deactivation head that can actually land any pending activations.
		// Trailing deactivations carry their own one-off signatures and do not close an activation
		// tranche.
		lastSignedHeadIndex = updates.length;
		uint32[] memory pendingActivationIndexes = new uint32[](updates.length);
		uint32 pendingActivationCount = 0;
		uint32 pendingSharedSignatureCount = 0;

		for (uint256 index = updates.length; index > 0;) {
			--index;

			if (updates[index].kind != GatewayUpdateKind.MintingAuthorityDeactivate) {
				lastSignedHeadIndex = index;
				break;
			}
		}

		for (uint256 index = 0; index < updates.length; ++index) {
			GatewayUpdateKind kind = updates[index].kind;
			if (kind == GatewayUpdateKind.MintingAuthorityActivate) {
				pendingActivationIndexes[pendingActivationCount] = uint32(index);
				++pendingActivationCount;
			}

			if (
				kind == GatewayUpdateKind.GlobalIssuanceCouncilRotate
					|| index == lastSignedHeadIndex
			) {
				pendingSharedSignatureCount += uint32(updates[index].signatures.length);
				if (pendingActivationCount != 0) {
					for (uint256 activationIndex = 0; activationIndex < pendingActivationCount; ++activationIndex) {
						uint256 updateIndex = uint256(pendingActivationIndexes[activationIndex]);
						activationCohortCounts[updateIndex] = pendingActivationCount;
						activationSharedSignatureCounts[updateIndex] = pendingSharedSignatureCount;
					}
					pendingActivationCount = 0;
					pendingSharedSignatureCount = 0;
				}
			}
		}
	}

	/// @notice Burns tokens on this chain to start a transfer to Argon.
	/// @param token Canonical token to permit and burn.
	/// @param amount Argon-chain-units (6 decimals) amount to send.
	/// @param argonAccountId Argon account that should receive the transfer.
	/// @param deadline Permit expiry timestamp.
	/// @param v Permit signature recovery id.
	/// @param r Permit signature r value.
	/// @param s Permit signature s value.
	function startTransferToArgon(
		address token,
		uint128 amount,
		bytes32 argonAccountId,
		uint256 deadline,
		uint8 v,
		bytes32 r,
		bytes32 s
	) external whenNotPaused {
		_requireCanonicalToken(token);
		if (amount == 0) revert ZeroAmount();

		uint256 tokenAmount = toTokenAmount(amount);
		ICanonicalToken(token).permit(msg.sender, address(this), tokenAmount, deadline, v, r, s);
		ICanonicalToken(token).burnFrom(msg.sender, tokenAmount);

		uint64 activityNonce = _nextGatewayActivityNonce();
		emit TransferToArgonStarted(
			msg.sender,
			token,
			amount,
			argonAccountId,
			_gatewayActivityState(activityNonce)
		);
	}

	/// @notice Finalizes a valid transfer out of Argon and mints to the request recipient.
	/// @dev Anyone can call this. The signed request fixes the recipient and the required collateral.
	/// @param request Signed transfer-out request.
	/// @param proof Minting-authority authorizations used to finalize the request.
	function finalizeTransferOutOfArgon(
		TransferOutOfArgonRequest calldata request,
		TransferOutOfArgonProof calldata proof
	) external whenNotPaused {
		_requireTransferOutOfArgonChain(request);
		_requireCanonicalToken(request.token);
		if (request.amount == 0) revert ZeroAmount();

		if (block.number > request.validUntilBlock) {
			revert TransferOutOfArgonExpired(block.number, request.validUntilBlock);
		}

		bytes32 transferId = _hashTransferOutOfArgonRequest(request);
		_requireTransferOutOfArgonNotFinalized(transferId);
		uint128 resolvedMicrogonsPerArgonot = _resolveMicrogonsPerArgonot(request.councilHash);
		uint128 authorizedMicrogonCollateral = 0;
		uint128 authorizedMicronotCollateral = 0;
		address previousSigningKey = address(0);
		MintingAuthorityCollateral[] memory mintingCollateral =
			new MintingAuthorityCollateral[](proof.authorizations.length);

		for (uint256 index = 0; index < proof.authorizations.length; ++index) {
			MintingAuthorization calldata authorization = proof.authorizations[index];
			address signingKey = ECDSA.recoverCalldata(
				_hashMintingAuthorization(
					request,
					authorization.microgonCollateral,
					authorization.micronotCollateral
				).toEthSignedMessageHash(),
				authorization.signature
			);

			// Require strictly increasing signing keys so the same collateral bucket cannot appear twice (eg, user replays the same signature).
			if (signingKey <= previousSigningKey) {
				revert InvalidSignatureOrder();
			}

			MintingCollateral storage collateralRemaining =
				mintingAuthorityCollateralRemaining[signingKey];
			if (
				collateralRemaining.microgonCollateral < authorization.microgonCollateral ||
				collateralRemaining.micronotCollateral < authorization.micronotCollateral
			) {
				revert MintingAuthorityTransferCapacityExceeded(signingKey);
			}

			collateralRemaining.microgonCollateral -= authorization.microgonCollateral;
			collateralRemaining.micronotCollateral -= authorization.micronotCollateral;

			authorizedMicrogonCollateral += authorization.microgonCollateral;
			authorizedMicronotCollateral += authorization.micronotCollateral;
			mintingCollateral[index] = MintingAuthorityCollateral({
				signingKey: signingKey,
				microgonCollateral: authorization.microgonCollateral,
				micronotCollateral: authorization.micronotCollateral
			});
			previousSigningKey = signingKey;
		}

		if (request.token == argonToken) {
			// cast to u256 for overly cautious overflow protection, then scale down by the microgons-per-argonot floor
			// to get the equivalent Argonot collateral. NOTE: this might not be pragmatically possible to overflow, but
			// we want to be sure that the math is safe.
			uint256 authorizedArgonotAsMicrogons =
				(uint256(authorizedMicronotCollateral) * uint256(resolvedMicrogonsPerArgonot))
					/ (10 ** RUNTIME_DECIMALS);
			uint256 authorizedCollateralMicrogons =
				uint256(authorizedMicrogonCollateral) + authorizedArgonotAsMicrogons;
			if (authorizedCollateralMicrogons < request.amount) {
				revert InsufficientAuthorizedCollateral(
					authorizedCollateralMicrogons,
					request.amount
				);
			}
		} else {
			if (authorizedMicrogonCollateral != 0) revert InvalidMicrogonCollateralForArgonotPayout();
			if (authorizedMicronotCollateral < request.amount) {
				revert InsufficientAuthorizedCollateral(
					authorizedMicronotCollateral,
					request.amount
				);
			}
		}

		_setTransferOutOfArgonFinalized(transferId);
		ICanonicalToken(request.token).mint(
			request.recipient, toTokenAmount(request.amount)
		);

		uint64 activityNonce = _nextGatewayActivityNonce();
		emit TransferOutOfArgonFinalized(
			transferId,
			request.token,
			request.amount,
			mintingCollateral,
			_gatewayActivityState(activityNonce)
		);
	}

	/// @notice Cancels an expired transfer out of Argon.
	/// @dev Anyone can call this after the request has expired.
	/// @param request Signed transfer-out request.
	function cancelTransferOutOfArgon(
		TransferOutOfArgonRequest calldata request
	) external whenNotPaused {
		_requireTransferOutOfArgonChain(request);
		_requireCanonicalToken(request.token);
		if (request.amount == 0) revert ZeroAmount();

		if (block.number <= request.validUntilBlock) {
			revert TransferOutOfArgonNotExpired(block.number, request.validUntilBlock);
		}

		bytes32 transferId = _hashTransferOutOfArgonRequest(request);
		_requireTransferOutOfArgonNotFinalized(transferId);
		_setTransferOutOfArgonFinalized(transferId);

		uint64 activityNonce = _nextGatewayActivityNonce();
		emit TransferOutOfArgonCanceled(
			transferId,
			_gatewayActivityState(activityNonce)
		);
	}

	/// @notice Pauses the gateway.
	function pause() external {
		if (msg.sender != guardian && msg.sender != owner()) {
			revert NotGuardianOrOwner(msg.sender);
		}

		_pause();
	}

	/// @notice Sets the guardian.
	/// @param guardianAddress New guardian address.
	function setGuardian(address guardianAddress) external onlyOwner {
		if (guardianAddress == address(0)) revert ZeroGuardian();

		address previousGuardian = guardian;
		guardian = guardianAddress;

		emit GuardianUpdated(previousGuardian, guardianAddress);
	}

	/// @notice Converts runtime units into token base units.
	/// @param amount Runtime-decimal token amount.
	/// @return tokenAmount Token base-unit amount.
	function toTokenAmount(uint128 amount) public pure returns (uint256) {
		return uint256(amount) * RUNTIME_TO_ERC20_SCALE;
	}

	/// @notice Unpauses the gateway.
	function unpause() external onlyOwner {
		_unpause();
	}

	/// @notice Returns current Argon circulation in runtime units.
	/// @return circulation Runtime-decimal Argon circulation.
	function argonCirculation() public view returns (uint128) {
		return _toRuntimeAmount(ICanonicalToken(argonToken).totalSupply());
	}

	/// @notice Returns current Argonot circulation in runtime units.
	/// @return circulation Runtime-decimal Argonot circulation.
	function argonotCirculation() public view returns (uint128) {
		return _toRuntimeAmount(ICanonicalToken(argonotToken).totalSupply());
	}

	/// Applies one council rotation after quorum verification.
	function _applyGlobalIssuanceCouncilUpdate(
		GatewayUpdate calldata update,
		ActiveCouncilSnapshot memory activeCouncil,
		bytes32 previousUpdateHash,
		bytes32 relayerArgonAccountId
	)
		private
		returns (ActiveCouncilSnapshot memory nextCouncilState, bytes32 updateHash)
	{
		GlobalIssuanceCouncilRotateTarget memory target =
			abi.decode(update.payload, (GlobalIssuanceCouncilRotateTarget));
		if (target.epochMicrogonsPerArgonot == 0) revert ZeroMicrogonsPerArgonot();
		bytes32 nextCouncilHash;
		uint256 nextCouncilTotalWeight;
		uint256 nextCouncilMemberCount;

		(nextCouncilHash, nextCouncilTotalWeight, nextCouncilMemberCount) =
			_validateCouncilSnapshot(
				target.council.signers,
				target.council.weights,
				target.epochMicrogonsPerArgonot
			);
		updateHash = _hashRotateGlobalIssuanceCouncilApproval(
			update.queueNonce,
			activeCouncil.councilHash,
			target.council.signers,
			target.council.weights,
			target.epochMicrogonsPerArgonot,
			previousUpdateHash
		);
		_requireCouncilQuorum(
			updateHash,
			update.signatures,
			activeCouncil.signers,
			activeCouncil.weights,
			activeCouncil.totalWeight,
			activeCouncil.memberCount
		);

		_rollMicrogonsPerArgonot();
		_storeGlobalIssuanceCouncil(
			nextCouncilHash,
			nextCouncilTotalWeight,
			nextCouncilMemberCount,
			target.epochMicrogonsPerArgonot
		);
		argonApprovalsNonce = update.queueNonce;
		argonApprovalsHash = updateHash;

		uint64 activityNonce = _nextGatewayActivityNonce();
		emit GlobalIssuanceCouncilRotated(
			nextCouncilHash,
			relayerArgonAccountId,
			_gatewayActivityState(activityNonce)
		);

		return (
			ActiveCouncilSnapshot({
				signers: target.council.signers,
				weights: target.council.weights,
				councilHash: nextCouncilHash,
				totalWeight: nextCouncilTotalWeight,
				memberCount: nextCouncilMemberCount
			}),
			updateHash
		);
	}

	/// Applies one minting-authority activation after quorum verification.
	function _applyMintingAuthorityActivation(
		GatewayUpdate calldata update,
		ActiveCouncilSnapshot memory activeCouncil,
		bytes32 previousUpdateHash,
		bool requireCouncilVerification,
		uint32 coactivationCount,
		uint32 sharedSignatureCount,
		bytes32 relayerArgonAccountId
	) private returns (bytes32 updateHash) {
		MintingAuthorityActivationTarget memory target =
			abi.decode(update.payload, (MintingAuthorityActivationTarget));
		if (target.signingKey == address(0)) revert ZeroSigningKey();
		if (target.microgonCollateral == 0 && target.micronotCollateral == 0) {
			revert InvalidMintingAuthority(target.signingKey);
		}
		if (coactivationCount == 0 || sharedSignatureCount == 0) {
			revert InvalidMintingAuthority(target.signingKey);
		}
		MintingCollateral storage collateralRemaining =
			mintingAuthorityCollateralRemaining[target.signingKey];
		if (
			collateralRemaining.microgonCollateral != 0
				|| collateralRemaining.micronotCollateral != 0
		) {
			revert MintingAuthorityAlreadyActive(target.signingKey);
		}

		updateHash = _hashActivateMintingAuthorityApproval(
			update.queueNonce,
			activeCouncil.councilHash,
			target.microgonCollateral,
			target.micronotCollateral,
			target.signingKey,
			previousUpdateHash
		);
		if (requireCouncilVerification) {
			_requireCouncilQuorum(
				updateHash,
				update.signatures,
				activeCouncil.signers,
				activeCouncil.weights,
				activeCouncil.totalWeight,
				activeCouncil.memberCount
			);
		}

		mintingAuthorityCollateralRemaining[target.signingKey] = MintingCollateral({
			microgonCollateral: target.microgonCollateral,
			micronotCollateral: target.micronotCollateral
		});

		argonApprovalsNonce = update.queueNonce;
		argonApprovalsHash = updateHash;
		uint64 activityNonce = _nextGatewayActivityNonce();
		emit MintingAuthorityActivated(
			target.signingKey,
			target.microgonCollateral,
			target.micronotCollateral,
			coactivationCount,
			sharedSignatureCount,
			relayerArgonAccountId,
			_gatewayActivityState(activityNonce)
		);
	}

	/// Applies one minting-authority deactivation after verifying the authority signing key.
	function _applyMintingAuthorityDeactivate(
		GatewayUpdate calldata update,
		bytes32 previousUpdateHash,
		bytes32 relayerArgonAccountId
	) private returns (bytes32 updateHash) {
		MintingAuthorityDeactivateTarget memory target =
			abi.decode(update.payload, (MintingAuthorityDeactivateTarget));
		if (update.signatures.length != 1) {
			revert InvalidSignatureCount(1, update.signatures.length);
		}

		updateHash = _hashMintingAuthorityDeactivation(
			update.queueNonce, target.signingKey, previousUpdateHash
		);
		address recoveredSigner = ECDSA.recoverCalldata(
			updateHash.toEthSignedMessageHash(),
			update.signatures[0]
		);
		if (recoveredSigner != target.signingKey) {
			revert InvalidMintingAuthorityDeactivationSigner(
				target.signingKey, recoveredSigner
			);
		}
		MintingCollateral storage collateralRemaining =
			mintingAuthorityCollateralRemaining[target.signingKey];

		uint128 microgonCollateral = collateralRemaining.microgonCollateral;
		uint128 micronotCollateral = collateralRemaining.micronotCollateral;

		delete mintingAuthorityCollateralRemaining[target.signingKey];
		argonApprovalsNonce = update.queueNonce;
		argonApprovalsHash = updateHash;

		uint64 activityNonce = _nextGatewayActivityNonce();
		emit MintingAuthorityDeactivated(
			target.signingKey,
			microgonCollateral,
			micronotCollateral,
			relayerArgonAccountId,
			_gatewayActivityState(activityNonce)
		);
	}

	/// Counts valid council signatures and the weight they represent.
	function _countCouncilApprovals(
		bytes32 approvalHash,
		bytes[] memory signatures,
		address[] memory councilSigners,
		uint256[] memory councilWeights
	) private pure returns (uint256 signerCount, uint256 signerWeight) {
		address previousSigner = address(0);
		bytes32 signedApprovalHash = approvalHash.toEthSignedMessageHash();
		uint256 councilIndex = 0;

		for (uint256 index = 0; index < signatures.length; ++index) {
			address signer = ECDSA.recover(signedApprovalHash, signatures[index]);
			if (signer <= previousSigner) revert InvalidSignatureOrder();

			while (councilIndex < councilSigners.length && councilSigners[councilIndex] < signer) {
				++councilIndex;
			}
			if (councilIndex == councilSigners.length || councilSigners[councilIndex] != signer) {
				revert InvalidGlobalIssuanceCouncilMember(index);
			}

			++signerCount;
			signerWeight += councilWeights[councilIndex];
			previousSigner = signer;
			++councilIndex;
		}

		return (signerCount, signerWeight);
	}

	/// Mints one migration asset distribution using exact token base units.
	function _mintMigrationBalances(
		address token,
		address[] calldata recipients,
		uint256[] calldata amounts
	) private returns (uint256 totalAmount) {
		if (recipients.length != amounts.length) revert ArrayLengthMismatch();

		for (uint256 index = 0; index < recipients.length; ++index) {
			address recipient = recipients[index];
			uint256 amount = amounts[index];

			if (recipient == address(0)) revert ZeroRecipient(index);
			if (amount == 0) revert ZeroAmount();

			ICanonicalToken(token).mint(recipient, amount);
			totalAmount += amount;
		}
	}

	/// Checks whether the collected council signatures meet quorum.
	function _isCouncilQuorumMet(
		uint256 signerCount,
		uint256 signerWeight,
		uint256 totalEligibleWeight,
		uint256 totalEligibleMemberCount
	) private pure returns (bool) {
		uint256 unsignedMemberCount = totalEligibleMemberCount - signerCount;

		if (signerWeight * 100 >= totalEligibleWeight * 90) {
			return true;
		}

		return unsignedMemberCount <= 2 && signerWeight * 100 >= totalEligibleWeight * 80;
	}

	/// Builds the shared gateway snapshot emitted with an activity event.
	function _gatewayActivityState(
		uint64 activityNonce
	) private view returns (GatewayActivityState memory gatewayState) {
		gatewayState = GatewayActivityState({
			gatewayActivityNonce: activityNonce,
			argonApprovalsNonce: argonApprovalsNonce,
			argonCirculation: argonCirculation(),
			argonotCirculation: argonotCirculation()
		});
	}

	/// Increments the activity counter and records it for the current block.
	function _nextGatewayActivityNonce() private returns (uint64 nextNonce) {
		nextNonce = gatewayActivityNonce + 1;
		gatewayActivityNonce = nextNonce;
		_recordActivity(nextNonce);
	}

	/// Updates the locator entry for the current block.
	function _recordActivity(uint64 activityNonce) private {
		uint64 currentBlock = uint64(block.number);

		if (latestActivityBlockLocatorIndex == 0) {
			latestActivityBlockLocatorIndex = 1;
			activityBlockLocators[1] = ActivityBlockLocator({
				blockNumber: currentBlock,
				startGatewayActivityNonce: activityNonce,
				endGatewayActivityNonce: activityNonce
			});
			return;
		}

		ActivityBlockLocator storage latestLocator =
			activityBlockLocators[latestActivityBlockLocatorIndex];
		if (latestLocator.blockNumber == currentBlock) {
			// If this block already has a locator entry, just extend its end nonce.
			latestLocator.endGatewayActivityNonce = activityNonce;
			return;
		}

		uint64 nextLocatorIndex = latestActivityBlockLocatorIndex + 1;
		latestActivityBlockLocatorIndex = nextLocatorIndex;
		activityBlockLocators[nextLocatorIndex] = ActivityBlockLocator({
			blockNumber: currentBlock,
			startGatewayActivityNonce: activityNonce,
			endGatewayActivityNonce: activityNonce
		});
	}

	/// Reverts unless the token is managed by this gateway.
	function _requireCanonicalToken(address token) private view {
		if (token == address(0)) revert UnsupportedToken(token);
		if (argonToken == address(0) || argonotToken == address(0)) revert UnsupportedToken(token);
		if (token != argonToken && token != argonotToken) revert UnsupportedToken(token);
	}

	/// Reverts unless the provided council signatures meet the current quorum rule.
	function _requireCouncilQuorum(
		bytes32 approvalHash,
		bytes[] memory signatures,
		address[] memory councilSigners,
		uint256[] memory councilWeights,
		uint256 councilTotalWeight,
		uint256 councilMemberCount
	) private pure {
		(uint256 signerCount, uint256 signerWeight) =
			_countCouncilApprovals(approvalHash, signatures, councilSigners, councilWeights);
		if (
			!_isCouncilQuorumMet(
				signerCount,
				signerWeight,
				councilTotalWeight,
				councilMemberCount
			)
		) {
			revert GlobalIssuanceCouncilQuorumNotMet();
		}
	}

	/// Reverts unless the transfer-out request is for this chain.
	function _requireTransferOutOfArgonChain(TransferOutOfArgonRequest calldata request) private view {
		uint64 thisChainId = uint64(block.chainid);
		if (request.chainId != thisChainId) {
			revert InvalidChainId(thisChainId, request.chainId);
		}
	}

	/// Reverts if the transfer-out request was already finalized.
	function _requireTransferOutOfArgonNotFinalized(bytes32 transferId) private view {
		if (finalizedTransferOutOfArgonIds[transferId]) {
			revert TransferOutOfArgonAlreadyFinalized(transferId);
		}
	}

	/// Marks a transfer-out request as permanently finalized.
	function _setTransferOutOfArgonFinalized(bytes32 transferId) private {
		finalizedTransferOutOfArgonIds[transferId] = true;
	}

	/// Stores a new council snapshot after checking signer order and weight values.
	function _storeGlobalIssuanceCouncil(
		bytes32 councilHash,
		uint256 totalWeight,
		uint256 memberCount,
		uint128 epochMicrogonsPerArgonot
	) private {
		bytes32 priorCouncilHash = globalIssuanceCouncil.councilHash;
		previousGlobalIssuanceCouncilHash =
			priorCouncilHash == bytes32(0) ? councilHash : priorCouncilHash;
		globalIssuanceCouncil = GlobalIssuanceCouncil({
			totalWeight: totalWeight,
			memberCount: memberCount,
			councilHash: councilHash,
			epochMicrogonsPerArgonot: epochMicrogonsPerArgonot
		});
	}

	/// Stores the active floor price for the current and previous council windows.
	function _rollMicrogonsPerArgonot() private {
		previousMicrogonsPerArgonot = globalIssuanceCouncil.epochMicrogonsPerArgonot;
	}

	/// Validates a council snapshot and returns its summary values.
	function _validateCouncilSnapshot(
		address[] memory signers,
		uint256[] memory weights,
		uint128 epochMicrogonsPerArgonot
	) private pure returns (bytes32 councilHash, uint256 totalWeight, uint256 memberCount) {
		if (signers.length == 0) revert InvalidGlobalIssuanceCouncilMember(0);
		if (signers.length != weights.length) revert ArrayLengthMismatch();

		address previousSigner = address(0);

		for (uint256 index = 0; index < signers.length; ++index) {
			address signer = signers[index];
			uint256 weight = weights[index];

			if (signer == address(0) || weight == 0 || signer <= previousSigner) {
				revert InvalidGlobalIssuanceCouncilMember(index);
			}

			totalWeight += weight;
			previousSigner = signer;
		}

		memberCount = signers.length;
		councilHash = _hashGlobalIssuanceCouncil(signers, weights, epochMicrogonsPerArgonot);
	}

	/// Converts token base units back into runtime units.
	function _toRuntimeAmount(uint256 tokenAmount) private pure returns (uint128) {
		uint256 runtimeAmount = tokenAmount / RUNTIME_TO_ERC20_SCALE;
		if (runtimeAmount > type(uint128).max) revert RuntimeAmountOverflow(runtimeAmount);
		return uint128(runtimeAmount);
	}

	/// Resolves the council-window floor price for one transfer request.
	function _resolveMicrogonsPerArgonot(bytes32 councilHash) private view returns (uint128) {
		if (councilHash == globalIssuanceCouncil.councilHash) {
			return globalIssuanceCouncil.epochMicrogonsPerArgonot;
		}
		if (councilHash == previousGlobalIssuanceCouncilHash) {
			return previousMicrogonsPerArgonot;
		}

		revert UnknownCouncilHash(councilHash);
	}

	/// @notice Returns the hash for a full council snapshot.
	/// @param signers Sorted council signer addresses.
	/// @param weights Signer weights aligned with signers.
	/// @return councilHash Council membership hash.
	function _hashGlobalIssuanceCouncil(
		address[] memory signers,
		uint256[] memory weights,
		uint128 epochMicrogonsPerArgonot
	) private pure returns (bytes32) {
		return keccak256(abi.encode(signers, weights, epochMicrogonsPerArgonot));
	}

	function _signingKeyTargetId(address signingKey) private pure returns (bytes32) {
		return bytes32(uint256(uint160(signingKey)));
	}

	/// @notice Returns the hash for one minting authority activation target.
	/// @param microgonCollateral Argon-denominated collateral committed on activation.
	/// @param micronotCollateral Argonot-denominated collateral committed on activation.
	/// @param signingKey Signing key used on this chain for later transfer authorizations.
	/// @return mintingAuthorityHash Activation target hash.
	function _hashMintingAuthority(
		uint128 microgonCollateral,
		uint128 micronotCollateral,
		address signingKey
	) private pure returns (bytes32) {
		return keccak256(
			abi.encode(microgonCollateral, micronotCollateral, signingKey)
		);
	}

	/// @notice Returns the hash that council members sign for a minting-authority activation.
	/// @param microgonCollateral Argon-denominated collateral committed on activation.
	/// @param micronotCollateral Argonot-denominated collateral committed on activation.
	/// @param signingKey Signing key used on this chain for later transfer authorizations.
	/// @return activationHash Activation hash.
	function _hashActivateMintingAuthority(
		uint128 microgonCollateral,
		uint128 micronotCollateral,
		address signingKey
	) private view returns (bytes32) {
		return keccak256(
			abi.encode(
				_MINTING_AUTHORITY_ACTIVATION_TAG,
				block.chainid,
				address(this),
				_hashMintingAuthority(
					microgonCollateral, micronotCollateral, signingKey
				)
			)
		);
	}

	/// @notice Returns the hash that council members sign for a council rotation.
	/// @param nextSigners Sorted next-council signer addresses.
	/// @param nextWeights Signer weights aligned with nextSigners.
	/// @param nextEpochMicrogonsPerArgonot Council-approved floor for the next epoch, measured in microgons per 1 whole Argonot.
	/// @return rotationHash Rotation hash.
	function _hashRotateGlobalIssuanceCouncil(
		address[] memory nextSigners,
		uint256[] memory nextWeights,
		uint128 nextEpochMicrogonsPerArgonot
	) private view returns (bytes32) {
		return keccak256(
			abi.encode(
				_GLOBAL_ISSUANCE_COUNCIL_ROTATION_TAG,
				block.chainid,
				address(this),
				_hashGlobalIssuanceCouncil(nextSigners, nextWeights, nextEpochMicrogonsPerArgonot),
				nextEpochMicrogonsPerArgonot
			)
		);
	}

	/// @notice Returns the base hash used for one queued gateway update signature.
	/// @param queueNonce Contiguous queue nonce being applied.
	/// @param approvingCouncilHash Hash of the approving council membership snapshot.
	/// @param kind Update kind being approved.
	/// @param targetId Stable target identifier for the update.
	/// @param targetPayloadHash Hash of the update payload itself.
	/// @param previousUpdateHash Chained signed hash of the immediately previous queue item.
	/// @return approvalHash Queue approval hash.
	function _hashGatewayUpdateApproval(
		uint64 queueNonce,
		bytes32 approvingCouncilHash,
		GatewayUpdateKind kind,
		bytes32 targetId,
		bytes32 targetPayloadHash,
		bytes32 previousUpdateHash
	) private view returns (bytes32) {
		return keccak256(
			abi.encode(
				_GATEWAY_UPDATE_APPROVAL_TAG,
				block.chainid,
				address(this),
				queueNonce,
				approvingCouncilHash,
				uint8(kind),
				targetId,
				targetPayloadHash,
				previousUpdateHash
			)
		);
	}

	/// @notice Returns the queue-signing hash for a council rotation.
	/// @param queueNonce Contiguous queue nonce being applied.
	/// @param approvingCouncilHash Hash of the approving council membership snapshot.
	/// @param nextSigners Sorted next-council signer addresses.
	/// @param nextWeights Signer weights aligned with nextSigners.
	/// @param nextEpochMicrogonsPerArgonot Council-approved floor for the next epoch, measured in microgons per 1 whole Argonot.
	/// @param previousUpdateHash Chained signed hash of the immediately previous queue item.
	/// @return approvalHash Council approval hash.
	function _hashRotateGlobalIssuanceCouncilApproval(
		uint64 queueNonce,
		bytes32 approvingCouncilHash,
		address[] memory nextSigners,
		uint256[] memory nextWeights,
		uint128 nextEpochMicrogonsPerArgonot,
		bytes32 previousUpdateHash
	) private view returns (bytes32) {
		bytes32 nextCouncilHash =
			_hashGlobalIssuanceCouncil(nextSigners, nextWeights, nextEpochMicrogonsPerArgonot);

		return _hashGatewayUpdateApproval(
			queueNonce,
			approvingCouncilHash,
			GatewayUpdateKind.GlobalIssuanceCouncilRotate,
			nextCouncilHash,
			_hashRotateGlobalIssuanceCouncil(
				nextSigners, nextWeights, nextEpochMicrogonsPerArgonot
			),
			previousUpdateHash
		);
	}

	/// @notice Returns the queue-signing hash for a minting-authority activation.
	/// @param queueNonce Contiguous queue nonce being applied.
	/// @param approvingCouncilHash Hash of the approving council membership snapshot.
	/// @param microgonCollateral Argon-denominated collateral committed on activation.
	/// @param micronotCollateral Argonot-denominated collateral committed on activation.
	/// @param signingKey Signing key used on this chain for later transfer authorizations.
	/// @param previousUpdateHash Chained signed hash of the immediately previous queue item.
	/// @return approvalHash Council approval hash.
	function _hashActivateMintingAuthorityApproval(
		uint64 queueNonce,
		bytes32 approvingCouncilHash,
		uint128 microgonCollateral,
		uint128 micronotCollateral,
		address signingKey,
		bytes32 previousUpdateHash
	) private view returns (bytes32) {
		return _hashGatewayUpdateApproval(
			queueNonce,
			approvingCouncilHash,
			GatewayUpdateKind.MintingAuthorityActivate,
			_signingKeyTargetId(signingKey),
			_hashActivateMintingAuthority(microgonCollateral, micronotCollateral, signingKey),
			previousUpdateHash
		);
	}

	/// @notice Returns the hash that a minting authority signs to authorize its own deactivation.
	/// @param queueNonce Contiguous queue nonce being applied.
	/// @param signingKey Signing key that must authorize the deactivation on this chain.
	/// @param previousUpdateHash Chained signed hash of the immediately previous queue item.
	/// @return deactivateHash Deactivation authorization hash.
	function _hashMintingAuthorityDeactivation(
		uint64 queueNonce,
		address signingKey,
		bytes32 previousUpdateHash
	) private view returns (bytes32) {
		return keccak256(
			abi.encode(
				_MINTING_AUTHORITY_DEACTIVATION_TAG,
				block.chainid,
				address(this),
				queueNonce,
				signingKey,
				previousUpdateHash
			)
		);
	}

	/// @notice Returns the hash for a transfer-out request.
	/// @param request Signed transfer-out request.
	/// @return transferId Canonical transfer-out identifier.
	function _hashTransferOutOfArgonRequest(
		TransferOutOfArgonRequest calldata request
	) private pure returns (bytes32) {
		return keccak256(
			abi.encode(
				request.argonAccountId,
				request.argonTransferNonce,
				request.chainId,
				request.councilHash,
				request.recipient,
				request.validUntilBlock,
				request.token,
				request.amount,
				request.finalizationTip
			)
		);
	}

	/// @notice Returns the hash that a minting authority signs for a transfer-out.
	/// @param request Signed transfer-out request.
	/// @param microgonCollateral Argon-denominated collateral committed by the minting authority.
	/// @param micronotCollateral Argonot-denominated collateral committed by the minting authority.
	/// @return authorizationHash Transfer-out authorization hash.
	function _hashMintingAuthorization(
		TransferOutOfArgonRequest calldata request,
		uint128 microgonCollateral,
		uint128 micronotCollateral
	) private view returns (bytes32) {
		return keccak256(
			abi.encode(
				_TRANSFER_OUT_OF_ARGON_AUTHORIZATION_TAG,
				block.chainid,
				address(this),
				_hashTransferOutOfArgonRequest(request),
				microgonCollateral,
				micronotCollateral
			)
		);
	}
}
