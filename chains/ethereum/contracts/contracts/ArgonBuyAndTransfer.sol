// SPDX-License-Identifier: MIT
pragma solidity ^0.8.24;

import {IERC20} from "@openzeppelin/contracts/token/ERC20/IERC20.sol";
import {SafeERC20} from "@openzeppelin/contracts/token/ERC20/utils/SafeERC20.sol";
import {SafeCast} from "@openzeppelin/contracts/utils/math/SafeCast.sol";
import {ReentrancyGuard} from "@openzeppelin/contracts/utils/ReentrancyGuard.sol";
import {IUniversalRouter} from "@uniswap/universal-router/contracts/interfaces/IUniversalRouter.sol";
import {Commands} from "@uniswap/universal-router/contracts/libraries/Commands.sol";
import {IAllowanceTransfer} from "@uniswap/permit2/src/interfaces/IAllowanceTransfer.sol";
import {IPoolManager} from "@uniswap/v4-core/src/interfaces/IPoolManager.sol";
import {PoolKey} from "@uniswap/v4-core/src/types/PoolKey.sol";
import {PoolId, PoolIdLibrary} from "@uniswap/v4-core/src/types/PoolId.sol";
import {Currency} from "@uniswap/v4-core/src/types/Currency.sol";
import {StateLibrary} from "@uniswap/v4-core/src/libraries/StateLibrary.sol";
import {IImmutableState} from "@uniswap/v4-periphery/src/interfaces/IImmutableState.sol";
import {IV4Router} from "@uniswap/v4-periphery/src/interfaces/IV4Router.sol";
import {Actions} from "@uniswap/v4-periphery/src/libraries/Actions.sol";
import {MintingGateway} from "./MintingGateway.sol";

/// @notice Buys ARGN from one native Uniswap v4 pool and burns it for delivery to Argon.
/// @dev Reward eligibility is determined on Argon; this contract has no reward-pool allowlist.
contract ArgonBuyAndTransfer is ReentrancyGuard {
	using SafeERC20 for IERC20;
	using SafeCast for uint256;
	using PoolIdLibrary for PoolKey;

	uint256 private constant _SCALE = 1e12;
	IERC20 public immutable usdc;
	IERC20 public immutable argon;
	MintingGateway public immutable gateway;
	IUniversalRouter public immutable router;
	IAllowanceTransfer public immutable permit2;
	IPoolManager public immutable poolManager;

	struct Request {
		bytes32 argonAccountId;
		uint128 maxUsdcIn;
		uint128 minMicrogonsOut;
		uint256 deadline;
	}

	error InvalidRequest();
	error InsufficientOutput();
	error InvalidPool();
	error PurchaseTooSmall();

	constructor(
		address usdcAddress,
		address argonAddress,
		address gatewayAddress,
		address routerAddress,
		address permit2Address
	) {
		usdc = IERC20(usdcAddress);
		argon = IERC20(argonAddress);
		gateway = MintingGateway(gatewayAddress);
		router = IUniversalRouter(routerAddress);
		permit2 = IAllowanceTransfer(permit2Address);
		poolManager = IImmutableState(routerAddress).poolManager();
	}

	/// @notice Buys from one canonical USDC/ARGN v4 pool using its native PoolKey.
	function buyAndTransfer(Request calldata request, PoolKey calldata key, bytes calldata hookData)
		external nonReentrant returns (uint128 microgons, uint64 activityNonce)
	{
		if (request.argonAccountId == bytes32(0) || request.maxUsdcIn == 0 || request.minMicrogonsOut == 0) {
			revert InvalidRequest();
		}
		if (request.deadline < block.timestamp) revert InvalidRequest();
		if (request.minMicrogonsOut < gateway.minimumSwapMicrogons()) revert PurchaseTooSmall();

		{
			bool usdcIsToken0 = address(usdc) < address(argon);
			if (Currency.unwrap(key.currency0) != (usdcIsToken0 ? address(usdc) : address(argon))) {
				revert InvalidPool();
			}
			if (Currency.unwrap(key.currency1) != (usdcIsToken0 ? address(argon) : address(usdc))) {
				revert InvalidPool();
			}
		}

		MintingGateway.SwapPurchase memory purchase;
		purchase.buyer = msg.sender;
		purchase.argonAccountId = request.argonAccountId;
		purchase.usdcToken = address(usdc);
		purchase.poolManager = address(poolManager);
		purchase.poolId = PoolId.unwrap(key.toId());
		(purchase.sqrtPriceBeforeX96,,,) = StateLibrary.getSlot0(poolManager, PoolId.wrap(purchase.poolId));
		if (purchase.sqrtPriceBeforeX96 == 0) revert InvalidPool();

		uint256 usdcBefore = usdc.balanceOf(address(this));
		uint256 argonBefore = argon.balanceOf(address(this));
		usdc.safeTransferFrom(msg.sender, address(this), request.maxUsdcIn);
		usdc.forceApprove(address(permit2), request.maxUsdcIn);
		permit2.approve(address(usdc), address(router), request.maxUsdcIn, request.deadline.toUint48());

		_swap(request, key, hookData);
		(purchase.sqrtPriceAfterX96,,,) = StateLibrary.getSlot0(poolManager, PoolId.wrap(purchase.poolId));
		permit2.approve(address(usdc), address(router), 0, 0);
		usdc.forceApprove(address(permit2), 0);

		uint256 refund = usdc.balanceOf(address(this)) - usdcBefore;
		purchase.usdcSpent = uint256(request.maxUsdcIn) - refund;
		purchase.argonBought = argon.balanceOf(address(this)) - argonBefore;
		if (purchase.argonBought < uint256(request.minMicrogonsOut) * _SCALE) revert InsufficientOutput();

		uint256 burnAmount = purchase.argonBought / _SCALE * _SCALE;
		argon.forceApprove(address(gateway), burnAmount);
		(microgons, activityNonce) = gateway.burnFromSwap(purchase);
		if (refund != 0) usdc.safeTransfer(msg.sender, refund);
		uint256 dust = purchase.argonBought - burnAmount;
		if (dust != 0) argon.safeTransfer(msg.sender, dust);
	}

	function _swap(Request calldata request, PoolKey calldata key, bytes calldata hookData) private {
		bytes memory actions = abi.encodePacked(
			uint8(Actions.SWAP_EXACT_IN_SINGLE),
			uint8(Actions.SETTLE_ALL),
			uint8(Actions.TAKE_ALL)
		);
		bytes[] memory params = new bytes[](3);
		params[0] = abi.encode(IV4Router.ExactInputSingleParams({
			poolKey: key,
			zeroForOne: address(usdc) < address(argon),
			amountIn: request.maxUsdcIn,
			amountOutMinimum: (uint256(request.minMicrogonsOut) * _SCALE).toUint128(),
			minHopPriceX36: 0,
			hookData: hookData
		}));
		params[1] = abi.encode(address(usdc), request.maxUsdcIn);
		params[2] = abi.encode(address(argon), 0);

		bytes[] memory inputs = new bytes[](1);
		inputs[0] = abi.encode(actions, params);
		router.execute(abi.encodePacked(uint8(Commands.V4_SWAP)), inputs, request.deadline);
	}
}
