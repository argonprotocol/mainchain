// SPDX-License-Identifier: MIT
pragma solidity ^0.8.24;

import { ERC20 } from "@openzeppelin/contracts/token/ERC20/ERC20.sol";

abstract contract CanonicalMintableBurnableERC20 is ERC20 {
	error InvalidGateway(address gateway);
	error OnlyGateway(address caller);

	address public immutable gateway;

	constructor(
		string memory name_,
		string memory symbol_,
		address gatewayAddress
	) ERC20(name_, symbol_) {
		if (gatewayAddress == address(0) || gatewayAddress.code.length == 0) {
			revert InvalidGateway(gatewayAddress);
		}

		gateway = gatewayAddress;
	}

	modifier onlyGateway() {
		if (msg.sender != gateway) revert OnlyGateway(msg.sender);
		_;
	}

	function mint(address to, uint256 amount) external onlyGateway {
		_mint(to, amount);
	}

	function burnFrom(address account, uint256 amount) external onlyGateway {
		_spendAllowance(account, msg.sender, amount);
		_burn(account, amount);
	}
}
