// SPDX-License-Identifier: MIT
pragma solidity ^0.8.24;

import {Create2} from "@openzeppelin/contracts/utils/Create2.sol";
import {PoolManager} from "@uniswap/v4-core/src/PoolManager.sol";
import {IPoolManager} from "@uniswap/v4-core/src/interfaces/IPoolManager.sol";
import {IHooks} from "@uniswap/v4-core/src/interfaces/IHooks.sol";
import {Hooks} from "@uniswap/v4-core/src/libraries/Hooks.sol";
import {LPFeeLibrary} from "@uniswap/v4-core/src/libraries/LPFeeLibrary.sol";
import {FullMath} from "@uniswap/v4-core/src/libraries/FullMath.sol";
import {StateLibrary} from "@uniswap/v4-core/src/libraries/StateLibrary.sol";
import {TickMath} from "@uniswap/v4-core/src/libraries/TickMath.sol";
import {Currency} from "@uniswap/v4-core/src/types/Currency.sol";
import {BalanceDelta} from "@uniswap/v4-core/src/types/BalanceDelta.sol";
import {PoolKey} from "@uniswap/v4-core/src/types/PoolKey.sol";
import {ModifyLiquidityParams, SwapParams} from "@uniswap/v4-core/src/types/PoolOperation.sol";
import {PoolModifyLiquidityTest} from "@uniswap/v4-core/src/test/PoolModifyLiquidityTest.sol";
import {PoolSwapTest} from "@uniswap/v4-core/src/test/PoolSwapTest.sol";
import {TestERC20} from "@uniswap/v4-core/src/test/TestERC20.sol";
import {StabilizationPoolHook} from "../StabilizationPoolHook.sol";
import {OracleHookTestDeployer} from "./OracleHookTestDeployer.sol";

interface IEchidnaVm {
	function warp(uint256 timestamp) external;
}

