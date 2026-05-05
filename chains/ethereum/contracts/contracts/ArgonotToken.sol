// SPDX-License-Identifier: MIT
pragma solidity ^0.8.24;

import { CanonicalMintableBurnableERC20 } from "./CanonicalMintableBurnableERC20.sol";

contract ArgonotToken is CanonicalMintableBurnableERC20 {
	constructor(address gateway) CanonicalMintableBurnableERC20("Argonot", "ARGNOT", gateway) {}
}
