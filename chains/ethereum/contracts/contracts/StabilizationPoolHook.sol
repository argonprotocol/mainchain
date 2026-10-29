// SPDX-License-Identifier: MIT
pragma solidity ^0.8.24;

import {Oracle} from "@uniswap/v3-core/contracts/libraries/Oracle.sol";
import {IPoolManager} from "@uniswap/v4-core/src/interfaces/IPoolManager.sol";
import {IHooks} from "@uniswap/v4-core/src/interfaces/IHooks.sol";
import {Hooks} from "@uniswap/v4-core/src/libraries/Hooks.sol";
import {LPFeeLibrary} from "@uniswap/v4-core/src/libraries/LPFeeLibrary.sol";
import {StateLibrary} from "@uniswap/v4-core/src/libraries/StateLibrary.sol";
import {PoolKey} from "@uniswap/v4-core/src/types/PoolKey.sol";
import {PoolId} from "@uniswap/v4-core/src/types/PoolId.sol";
import {Currency} from "@uniswap/v4-core/src/types/Currency.sol";
import {ModifyLiquidityParams, SwapParams} from "@uniswap/v4-core/src/types/PoolOperation.sol";
import {BeforeSwapDelta, BeforeSwapDeltaLibrary} from "@uniswap/v4-core/src/types/BeforeSwapDelta.sol";

/// @notice Maintains oracle history and charges LP fees of 10% for ARGN sales and 0.5% for buys.
/// @dev One immutable ARGN/USDC pool. Fees apply to all routers, senders and exact-input/output swaps.
contract StabilizationPoolHook {
    using StateLibrary for IPoolManager;
    using Oracle for Oracle.Observation[65535];

    /// @notice Required address permissions: initialization, liquidity changes and swaps.
    uint160 public constant HOOK_FLAGS = Hooks.AFTER_INITIALIZE_FLAG | Hooks.BEFORE_ADD_LIQUIDITY_FLAG
        | Hooks.BEFORE_REMOVE_LIQUIDITY_FLAG | Hooks.BEFORE_SWAP_FLAG;

    /// @notice Retains at least one hour of history once warm, at one observation per second.
    uint16 public constant OBSERVATION_CAPACITY = 3601;

    /// @notice LP fees use millionths of the input amount: 100,000 is 10%.
    uint24 public constant SELL_FEE = 100_000;
    /// @notice A 5,000 millionths LP fee is 0.5%.
    uint24 public constant BUY_FEE = 5_000;

    /// @notice Uniswap core contract owning this pool and authorized to invoke callbacks.
    IPoolManager public immutable poolManager;
    /// @notice ABI hash of this hook's immutable, dynamic-fee PoolKey.
    PoolId public immutable poolId;
    /// @notice Lower-sorted currency address, exposed through the price oracle's existing ABI.
    address public immutable token0;
    /// @notice Higher-sorted currency address, exposed through the price oracle's existing ABI.
    address public immutable token1;
    /// @notice Canonical ARGN token; determines the trade direction independently of token ordering.
    address public immutable argonToken;

    /// @notice Uniswap v3 Oracle ring buffer; stores tick and seconds-per-liquidity cumulatives.
    Oracle.Observation[65535] public observations;
    /// @notice Index of the most recently written observation in the ring buffer.
    uint16 public observationIndex;
    /// @notice Active ring-buffer length; can include uninitialized slots while history warms up.
    uint16 public observationCardinality;

    /// @notice The manager, token pair or tick spacing cannot identify a supported pool.
    error InvalidConfiguration();
    /// @notice The deployed address does not encode exactly the required hook permissions.
    error InvalidHookAddress();
    /// @notice A callback came from an account other than the bound PoolManager.
    error OnlyPoolManager();
    /// @notice A callback describes a PoolKey other than this hook's immutable pool.
    error WrongPool();
    /// @notice A swap or liquidity callback attempted to write before initialization created history.
    error NotInitialized();
    /// @notice The initialization callback attempted to replace existing oracle history.
    error AlreadyInitialized();

    constructor(IPoolManager manager, address argon, address usdc, int24 tickSpacing) {
        if (address(manager) == address(0)) revert InvalidConfiguration();
        if (argon == address(0)) revert InvalidConfiguration();
        if (usdc == address(0)) revert InvalidConfiguration();
        if (argon == usdc) revert InvalidConfiguration();
        if (tickSpacing <= 0) revert InvalidConfiguration();
        if (tickSpacing > 32_767) revert InvalidConfiguration();
        if ((uint160(address(this)) & Hooks.ALL_HOOK_MASK) != HOOK_FLAGS) revert InvalidHookAddress();

        bool argonIsToken0 = argon < usdc;
        poolManager = manager;
        token0 = argonIsToken0 ? argon : usdc;
        token1 = argonIsToken0 ? usdc : argon;
        argonToken = argon;
        poolId = PoolKey(
            Currency.wrap(token0), Currency.wrap(token1), LPFeeLibrary.DYNAMIC_FEE_FLAG,
            tickSpacing, IHooks(address(this))
        ).toId();
    }

    modifier onlyPoolManager(PoolKey calldata key) {
        if (msg.sender != address(poolManager)) revert OnlyPoolManager();
        if (PoolId.unwrap(key.toId()) != PoolId.unwrap(poolId)) revert WrongPool();
        _;
    }

    /// @notice Starts oracle history when the bound pool is initialized.
    function afterInitialize(address, PoolKey calldata key, uint160, int24)
        external onlyPoolManager(key) returns (bytes4)
    {
        if (observationCardinality != 0) revert AlreadyInitialized();
        (observationCardinality,) = observations.initialize(uint32(block.timestamp));
        return IHooks.afterInitialize.selector;
    }

    /// @notice Records the pre-swap observation, then selects the directional LP fee.
    function beforeSwap(address, PoolKey calldata key, SwapParams calldata params, bytes calldata)
        external onlyPoolManager(key) returns (bytes4, BeforeSwapDelta, uint24)
    {
        _writeObservation();

        bool sellingArgon = params.zeroForOne == (argonToken == token0);
        uint24 fee = sellingArgon ? SELL_FEE : BUY_FEE;
        return (IHooks.beforeSwap.selector, BeforeSwapDeltaLibrary.ZERO_DELTA, fee | LPFeeLibrary.OVERRIDE_FEE_FLAG);
    }

    /// @notice Records the preceding liquidity interval before an addition changes its denominator.
    function beforeAddLiquidity(address, PoolKey calldata key, ModifyLiquidityParams calldata, bytes calldata)
        external onlyPoolManager(key) returns (bytes4)
    {
        _writeObservation();
        return IHooks.beforeAddLiquidity.selector;
    }

    /// @notice Records the preceding liquidity interval before a removal changes its denominator.
    function beforeRemoveLiquidity(address, PoolKey calldata key, ModifyLiquidityParams calldata, bytes calldata)
        external onlyPoolManager(key) returns (bytes4)
    {
        _writeObservation();
        return IHooks.beforeRemoveLiquidity.selector;
    }

    /// @notice Returns cumulative tick and seconds-per-liquidity values at the requested lookbacks.
    /// @dev Uses the v3 observe ABI and extrapolates quiet intervals from the pool's current state.
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