/// Echidna drives the real dynamic-fee PoolManager, settlement routers and stabilization hook.
/// Oracle expectations are calculated independently; applied fees are checked against core fee growth.
contract StabilizationPoolHookInvariant {
	using StateLibrary for IPoolManager;

	enum TransitionFailure {
		None,
		LiquidityMismatch,
		LiquidityReverted,
		SwapDidNotMove,
		SwapReverted
	}

	int24 private constant _TICK_SPACING = 10;
	int24 private constant _POSITION_LOWER_TICK = -1000;
	int24 private constant _POSITION_UPPER_TICK = 1000;
	int24 private constant _SWAP_LOWER_LIMIT = -900;
	int24 private constant _SWAP_UPPER_LIMIT = 900;
	uint24 private constant _EXPECTED_SELL_FEE = 100_000;
	uint24 private constant _EXPECTED_BUY_FEE = 5_000;
	// Covers per-step fee rounding and reconstruction from the floored fee-growth accumulator.
	uint256 private constant _FEE_ROUNDING_TOLERANCE = 4;
	uint128 private constant _MIN_POSITION_LIQUIDITY = 1 << 32;
	uint128 private constant _INITIAL_LIQUIDITY = 1 << 64;
	uint256 private constant _Q128 = 1 << 128;
	IEchidnaVm private constant _VM =
		IEchidnaVm(address(uint160(uint256(keccak256("hevm cheat code")))));

	IPoolManager public immutable poolManager;
	PoolModifyLiquidityTest public immutable liquidityRouter;
	PoolSwapTest public immutable swapRouter;
	TestERC20 public immutable token0;
	TestERC20 public immutable token1;
	TestERC20 public immutable argonToken;
	TestERC20 public immutable usdcToken;
	StabilizationPoolHook public immutable hook;

	uint64 public successfulArgonSales;
	uint64 public successfulArgonBuys;
	uint64 public successfulExactInputs;
	uint64 public successfulExactOutputs;
	uint64 public successfulLiquidityAdds;
	uint64 public successfulLiquidityRemovals;
	uint64 public successfulTimeAdvances;
	uint64 public generatedSellExactInputs;
	uint64 public generatedSellExactOutputs;
	uint64 public generatedBuyExactInputs;
	uint64 public generatedBuyExactOutputs;
	TransitionFailure public transitionFailure;
	uint8 public feeFailure;

	PoolKey private _key;
	uint128 private _positionLiquidity;
	uint128 private _activePositionLiquidity;
	bytes32 private _activePositionSalt;
	uint64 private _nextPositionSalt;
	uint32 private _modelTimestamp;
	int56 private _modelTickCumulative;
	uint160 private _modelSecondsPerLiquidityCumulativeX128;
	int24 private _modelTick;
	uint128 private _modelLiquidity;
	bool private _bootstrapping;
	bool private _hasPreviousCheckpoint;
	uint32 private _previousTimestamp;
	int56 private _previousTickCumulative;
	uint160 private _previousSecondsPerLiquidityCumulativeX128;

	constructor() {
		PoolManager manager = new PoolManager(address(this));
		poolManager = manager;
		liquidityRouter = new PoolModifyLiquidityTest(manager);
		swapRouter = new PoolSwapTest(manager);

		TestERC20 firstToken = new TestERC20(1 << 120);
		TestERC20 secondToken = new TestERC20(1 << 120);
		if (address(firstToken) < address(secondToken)) {
			token0 = firstToken;
			token1 = secondToken;
		} else {
			token0 = secondToken;
			token1 = firstToken;
		}
		argonToken = token1;
		usdcToken = token0;

		token0.approve(address(liquidityRouter), type(uint256).max);
		token1.approve(address(liquidityRouter), type(uint256).max);
		token0.approve(address(swapRouter), type(uint256).max);
		token1.approve(address(swapRouter), type(uint256).max);

		hook = _deployHook(manager);
		_key = PoolKey({
			currency0: Currency.wrap(address(token0)),
			currency1: Currency.wrap(address(token1)),
			fee: LPFeeLibrary.DYNAMIC_FEE_FLAG,
			tickSpacing: _TICK_SPACING,
			hooks: IHooks(address(hook))
		});
		manager.initialize(_key, 1 << 96);

		_modelTimestamp = uint32(block.timestamp);
		liquidityRouter.modifyLiquidity(
			_key,
			ModifyLiquidityParams({
				tickLower: _POSITION_LOWER_TICK,
				tickUpper: _POSITION_UPPER_TICK,
				liquidityDelta: int256(uint256(_INITIAL_LIQUIDITY)),
				salt: bytes32(0)
			}),
			bytes("")
		);
		_positionLiquidity = _INITIAL_LIQUIDITY;
		_activePositionLiquidity = _INITIAL_LIQUIDITY;
		successfulLiquidityAdds = 1;
		_refreshPoolState();
		require(_historyMatchesModel(), "initial stabilization history mismatch");

		_bootstrapping = true;
		_executeSwap(true, true, 0);
		_executeSwap(false, true, 0);
		_executeSwap(true, false, 0);
		_executeSwap(false, false, 0);
		_bootstrapping = false;
	}

	function changeLiquidity(bool add, bool removeAll, uint48 requested) external {
		if (!add && _activePositionLiquidity == 0) return;

		uint128 magnitude;
		bytes32 positionSalt = _activePositionSalt;
		if (add) {
			magnitude = _MIN_POSITION_LIQUIDITY + uint128(uint256(requested) % (1 << 48));
			positionSalt = bytes32(uint256(_nextPositionSalt) + 1);
		} else if (removeAll || _activePositionLiquidity < 2 * _MIN_POSITION_LIQUIDITY) {
			magnitude = _activePositionLiquidity;
		} else {
			magnitude = _MIN_POSITION_LIQUIDITY
				+ uint128(uint256(requested) % (_activePositionLiquidity - 2 * _MIN_POSITION_LIQUIDITY + 1));
		}
		int256 liquidityDelta = int256(uint256(magnitude));
		if (!add) liquidityDelta = -liquidityDelta;

		_syncModel();
		try liquidityRouter.modifyLiquidity(
			_key,
			ModifyLiquidityParams({
				tickLower: _POSITION_LOWER_TICK,
				tickUpper: _POSITION_UPPER_TICK,
				liquidityDelta: liquidityDelta,
				salt: positionSalt
			}),
			bytes("")
		) returns (BalanceDelta) {
			if (add) {
				_positionLiquidity += magnitude;
				_activePositionLiquidity = magnitude;
				_activePositionSalt = positionSalt;
				++_nextPositionSalt;
				++successfulLiquidityAdds;
			} else {
				_positionLiquidity -= magnitude;
				_activePositionLiquidity -= magnitude;
				++successfulLiquidityRemovals;
			}
			_refreshPoolState();
			if (_modelLiquidity != _positionLiquidity) transitionFailure = TransitionFailure.LiquidityMismatch;
		} catch {
			transitionFailure = TransitionFailure.LiquidityReverted;
		}
	}

	function swap(bool sellingArgon, bool exactInput, uint56 requested) external {
		_executeSwap(sellingArgon, exactInput, requested);
	}

	function advanceTime(uint16 requested) external {
		_VM.warp(block.timestamp + uint256(requested) % 300 + 1);
		++successfulTimeAdvances;
	}

	function echidna_observations_match_independent_history() external view returns (bool) {
		return _historyMatchesModel();
	}

	function echidna_pool_state_matches_successful_liquidity() external view returns (bool) {
		(, int24 tick,,) = poolManager.getSlot0(hook.poolId());
		return tick == _modelTick && poolManager.getLiquidity(hook.poolId()) == _modelLiquidity
			&& _modelLiquidity == _positionLiquidity;
	}

	function echidna_expected_transitions_do_not_revert() external view returns (bool) {
		return transitionFailure == TransitionFailure.None;
	}

	function echidna_directional_fees_match_argon_flow() external view returns (bool) {
		return feeFailure == 0;
	}

	function echidna_bootstrap_exercises_all_swap_modes() external view returns (bool) {
		return successfulArgonSales > 0 && successfulArgonBuys > 0 && successfulExactInputs > 0
			&& successfulExactOutputs > 0;
	}

	function _executeSwap(bool sellingArgon, bool exactInput, uint56 requested) private {
		if (_modelLiquidity == 0) return;

		bool zeroForOne = sellingArgon == (address(argonToken) == address(token0));
		if (zeroForOne && _modelTick <= _SWAP_LOWER_LIMIT) return;
		if (!zeroForOne && _modelTick >= _SWAP_UPPER_LIMIT) return;

		_syncModel();
		int24 tickBefore = _modelTick;
		uint256 amount = uint256(requested) % (1 << 52) + (1 << 52);
		int256 amountSpecified = int256(amount);
		if (exactInput) amountSpecified = -amountSpecified;
		uint160 priceLimit = TickMath.getSqrtPriceAtTick(
			zeroForOne ? _SWAP_LOWER_LIMIT : _SWAP_UPPER_LIMIT
		);
		uint24 expectedFee = sellingArgon ? _EXPECTED_SELL_FEE : _EXPECTED_BUY_FEE;
		uint256 feeGrowthBefore = _inputFeeGrowth(zeroForOne);

		try swapRouter.swap(
			_key,
			SwapParams({
				zeroForOne: zeroForOne,
				amountSpecified: amountSpecified,
				sqrtPriceLimitX96: priceLimit
			}),
			PoolSwapTest.TestSettings({takeClaims: false, settleUsingBurn: false}),
			bytes("")
		) returns (BalanceDelta delta) {
			if (!_feeGrowthMatches(zeroForOne, delta, expectedFee, feeGrowthBefore)) feeFailure = 1;
			if (!_bootstrapping) {
				if (sellingArgon && exactInput) ++generatedSellExactInputs;
				if (sellingArgon && !exactInput) ++generatedSellExactOutputs;
				if (!sellingArgon && exactInput) ++generatedBuyExactInputs;
				if (!sellingArgon && !exactInput) ++generatedBuyExactOutputs;
			}

			if (sellingArgon) ++successfulArgonSales;
			else ++successfulArgonBuys;
			if (exactInput) ++successfulExactInputs;
			else ++successfulExactOutputs;

			_refreshPoolState();
			if (_modelTick == tickBefore) transitionFailure = TransitionFailure.SwapDidNotMove;
		} catch {
			transitionFailure = TransitionFailure.SwapReverted;
		}
	}

	function _deployHook(IPoolManager manager) private returns (StabilizationPoolHook deployedHook) {
		OracleHookTestDeployer deployer = new OracleHookTestDeployer();
		bytes memory initCode = abi.encodePacked(
			type(StabilizationPoolHook).creationCode,
			abi.encode(manager, address(argonToken), address(usdcToken), _TICK_SPACING)
		);
		bytes32 initCodeHash = keccak256(initCode);
		uint160 flags = Hooks.AFTER_INITIALIZE_FLAG | Hooks.BEFORE_ADD_LIQUIDITY_FLAG
			| Hooks.BEFORE_REMOVE_LIQUIDITY_FLAG | Hooks.BEFORE_SWAP_FLAG;
		uint256 salt;
		address predicted;
		do {
			predicted = Create2.computeAddress(bytes32(salt), initCodeHash, address(deployer));
			++salt;
		} while (uint160(predicted) & Hooks.ALL_HOOK_MASK != flags);
		deployedHook = StabilizationPoolHook(deployer.deploy(bytes32(salt - 1), initCode));
	}

	function _feeGrowthMatches(bool zeroForOne, BalanceDelta delta, uint24 fee, uint256 feeGrowthBefore)
		private view returns (bool)
	{
		int128 inputDelta;
		if (zeroForOne) inputDelta = delta.amount0();
		else inputDelta = delta.amount1();

		uint256 totalInput = uint128(-inputDelta);
		uint256 expectedFeeAmount = FullMath.mulDiv(totalInput, fee, 1_000_000);
		uint256 minimumFeeAmount = expectedFeeAmount - _FEE_ROUNDING_TOLERANCE;
		uint256 maximumFeeAmount = expectedFeeAmount + _FEE_ROUNDING_TOLERANCE;
		uint256 minimumFeeGrowth = (minimumFeeAmount * _Q128) / _modelLiquidity;
		uint256 maximumFeeGrowth = (maximumFeeAmount * _Q128) / _modelLiquidity;
		uint256 feeGrowthAfter = _inputFeeGrowth(zeroForOne);
		unchecked {
			uint256 actualFeeGrowth = feeGrowthAfter - feeGrowthBefore;
			return actualFeeGrowth >= minimumFeeGrowth && actualFeeGrowth <= maximumFeeGrowth;
		}
	}

	function _inputFeeGrowth(bool zeroForOne) private view returns (uint256) {
		(uint256 feeGrowth0, uint256 feeGrowth1) = poolManager.getFeeGrowthGlobals(hook.poolId());
		if (zeroForOne) return feeGrowth0;

		return feeGrowth1;
	}

	function _syncModel() private {
		uint32 timestamp = uint32(block.timestamp);
		if (timestamp == _modelTimestamp) return;

		_hasPreviousCheckpoint = true;
		_previousTimestamp = _modelTimestamp;
		_previousTickCumulative = _modelTickCumulative;
		_previousSecondsPerLiquidityCumulativeX128 = _modelSecondsPerLiquidityCumulativeX128;
		(_modelTickCumulative, _modelSecondsPerLiquidityCumulativeX128) = _project(
			_modelTickCumulative,
			_modelSecondsPerLiquidityCumulativeX128,
			_elapsed(_modelTimestamp, timestamp),
			_modelTick,
			_modelLiquidity
		);
		_modelTimestamp = timestamp;
	}

	function _refreshPoolState() private {
		(, _modelTick,,) = poolManager.getSlot0(hook.poolId());
		_modelLiquidity = poolManager.getLiquidity(hook.poolId());
	}

	function _historyMatchesModel() private view returns (bool) {
		uint32 timestamp = uint32(block.timestamp);
		uint32 sinceModel = _elapsed(_modelTimestamp, timestamp);
		(int56 currentTickCumulative, uint160 currentSecondsPerLiquidityCumulativeX128) = _project(
			_modelTickCumulative,
			_modelSecondsPerLiquidityCumulativeX128,
			sinceModel,
			_modelTick,
			_modelLiquidity
		);

		(uint32 observedTimestamp, int56 observedTickCumulative, uint160 observedSeconds, bool initialized) =
			hook.observations(hook.observationIndex());
		if (
			!initialized || observedTimestamp != _modelTimestamp
				|| observedTickCumulative != _modelTickCumulative
				|| observedSeconds != _modelSecondsPerLiquidityCumulativeX128
		) return false;

		uint32[] memory secondsAgos = new uint32[](_hasPreviousCheckpoint ? 3 : 2);
		uint256 modelIndex;
		if (_hasPreviousCheckpoint) {
			secondsAgos[0] = _elapsed(_previousTimestamp, timestamp);
			modelIndex = 1;
		}
		secondsAgos[modelIndex] = sinceModel;
		secondsAgos[modelIndex + 1] = 0;

		try hook.observe(secondsAgos) returns (int56[] memory ticks, uint160[] memory secondsPerLiquidity) {
			if (
				_hasPreviousCheckpoint
					&& (ticks[0] != _previousTickCumulative
						|| secondsPerLiquidity[0] != _previousSecondsPerLiquidityCumulativeX128)
			) return false;
			return ticks[modelIndex] == _modelTickCumulative
				&& secondsPerLiquidity[modelIndex] == _modelSecondsPerLiquidityCumulativeX128
				&& ticks[modelIndex + 1] == currentTickCumulative
				&& secondsPerLiquidity[modelIndex + 1] == currentSecondsPerLiquidityCumulativeX128;
		} catch {
			return false;
		}
	}

	function _project(
		int56 tickCumulative,
		uint160 secondsPerLiquidityCumulativeX128,
		uint32 elapsed,
		int24 tick,
		uint128 liquidity
	) private pure returns (int56 projectedTickCumulative, uint160 projectedSecondsPerLiquidityCumulativeX128) {
		unchecked {
			projectedTickCumulative = tickCumulative + int56(tick) * int56(uint56(elapsed));
			projectedSecondsPerLiquidityCumulativeX128 = secondsPerLiquidityCumulativeX128
				+ uint160((uint256(elapsed) * _Q128) / (liquidity == 0 ? 1 : liquidity));
		}
	}

	function _elapsed(uint32 from, uint32 to) private pure returns (uint32 elapsed) {
		unchecked {
			elapsed = to - from;
		}
	}
}
