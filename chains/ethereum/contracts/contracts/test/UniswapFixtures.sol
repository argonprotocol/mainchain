// SPDX-License-Identifier: MIT
pragma solidity ^0.8.24;

import {ERC20} from "@openzeppelin/contracts/token/ERC20/ERC20.sol";
import {IERC20Permit} from "@openzeppelin/contracts/token/ERC20/extensions/IERC20Permit.sol";
import {EIP712} from "@openzeppelin/contracts/utils/cryptography/EIP712.sol";
import {ECDSA} from "@openzeppelin/contracts/utils/cryptography/ECDSA.sol";
import {Nonces} from "@openzeppelin/contracts/utils/Nonces.sol";

contract TestArgon is ERC20 {
	constructor() ERC20("Test Argon", "ARGN") {
		_mint(msg.sender, 1_000_000e18);
	}
}

contract TestUSDC is ERC20, EIP712, Nonces, IERC20Permit {
	bytes32 private constant _PERMIT_TYPEHASH =
		keccak256("Permit(address owner,address spender,uint256 value,uint256 nonce,uint256 deadline)");
	/// EIP-712 domain version used to sign USDC permits for this fixture.
	string public constant version = "2";

	/// The permit deadline is earlier than the current block timestamp.
	error ERC2612ExpiredSignature(uint256 deadline);
	/// The signature does not recover the owner authorizing the allowance.
	error ERC2612InvalidSigner(address signer, address owner);
	error ReentrySucceeded();

	address public reentryTarget;
	bytes4 public reentryError;
	bytes private _reentryCallData;

	constructor() ERC20("Test USDC", "USDC") EIP712("Test USDC", "2") {
		_mint(msg.sender, 1_000_000e6);
	}

	function permit(address owner, address spender, uint256 value, uint256 deadline, uint8 v, bytes32 r, bytes32 s)
		external
	{
		if (block.timestamp > deadline) revert ERC2612ExpiredSignature(deadline);

		bytes32 structHash = keccak256(abi.encode(_PERMIT_TYPEHASH, owner, spender, value, _useNonce(owner), deadline));
		address signer = ECDSA.recover(_hashTypedDataV4(structHash), v, r, s);
		if (signer != owner) revert ERC2612InvalidSigner(signer, owner);

		_approve(owner, spender, value);
	}

	function nonces(address owner) public view override(IERC20Permit, Nonces) returns (uint256) {
		return super.nonces(owner);
	}

	function DOMAIN_SEPARATOR() external view returns (bytes32) {
		return _domainSeparatorV4();
	}

	function decimals() public pure override returns (uint8) {
		return 6;
	}

	function setReentrantCall(address target, bytes calldata callData) external {
		reentryTarget = target;
		_reentryCallData = callData;
		reentryError = bytes4(0);
	}

	function transferFrom(address from, address to, uint256 value) public override returns (bool) {
		if (msg.sender == reentryTarget && _reentryCallData.length != 0) {
			bytes memory callData = _reentryCallData;
			delete _reentryCallData;
			(bool success, bytes memory result) = reentryTarget.call(callData);
			if (success) revert ReentrySucceeded();
			if (result.length >= 4) reentryError = bytes4(result);
		}
		return super.transferFrom(from, to, value);
	}
}
