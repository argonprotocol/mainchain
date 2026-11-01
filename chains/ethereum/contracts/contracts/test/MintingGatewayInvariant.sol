// SPDX-License-Identifier: MIT
pragma solidity ^0.8.24;

import {MintingGateway} from "../MintingGateway.sol";
import {ArgonToken} from "../ArgonToken.sol";
import {ArgonotToken} from "../ArgonotToken.sol";
import {ICanonicalToken} from "../interfaces/ICanonicalToken.sol";
import {TransparentUpgradeableProxy} from "../ProxyArtifacts.sol";

/// A distinct caller exercises permission checks without impersonating the owner or gateway.
contract GatewayUnprivilegedCaller {
	function callContract(address destination, bytes calldata data) external returns (bool) {
		(bool success,) = destination.call(data);
		return success;
	}
}

/// Echidna generates sequences of real proxy, token, burn, pause and cancellation calls.
/// Signed council updates, transfer finalization, purchases and the oracle need separate campaigns.
contract MintingGatewayInvariant {
	uint256 private constant _INITIAL_SUPPLY = 1e30;
	uint256 private constant _SCALE = 1e12;
	MintingGateway private immutable _gateway;
	ArgonToken private immutable _argon;
	ArgonotToken private immutable _argonot;
	GatewayUnprivilegedCaller private immutable _unprivileged;
	uint256 private _burned;
	uint64 private _activities;
	bool private _paused;
	bool private _violated;
	mapping(bytes32 => bool) private _canceled;

	constructor() {
		// These are this contract's third and fourth CREATEs. The proxy must exist before
		// canonical tokens validate its code; the implementation binds their future addresses.
		address argonAddress = address(uint160(uint256(keccak256(
			abi.encodePacked(hex"d694", address(this), hex"03")
		))));
		address argonotAddress = address(uint160(uint256(keccak256(
			abi.encodePacked(hex"d694", address(this), hex"04")
		))));
		MintingGateway implementation = new MintingGateway(argonAddress, argonotAddress, address(this));
		_gateway = MintingGateway(address(new TransparentUpgradeableProxy(
			address(implementation),
			address(this),
			abi.encodeCall(MintingGateway.initialize, (
				address(this), address(this), keccak256("invariant council"), 1, 1, 1e6
			))
		)));
		_argon = new ArgonToken(address(_gateway));
		_argonot = new ArgonotToken(address(_gateway));
		require(address(_argon) == argonAddress && address(_argonot) == argonotAddress);
		_unprivileged = new GatewayUnprivilegedCaller();

		address[] memory recipients = new address[](1);
		recipients[0] = address(this);
		uint256[] memory amounts = new uint256[](1);
		amounts[0] = _INITIAL_SUPPLY;
		MintingGateway.MigrationAssetDistribution memory distribution =
			MintingGateway.MigrationAssetDistribution(recipients, amounts);
		_gateway.migrate(distribution, distribution);
		_gateway.setMinimumSwapMicrogons(1);
		_argon.approve(address(_gateway), type(uint256).max);
	}

	function approveBurn(uint256 allowance) external {
		_argon.approve(address(_gateway), allowance);
	}

	function setPaused(bool shouldPause) external {
		if (_paused == shouldPause) return;
		if (shouldPause) {
			_gateway.pause();
		} else {
			_gateway.unpause();
		}
		_paused = shouldPause;
	}

	function burnPurchase(uint128 microgons, uint64 dust) external {
		uint256 tokenAmount = uint256(microgons) * _SCALE;
		uint256 allowance = _argon.allowance(address(this), address(_gateway));
		bool shouldSucceed = !_paused && microgons != 0 && tokenAmount <= _INITIAL_SUPPLY - _burned
			&& allowance >= tokenAmount && !_activityBlockFull();
		MintingGateway.SwapPurchase memory purchase;
		purchase.buyer = msg.sender;
		purchase.argonAccountId = bytes32(uint256(1));
		purchase.argonBought = tokenAmount + uint256(dust) % _SCALE;

		try _gateway.burnFromSwap(purchase) returns (uint128 amount, uint64 nonce) {
			if (!shouldSucceed) {
				_violated = true;
				return;
			}
			if (amount != microgons || nonce != _activities + 1) _violated = true;
			_burned += tokenAmount;
			++_activities;
			uint256 expectedAllowance = allowance;
			if (allowance != type(uint256).max) expectedAllowance -= tokenAmount;
			if (_argon.allowance(address(this), address(_gateway)) != expectedAllowance) _violated = true;
		} catch {
			if (shouldSucceed) _violated = true;
		}
	}

	function cancelExpiredTransfer(uint8 requestNonce, uint128 amount, bool ownershipToken) external {
		MintingGateway.TransferOutOfArgonRequest memory request;
		request.argonAccountId = bytes32(uint256(1));
		request.argonTransferNonce = requestNonce;
		request.chainId = uint64(block.chainid);
		request.microgonsPerArgonot = 1e6;
		request.recipient = address(this);
		request.token = address(_argon);
		if (ownershipToken) request.token = address(_argonot);
		request.amount = amount;
		bytes32 transferId = keccak256(abi.encode(
			request.argonAccountId, request.argonTransferNonce, request.chainId,
			request.microgonsPerArgonot, request.recipient, request.validUntilBlock,
			request.token, request.amount, request.mintingAuthorityTip
		));
		bool shouldSucceed = !_paused && amount != 0 && block.number > 0
			&& !_canceled[transferId] && !_activityBlockFull();
		try _gateway.cancelTransferOutOfArgon(request) {
			if (!shouldSucceed || !_gateway.finalizedTransferOutOfArgonIds(transferId)) _violated = true;
			_canceled[transferId] = true;
			++_activities;
		} catch {
			if (shouldSucceed) _violated = true;
		}
	}

	function retryMigration() external {
		MintingGateway.MigrationAssetDistribution memory empty;
		empty.recipients = new address[](0);
		empty.amounts = new uint256[](0);
		try _gateway.migrate(empty, empty) {
			_violated = true;
		} catch {}
	}

	function attemptUnauthorizedAction(uint8 action, uint128 amount) external {
		bytes memory data;
		address destination = address(_gateway);
		uint8 operation = action % 5;
		if (operation == 0) {
			destination = address(_argon);
			data = abi.encodeCall(ICanonicalToken.mint, (address(this), uint256(amount)));
		} else if (operation == 1) {
			data = abi.encodeCall(MintingGateway.pause, ());
		} else if (operation == 2) {
			data = abi.encodeCall(MintingGateway.unpause, ());
		} else if (operation == 3) {
			data = abi.encodeCall(MintingGateway.setGuardian, (address(_unprivileged)));
		} else {
			data = abi.encodeCall(MintingGateway.setMinimumSwapMicrogons, (amount));
		}
		if (_unprivileged.callContract(destination, data)) _violated = true;
	}

	function echidna_supply_matches_burns() external view returns (bool) {
		return _argon.totalSupply() == _INITIAL_SUPPLY - _burned
			&& _argon.balanceOf(address(this)) == _INITIAL_SUPPLY - _burned
			&& _argonot.totalSupply() == _INITIAL_SUPPLY
			&& _argonot.balanceOf(address(this)) == _INITIAL_SUPPLY;
	}

	function echidna_gateway_authority_is_preserved() external view returns (bool) {
		return _gateway.owner() == address(this) && _gateway.guardian() == address(this)
			&& _gateway.paused() == _paused && _gateway.migrationCompleted()
			&& _gateway.argonToken() == address(_argon) && _gateway.argonotToken() == address(_argonot)
			&& _argon.gateway() == address(_gateway) && _argonot.gateway() == address(_gateway)
			&& _gateway.argonApprovalsNonce() == 0 && _gateway.argonApprovalsHash() == bytes32(0);
	}

	function echidna_activity_nonce_matches_successes() external view returns (bool) {
		if (_gateway.gatewayActivityNonce() != _activities) return false;
		uint64 index = _gateway.latestActivityBlockLocatorIndex();
		if (_activities == 0) return index == 0;
		if (index == 0 || index > _activities) return false;
		(, uint64 start, uint64 end, bytes32 root) = _gateway.activityBlockLocators(index);
		return start != 0 && start <= end && end == _activities && root != bytes32(0);
	}

	function echidna_transitions_respect_permissions_and_replay() external view returns (bool) {
		return !_violated;
	}

	function _activityBlockFull() private view returns (bool) {
		uint64 index = _gateway.latestActivityBlockLocatorIndex();
		if (index == 0) return false;
		(uint64 number, uint64 start, uint64 end,) = _gateway.activityBlockLocators(index);
		return number == block.number && end - start + 1 >= _gateway.MAX_GATEWAY_ACTIVITIES_PER_BLOCK();
	}
}
