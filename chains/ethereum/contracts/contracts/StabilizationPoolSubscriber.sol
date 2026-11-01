// SPDX-License-Identifier: MIT
pragma solidity ^0.8.24;

import {ISubscriber} from "@uniswap/v4-periphery/src/interfaces/ISubscriber.sol";
import {PositionInfo} from "@uniswap/v4-periphery/src/libraries/PositionInfoLibrary.sol";
import {BalanceDelta} from "@uniswap/v4-core/src/types/BalanceDelta.sol";

/// @notice Lets Argon prove that a user-owned v4 position was maintained between checkpoints.
/// @dev Liquidity and range remain in Uniswap. Never delete or reset a subscription's revision.
contract StabilizationPoolSubscriber is ISubscriber {
    /// @notice The sole caller authorized to notify position changes.
    address public immutable positionManager;

    struct Subscription {
        /// @notice Raw AccountId32 bytes identifying the Argon beneficiary.
        bytes32 argonAccountId;
        /// @notice Permanent continuity counter; never cleared on unsubscribe or burn.
        uint256 revision;
    }

    /// @notice Beneficiary and permanent continuity marker; Uniswap retains the position itself.
    mapping(uint256 tokenId => Subscription) public subscriptions;

    /// @notice The configured PositionManager is zero or has no deployed code.
    error InvalidPositionManager();
    /// @notice A subscription callback was called by an address other than the fixed PositionManager.
    error OnlyPositionManager();
    /// @notice Enrollment supplied a zero Argon beneficiary account.
    error InvalidArgonAccount();

    constructor(address positionManagerAddress) {
        if (positionManagerAddress == address(0)) revert InvalidPositionManager();
        if (positionManagerAddress.code.length == 0) revert InvalidPositionManager();

        positionManager = positionManagerAddress;
    }

    modifier onlyPositionManager() {
        if (msg.sender != positionManager) revert OnlyPositionManager();
        _;
    }

    /// @inheritdoc ISubscriber
    /// @dev Enrollment data is abi.encode(argonAccountId).
    function notifySubscribe(uint256 tokenId, bytes calldata data) external onlyPositionManager {
        bytes32 argonAccountId = abi.decode(data, (bytes32));
        if (argonAccountId == bytes32(0)) revert InvalidArgonAccount();

        Subscription storage subscription = subscriptions[tokenId];
        subscription.argonAccountId = argonAccountId;
        ++subscription.revision;
    }

    /// @inheritdoc ISubscriber
    function notifyModifyLiquidity(uint256 tokenId, int256 liquidityChange, BalanceDelta)
        external
        onlyPositionManager
    {
        if (liquidityChange != 0) {
            ++subscriptions[tokenId].revision;
        }
    }

    /// @inheritdoc ISubscriber
    function notifyBurn(uint256 tokenId, address, PositionInfo, uint256, BalanceDelta)
        external
        onlyPositionManager
    {
        ++subscriptions[tokenId].revision;
    }

    /// @inheritdoc ISubscriber
    /// @dev This notification is best effort. Argon must also prove PositionManager.subscriber(tokenId).
    function notifyUnsubscribe(uint256) external onlyPositionManager {
        return;
    }
}
