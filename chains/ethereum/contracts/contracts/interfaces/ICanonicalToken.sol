// SPDX-License-Identifier: MIT
pragma solidity ^0.8.24;

import { IERC20 } from "@openzeppelin/contracts/token/ERC20/IERC20.sol";
import { IERC20Permit } from "@openzeppelin/contracts/token/ERC20/extensions/IERC20Permit.sol";

/// @title ICanonicalToken
/// @author Argon Protocol
/// @notice ERC-20 token interface used by the gateway for canonical mint and burn actions.
interface ICanonicalToken is IERC20, IERC20Permit {
	/// @notice Burns canonical tokens from an account after spending the gateway allowance.
	/// @param account Account whose tokens should be burned.
	/// @param amount Exact token base-unit amount to burn.
	function burnFrom(address account, uint256 amount) external;

	/// @notice Returns the token decimal precision.
	/// @return decimalsValue Token decimals.
	function decimals() external view returns (uint8);

	/// @notice Mints canonical tokens to a recipient.
	/// @param to Account that should receive the minted tokens.
	/// @param amount Exact token base-unit amount to mint.
	function mint(address to, uint256 amount) external;
}
