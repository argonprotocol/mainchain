// SPDX-License-Identifier: MIT
pragma solidity ^0.8.24;

import {ProxyAdmin as OpenZeppelinProxyAdmin} from "@openzeppelin/contracts/proxy/transparent/ProxyAdmin.sol";
import {TransparentUpgradeableProxy as OpenZeppelinTransparentUpgradeableProxy} from "@openzeppelin/contracts/proxy/transparent/TransparentUpgradeableProxy.sol";

/// @title ProxyAdmin
/// @author Argon Protocol
/// @notice Thin local export of OpenZeppelin's proxy admin contract for artifact generation.
contract ProxyAdmin is OpenZeppelinProxyAdmin {
    constructor(address initialOwner) OpenZeppelinProxyAdmin(initialOwner) {}
}

/// @title TransparentUpgradeableProxy
/// @author Argon Protocol
/// @notice Thin local export of OpenZeppelin's transparent proxy contract for artifact generation.
contract TransparentUpgradeableProxy is OpenZeppelinTransparentUpgradeableProxy {
    constructor(address logic, address initialOwner, bytes memory data)
        payable
        OpenZeppelinTransparentUpgradeableProxy(logic, initialOwner, data)
    {}
}
