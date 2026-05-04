// SPDX-License-Identifier: MIT
pragma solidity ^0.8.24;

import { IERC20 } from "@openzeppelin/contracts/token/ERC20/IERC20.sol";

interface ICanonicalToken is IERC20 {
	function burnFrom(address account, uint256 amount) external;

	function decimals() external view returns (uint8);

	function mint(address to, uint256 amount) external;
}
