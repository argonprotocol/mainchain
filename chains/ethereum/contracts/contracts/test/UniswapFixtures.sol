// SPDX-License-Identifier: MIT
pragma solidity ^0.8.24;

import {ERC20} from "@openzeppelin/contracts/token/ERC20/ERC20.sol";

contract TestUSDC is ERC20 {
	constructor() ERC20("Test USDC", "USDC") {
		_mint(msg.sender, 1_000_000e6);
	}

	function decimals() public pure override returns (uint8) {
		return 6;
	}
}
