// SPDX-License-Identifier: MIT
pragma solidity ^0.8.24;

import {Oracle} from "@uniswap/v3-core/contracts/libraries/Oracle.sol";
import {IPoolManager} from "@uniswap/v4-core/src/interfaces/IPoolManager.sol";
import {IHooks} from "@uniswap/v4-core/src/interfaces/IHooks.sol";
import {Hooks} from "@uniswap/v4-core/src/libraries/Hooks.sol";
import {StateLibrary} from "@uniswap/v4-core/src/libraries/StateLibrary.sol";
import {PoolKey} from "@uniswap/v4-core/src/types/PoolKey.sol";
import {PoolId} from "@uniswap/v4-core/src/types/PoolId.sol";
import {Currency} from "@uniswap/v4-core/src/types/Currency.sol";
import {ModifyLiquidityParams, SwapParams} from "@uniswap/v4-core/src/types/PoolOperation.sol";
import {BeforeSwapDelta, BeforeSwapDeltaLibrary} from "@uniswap/v4-core/src/types/BeforeSwapDelta.sol";

/// @notice Records cumulative tick and seconds-per-liquidity history for one Uniswap v4 pool.
/// @dev Stores observations in this hook, accepting updates only from its bound PoolManager and pool.
/// History starts at pool initialization and is updated before swaps and liquidity changes.
contract UniswapV4OracleHook {
    using StateLibrary for IPoolManager;
    using Oracle for Oracle.Observation[65535];

    uint160 public constant HOOK_FLAGS = Hooks.AFTER_INITIALIZE_FLAG | Hooks.BEFORE_ADD_LIQUIDITY_FLAG
        | Hooks.BEFORE_REMOVE_LIQUIDITY_FLAG | Hooks.BEFORE_SWAP_FLAG;

    /// @notice Retains at least one hour of history once warm, with at most one observation per second.
    uint16 public constant OBSERVATION_CAPACITY = 3601;

    IPoolManager public immutable poolManager;
    PoolId public immutable poolId;
    address public immutable token0;
    address public immutable token1;
    Oracle.Observation[65535] public observations;
    uint16 public observationIndex;
    uint16 public observationCardinality;

    error InvalidConfiguration();
    error InvalidHookAddress();
    error OnlyPoolManager();
    error WrongPool();
    error NotInitialized();
    error AlreadyInitialized();

    constructor(IPoolManager manager, address currency0, address currency1, uint24 fee, int24 tickSpacing) {
        if (address(manager) == address(0) || currency0 >= currency1) revert InvalidConfiguration();
        if (uint160(address(this)) & Hooks.ALL_HOOK_MASK != HOOK_FLAGS) revert InvalidHookAddress();

        poolManager = manager;
        token0 = currency0;
        token1 = currency1;
        poolId = PoolKey(Currency.wrap(currency0), Currency.wrap(currency1), fee, tickSpacing, IHooks(address(this))).toId();
    }

    modifier onlyPoolManager(PoolKey calldata key) {
        if (msg.sender != address(poolManager)) revert OnlyPoolManager();
        if (PoolId.unwrap(key.toId()) != PoolId.unwrap(poolId)) revert WrongPool();
        _;
    }

    function afterInitialize(address, PoolKey calldata key, uint160, int24)
        external onlyPoolManager(key) returns (bytes4)
    {
        if (observationCardinality != 0) revert AlreadyInitialized();
        (observationCardinality,) = observations.initialize(uint32(block.timestamp));
        return IHooks.afterInitialize.selector;
    }

    function beforeSwap(address, PoolKey calldata key, SwapParams calldata, bytes calldata)
        external onlyPoolManager(key) returns (bytes4, BeforeSwapDelta, uint24)
    {
        _writeObservation();
        return (IHooks.beforeSwap.selector, BeforeSwapDeltaLibrary.ZERO_DELTA, 0);
    }

    function beforeAddLiquidity(address, PoolKey calldata key, ModifyLiquidityParams calldata, bytes calldata)
        external onlyPoolManager(key) returns (bytes4)
    {
        _writeObservation();
        return IHooks.beforeAddLiquidity.selector;
    }

    function beforeRemoveLiquidity(address, PoolKey calldata key, ModifyLiquidityParams calldata, bytes calldata)
        external onlyPoolManager(key) returns (bytes4)
    {
        _writeObservation();
        return IHooks.beforeRemoveLiquidity.selector;
    }

    /// @notice Returns cumulative tick and seconds-per-liquidity values at the requested lookback times.
    /// @dev Uses the v3 observe ABI and extrapolates recent history from the pool's current state.
    function observe(uint32[] calldata secondsAgos)
        external view returns (int56[] memory tickCumulatives, uint160[] memory secondsPerLiquidityCumulativeX128s)
    {
        (, int24 tick,,) = poolManager.getSlot0(poolId);
        return observations.observe(
            uint32(block.timestamp), secondsAgos, tick, observationIndex,
            poolManager.getLiquidity(poolId), observationCardinality
        );
    }

    function _writeObservation() private {
        if (observationCardinality == 0) revert NotInitialized();
        (, int24 tick,,) = poolManager.getSlot0(poolId);
        (observationIndex, observationCardinality) = observations.write(
            observationIndex, uint32(block.timestamp), tick, poolManager.getLiquidity(poolId),
            observationCardinality, OBSERVATION_CAPACITY
        );
    }
}
