// SPDX-License-Identifier: MIT
pragma solidity ^0.8.24;

import { ERC20 } from "@openzeppelin/contracts/token/ERC20/ERC20.sol";
import { ERC20Permit } from "@openzeppelin/contracts/token/ERC20/extensions/ERC20Permit.sol";

/// @title CanonicalMintableBurnableERC20
/// @author Argon Protocol
/// @notice Canonical ERC-20 token that only the gateway can mint or burn.
abstract contract CanonicalMintableBurnableERC20 is ERC20Permit {
	error InvalidGateway(address gateway);
	error OnlyGateway(address caller);

	/// @notice Gateway contract allowed to mint and burn this token.
	address public immutable gateway;

	constructor(
		string memory name_,
		string memory symbol_,
		address gatewayAddress
	) ERC20(name_, symbol_) ERC20Permit(name_) {
		if (gatewayAddress == address(0) || gatewayAddress.code.length == 0) {
			revert InvalidGateway(gatewayAddress);
		}

		gateway = gatewayAddress;
	}

	modifier onlyGateway() {
		if (msg.sender != gateway) revert OnlyGateway(msg.sender);
		_;
	}

	/// @notice Mints canonical tokens to a recipient.
	/// @param to Account that should receive the minted tokens.
	/// @param amount Exact token base-unit amount to mint.
	function mint(address to, uint256 amount) external onlyGateway {
		_mint(to, amount);
	}

	/// @notice Burns canonical tokens from an account after spending the gateway allowance.
	/// @param account Account whose tokens should be burned.
	/// @param amount Exact token base-unit amount to burn.
	function burnFrom(address account, uint256 amount) external onlyGateway {
		_spendAllowance(account, msg.sender, amount);
		_burn(account, amount);
	}
}
