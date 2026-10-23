// SPDX-License-Identifier: MIT
pragma solidity ^0.8.24;

import {Create2} from "@openzeppelin/contracts/utils/Create2.sol";

contract OracleHookTestDeployer {
    function deploy(bytes32 salt, bytes memory initCode) external returns (address) {
        return Create2.deploy(0, salt, initCode);
    }
}
