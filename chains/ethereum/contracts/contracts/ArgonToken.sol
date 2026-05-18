// SPDX-License-Identifier: MIT
pragma solidity ^0.8.24;

import { CanonicalMintableBurnableERC20 } from "./CanonicalMintableBurnableERC20.sol";

/// @title ArgonToken
/// @author Argon Protocol
/// @notice Canonical ERC-20 representation of Argon on this chain.
contract ArgonToken is CanonicalMintableBurnableERC20 {
	constructor(address gateway) CanonicalMintableBurnableERC20("Argon", "ARGN", gateway) {}
}
