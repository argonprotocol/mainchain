// SPDX-License-Identifier: MIT
pragma solidity ^0.8.24;

import {ProxyAdmin as OpenZeppelinProxyAdmin} from "@openzeppelin/contracts/proxy/transparent/ProxyAdmin.sol";
import {TransparentUpgradeableProxy as OpenZeppelinTransparentUpgradeableProxy} from "@openzeppelin/contracts/proxy/transparent/TransparentUpgradeableProxy.sol";

contract ProxyAdmin is OpenZeppelinProxyAdmin {
    constructor(address initialOwner) OpenZeppelinProxyAdmin(initialOwner) {}
}

contract TransparentUpgradeableProxy is OpenZeppelinTransparentUpgradeableProxy {
    constructor(address logic, address initialOwner, bytes memory data)
        payable
        OpenZeppelinTransparentUpgradeableProxy(logic, initialOwner, data)
    {}
}
