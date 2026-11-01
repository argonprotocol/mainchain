// SPDX-License-Identifier: MIT
pragma solidity ^0.8.24;

import {ERC20} from "@openzeppelin/contracts/token/ERC20/ERC20.sol";

contract TestArgon is ERC20 {
	constructor() ERC20("Test Argon", "ARGN") {
		_mint(msg.sender, 1_000_000e18);
	}
}

contract TestUSDC is ERC20 {
	error ReentrySucceeded();

	address public reentryTarget;
	bytes4 public reentryError;
	bytes private _reentryCallData;

	constructor() ERC20("Test USDC", "USDC") {
		_mint(msg.sender, 1_000_000e6);
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
