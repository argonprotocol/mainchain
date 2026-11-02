// SPDX-License-Identifier: MIT
pragma solidity ^0.8.24;

import {Bytes} from "@openzeppelin/contracts/utils/Bytes.sol";
import {SafeCast} from "@openzeppelin/contracts/utils/math/SafeCast.sol";
import {IPoolManager} from "@uniswap/v4-core/src/interfaces/IPoolManager.sol";
import {ParseBytes} from "@uniswap/v4-core/src/libraries/ParseBytes.sol";
import {StateLibrary} from "@uniswap/v4-core/src/libraries/StateLibrary.sol";
import {BalanceDelta} from "@uniswap/v4-core/src/types/BalanceDelta.sol";
import {BaseV4Quoter} from "@uniswap/v4-periphery/src/base/BaseV4Quoter.sol";
import {IMsgSender} from "@uniswap/v4-periphery/src/interfaces/IMsgSender.sol";
import {IV4Quoter} from "@uniswap/v4-periphery/src/interfaces/IV4Quoter.sol";
import {Locker} from "@uniswap/v4-periphery/src/libraries/Locker.sol";
import {QuoterRevert} from "@uniswap/v4-periphery/src/libraries/QuoterRevert.sol";

/// @notice Quotes single-pool v4 swaps, including hook execution and the resulting spot price.
/// @dev Use eth_call: the simulated swap and hook writes are rolled back before returning a quote.
contract UniswapV4PriceQuoter is BaseV4Quoter, IMsgSender {
    using ParseBytes for bytes;
    using SafeCast for int256;
    using StateLibrary for IPoolManager;

    /// @notice The configured PoolManager is zero or has no deployed code.
    error InvalidPoolManager();
    /// @notice Internal simulation result, caught only after reverting the simulated state changes.
    error QuoteResult(uint256 amount, uint160 sqrtPriceX96After);

    constructor(IPoolManager manager) BaseV4Quoter(manager) {
        if (address(manager) == address(0)) revert InvalidPoolManager();
        if (address(manager).code.length == 0) revert InvalidPoolManager();
    }

    /// @notice Returns output base units, simulated gas use and the post-swap Q64.96 square-root price.
    /// @dev The first two return values match the standard v4 quoter's order.
    function quoteExactInputSingle(IV4Quoter.QuoteExactSingleParams calldata params)
        external returns (uint256 amountOut, uint256 gasEstimate, uint160 sqrtPriceX96After)
    {
        return _quote(params, true);
    }

    /// @notice Returns input base units, simulated gas use and the post-swap Q64.96 square-root price.
    /// @dev Reverts if the requested output cannot be fully filled, as enforced by BaseV4Quoter.
    function quoteExactOutputSingle(IV4Quoter.QuoteExactSingleParams calldata params)
        external returns (uint256 amountIn, uint256 gasEstimate, uint160 sqrtPriceX96After)
    {
        return _quote(params, false);
    }

    /// @inheritdoc IMsgSender
    function msgSender() external view returns (address) {
        return Locker.get();
    }

    /// @notice Self-call dispatched by BaseV4Quoter during unlock; always reverts with the quote.
    function _quoteSingle(IV4Quoter.QuoteExactSingleParams calldata params, bool exactInput)
        external selfOnly returns (bytes memory)
    {
        int256 amountSpecified = int256(uint256(params.exactAmount));
        if (exactInput) amountSpecified = -amountSpecified;

        BalanceDelta swapDelta = _swap(params.poolKey, params.zeroForOne, amountSpecified, params.hookData);
        int256 quoteAmount;
        if (exactInput) {
            quoteAmount = params.zeroForOne ? swapDelta.amount1() : swapDelta.amount0();
        } else {
            quoteAmount = params.zeroForOne ? swapDelta.amount0() : swapDelta.amount1();
            quoteAmount = -quoteAmount;
        }

        // Capture the price while the simulated swap is still applied, before rolling it back.
        (uint160 sqrtPriceX96After,,,) = poolManager.getSlot0(params.poolKey.toId());
        revert QuoteResult(quoteAmount.toUint256(), sqrtPriceX96After);
    }

    function _quote(IV4Quoter.QuoteExactSingleParams calldata params, bool exactInput)
        private returns (uint256 amount, uint256 gasEstimate, uint160 sqrtPriceX96After)
    {
        Locker.set(msg.sender);
        uint256 gasBefore = gasleft();
        try poolManager.unlock(abi.encodeCall(this._quoteSingle, (params, exactInput))) {
            revert UnexpectedCallSuccess();
        } catch (bytes memory reason) {
            gasEstimate = gasBefore - gasleft();
            Locker.set(address(0));

            if (reason.length != 68) revert QuoterRevert.UnexpectedRevertBytes(reason);
            if (reason.parseSelector() != QuoteResult.selector) {
                revert QuoterRevert.UnexpectedRevertBytes(reason);
            }
            (amount, sqrtPriceX96After) = abi.decode(Bytes.slice(reason, 4), (uint256, uint160));
        }
    }
}
