// SPDX-License-Identifier: MIT
pragma solidity ^0.8.24;

import { Initializable } from "@openzeppelin/contracts-upgradeable/proxy/utils/Initializable.sol";
import { OwnableUpgradeable } from "@openzeppelin/contracts-upgradeable/access/OwnableUpgradeable.sol";
import { PausableUpgradeable } from "@openzeppelin/contracts-upgradeable/utils/PausableUpgradeable.sol";
import { ICanonicalToken } from "./interfaces/ICanonicalToken.sol";

contract MintingGateway is Initializable, OwnableUpgradeable, PausableUpgradeable {
	uint8 public constant RUNTIME_DECIMALS = 6;
	uint8 public constant TOKEN_DECIMALS = 18;
	uint256 public constant RUNTIME_TO_ERC20_SCALE = 10 ** (TOKEN_DECIMALS - RUNTIME_DECIMALS);

	mapping(address account => uint64 nonce) public accountNonces;
	address public immutable argonToken;
	address public immutable argonotToken;
	address public guardian;

	error ArrayLengthMismatch();
	error NotGuardianOrOwner(address caller);
	error UnsupportedToken(address token);
	error ZeroAdminSafe();
	error ZeroGuardian();
	error ZeroAmount();
	error ZeroRecipient(uint256 index);

	event BurnForTransfer(
		address indexed from,
		address indexed token,
		uint256 amountBaseUnits,
		bytes32 argonDestination,
		uint64 accountNonce
	);

	event AdminMintBatch(
		address indexed token,
		uint256 recipientCount,
		uint256 totalAmountBaseUnits
	);
	event GuardianUpdated(address indexed previousGuardian, address indexed newGuardian);

	/// @custom:oz-upgrades-unsafe-allow constructor
	constructor(address argonTokenAddress, address argonotTokenAddress) {
		argonToken = argonTokenAddress;
		argonotToken = argonotTokenAddress;
		_disableInitializers();
	}

	function initialize(address adminSafe, address guardianAddress) external initializer {
		if (adminSafe == address(0)) revert ZeroAdminSafe();
		if (guardianAddress == address(0)) revert ZeroGuardian();

		__Ownable_init(adminSafe);
		__Pausable_init();
		guardian = guardianAddress;
	}

	function burnForTransfer(
		address token,
		uint256 amountBaseUnits,
		bytes32 argonDestination
	) external whenNotPaused {
		_requireCanonicalToken(token);
		if (amountBaseUnits == 0) revert ZeroAmount();

		ICanonicalToken(token).burnFrom(msg.sender, toTokenAmount(amountBaseUnits));

		uint64 accountNonce = accountNonces[msg.sender] + 1;
		accountNonces[msg.sender] = accountNonce;

		emit BurnForTransfer(msg.sender, token, amountBaseUnits, argonDestination, accountNonce);
	}

	function adminMintBatch(
		address token,
		address[] calldata recipients,
		uint256[] calldata amountsBaseUnits
	) external onlyOwner whenNotPaused {
		_requireCanonicalToken(token);
		if (recipients.length != amountsBaseUnits.length) revert ArrayLengthMismatch();

		uint256 totalAmountBaseUnits = 0;

		for (uint256 index = 0; index < recipients.length; index += 1) {
			address recipient = recipients[index];
			uint256 amountBaseUnits = amountsBaseUnits[index];

			if (recipient == address(0)) revert ZeroRecipient(index);
			if (amountBaseUnits == 0) revert ZeroAmount();

			ICanonicalToken(token).mint(recipient, toTokenAmount(amountBaseUnits));
			totalAmountBaseUnits += amountBaseUnits;
		}

		emit AdminMintBatch(token, recipients.length, totalAmountBaseUnits);
	}

	function pause() external {
		if (msg.sender != guardian && msg.sender != owner()) {
			revert NotGuardianOrOwner(msg.sender);
		}

		_pause();
	}

	function setGuardian(address guardianAddress) external onlyOwner {
		if (guardianAddress == address(0)) revert ZeroGuardian();

		address previousGuardian = guardian;
		guardian = guardianAddress;

		emit GuardianUpdated(previousGuardian, guardianAddress);
	}

	function toTokenAmount(uint256 amountBaseUnits) public pure returns (uint256) {
		return amountBaseUnits * RUNTIME_TO_ERC20_SCALE;
	}

	function unpause() external onlyOwner {
		_unpause();
	}

	function _requireCanonicalToken(address token) private view {
		if (token != argonToken && token != argonotToken) revert UnsupportedToken(token);
	}
}
