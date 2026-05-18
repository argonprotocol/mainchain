//////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////
// ArgonToken
//////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////

export const argonTokenAbi = [
  {
    type: 'constructor',
    inputs: [{ name: 'gateway', internalType: 'address', type: 'address' }],
    stateMutability: 'nonpayable',
  },
  { type: 'error', inputs: [], name: 'ECDSAInvalidSignature' },
  {
    type: 'error',
    inputs: [{ name: 'length', internalType: 'uint256', type: 'uint256' }],
    name: 'ECDSAInvalidSignatureLength',
  },
  {
    type: 'error',
    inputs: [{ name: 's', internalType: 'bytes32', type: 'bytes32' }],
    name: 'ECDSAInvalidSignatureS',
  },
  {
    type: 'error',
    inputs: [
      { name: 'spender', internalType: 'address', type: 'address' },
      { name: 'allowance', internalType: 'uint256', type: 'uint256' },
      { name: 'needed', internalType: 'uint256', type: 'uint256' },
    ],
    name: 'ERC20InsufficientAllowance',
  },
  {
    type: 'error',
    inputs: [
      { name: 'sender', internalType: 'address', type: 'address' },
      { name: 'balance', internalType: 'uint256', type: 'uint256' },
      { name: 'needed', internalType: 'uint256', type: 'uint256' },
    ],
    name: 'ERC20InsufficientBalance',
  },
  {
    type: 'error',
    inputs: [{ name: 'approver', internalType: 'address', type: 'address' }],
    name: 'ERC20InvalidApprover',
  },
  {
    type: 'error',
    inputs: [{ name: 'receiver', internalType: 'address', type: 'address' }],
    name: 'ERC20InvalidReceiver',
  },
  {
    type: 'error',
    inputs: [{ name: 'sender', internalType: 'address', type: 'address' }],
    name: 'ERC20InvalidSender',
  },
  {
    type: 'error',
    inputs: [{ name: 'spender', internalType: 'address', type: 'address' }],
    name: 'ERC20InvalidSpender',
  },
  {
    type: 'error',
    inputs: [{ name: 'deadline', internalType: 'uint256', type: 'uint256' }],
    name: 'ERC2612ExpiredSignature',
  },
  {
    type: 'error',
    inputs: [
      { name: 'signer', internalType: 'address', type: 'address' },
      { name: 'owner', internalType: 'address', type: 'address' },
    ],
    name: 'ERC2612InvalidSigner',
  },
  {
    type: 'error',
    inputs: [
      { name: 'account', internalType: 'address', type: 'address' },
      { name: 'currentNonce', internalType: 'uint256', type: 'uint256' },
    ],
    name: 'InvalidAccountNonce',
  },
  {
    type: 'error',
    inputs: [{ name: 'gateway', internalType: 'address', type: 'address' }],
    name: 'InvalidGateway',
  },
  { type: 'error', inputs: [], name: 'InvalidShortString' },
  {
    type: 'error',
    inputs: [{ name: 'caller', internalType: 'address', type: 'address' }],
    name: 'OnlyGateway',
  },
  {
    type: 'error',
    inputs: [{ name: 'str', internalType: 'string', type: 'string' }],
    name: 'StringTooLong',
  },
  {
    type: 'event',
    anonymous: false,
    inputs: [
      { name: 'owner', internalType: 'address', type: 'address', indexed: true },
      { name: 'spender', internalType: 'address', type: 'address', indexed: true },
      { name: 'value', internalType: 'uint256', type: 'uint256', indexed: false },
    ],
    name: 'Approval',
  },
  { type: 'event', anonymous: false, inputs: [], name: 'EIP712DomainChanged' },
  {
    type: 'event',
    anonymous: false,
    inputs: [
      { name: 'from', internalType: 'address', type: 'address', indexed: true },
      { name: 'to', internalType: 'address', type: 'address', indexed: true },
      { name: 'value', internalType: 'uint256', type: 'uint256', indexed: false },
    ],
    name: 'Transfer',
  },
  {
    type: 'function',
    inputs: [],
    name: 'DOMAIN_SEPARATOR',
    outputs: [{ name: '', internalType: 'bytes32', type: 'bytes32' }],
    stateMutability: 'view',
  },
  {
    type: 'function',
    inputs: [
      { name: 'owner', internalType: 'address', type: 'address' },
      { name: 'spender', internalType: 'address', type: 'address' },
    ],
    name: 'allowance',
    outputs: [{ name: '', internalType: 'uint256', type: 'uint256' }],
    stateMutability: 'view',
  },
  {
    type: 'function',
    inputs: [
      { name: 'spender', internalType: 'address', type: 'address' },
      { name: 'value', internalType: 'uint256', type: 'uint256' },
    ],
    name: 'approve',
    outputs: [{ name: '', internalType: 'bool', type: 'bool' }],
    stateMutability: 'nonpayable',
  },
  {
    type: 'function',
    inputs: [{ name: 'account', internalType: 'address', type: 'address' }],
    name: 'balanceOf',
    outputs: [{ name: '', internalType: 'uint256', type: 'uint256' }],
    stateMutability: 'view',
  },
  {
    type: 'function',
    inputs: [
      { name: 'account', internalType: 'address', type: 'address' },
      { name: 'amount', internalType: 'uint256', type: 'uint256' },
    ],
    name: 'burnFrom',
    outputs: [],
    stateMutability: 'nonpayable',
  },
  {
    type: 'function',
    inputs: [],
    name: 'decimals',
    outputs: [{ name: '', internalType: 'uint8', type: 'uint8' }],
    stateMutability: 'view',
  },
  {
    type: 'function',
    inputs: [],
    name: 'eip712Domain',
    outputs: [
      { name: 'fields', internalType: 'bytes1', type: 'bytes1' },
      { name: 'name', internalType: 'string', type: 'string' },
      { name: 'version', internalType: 'string', type: 'string' },
      { name: 'chainId', internalType: 'uint256', type: 'uint256' },
      { name: 'verifyingContract', internalType: 'address', type: 'address' },
      { name: 'salt', internalType: 'bytes32', type: 'bytes32' },
      { name: 'extensions', internalType: 'uint256[]', type: 'uint256[]' },
    ],
    stateMutability: 'view',
  },
  {
    type: 'function',
    inputs: [],
    name: 'gateway',
    outputs: [{ name: '', internalType: 'address', type: 'address' }],
    stateMutability: 'view',
  },
  {
    type: 'function',
    inputs: [
      { name: 'to', internalType: 'address', type: 'address' },
      { name: 'amount', internalType: 'uint256', type: 'uint256' },
    ],
    name: 'mint',
    outputs: [],
    stateMutability: 'nonpayable',
  },
  {
    type: 'function',
    inputs: [],
    name: 'name',
    outputs: [{ name: '', internalType: 'string', type: 'string' }],
    stateMutability: 'view',
  },
  {
    type: 'function',
    inputs: [{ name: 'owner', internalType: 'address', type: 'address' }],
    name: 'nonces',
    outputs: [{ name: '', internalType: 'uint256', type: 'uint256' }],
    stateMutability: 'view',
  },
  {
    type: 'function',
    inputs: [
      { name: 'owner', internalType: 'address', type: 'address' },
      { name: 'spender', internalType: 'address', type: 'address' },
      { name: 'value', internalType: 'uint256', type: 'uint256' },
      { name: 'deadline', internalType: 'uint256', type: 'uint256' },
      { name: 'v', internalType: 'uint8', type: 'uint8' },
      { name: 'r', internalType: 'bytes32', type: 'bytes32' },
      { name: 's', internalType: 'bytes32', type: 'bytes32' },
    ],
    name: 'permit',
    outputs: [],
    stateMutability: 'nonpayable',
  },
  {
    type: 'function',
    inputs: [],
    name: 'symbol',
    outputs: [{ name: '', internalType: 'string', type: 'string' }],
    stateMutability: 'view',
  },
  {
    type: 'function',
    inputs: [],
    name: 'totalSupply',
    outputs: [{ name: '', internalType: 'uint256', type: 'uint256' }],
    stateMutability: 'view',
  },
  {
    type: 'function',
    inputs: [
      { name: 'to', internalType: 'address', type: 'address' },
      { name: 'value', internalType: 'uint256', type: 'uint256' },
    ],
    name: 'transfer',
    outputs: [{ name: '', internalType: 'bool', type: 'bool' }],
    stateMutability: 'nonpayable',
  },
  {
    type: 'function',
    inputs: [
      { name: 'from', internalType: 'address', type: 'address' },
      { name: 'to', internalType: 'address', type: 'address' },
      { name: 'value', internalType: 'uint256', type: 'uint256' },
    ],
    name: 'transferFrom',
    outputs: [{ name: '', internalType: 'bool', type: 'bool' }],
    stateMutability: 'nonpayable',
  },
] as const;

//////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////
// ArgonotToken
//////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////

export const argonotTokenAbi = [
  {
    type: 'constructor',
    inputs: [{ name: 'gateway', internalType: 'address', type: 'address' }],
    stateMutability: 'nonpayable',
  },
  { type: 'error', inputs: [], name: 'ECDSAInvalidSignature' },
  {
    type: 'error',
    inputs: [{ name: 'length', internalType: 'uint256', type: 'uint256' }],
    name: 'ECDSAInvalidSignatureLength',
  },
  {
    type: 'error',
    inputs: [{ name: 's', internalType: 'bytes32', type: 'bytes32' }],
    name: 'ECDSAInvalidSignatureS',
  },
  {
    type: 'error',
    inputs: [
      { name: 'spender', internalType: 'address', type: 'address' },
      { name: 'allowance', internalType: 'uint256', type: 'uint256' },
      { name: 'needed', internalType: 'uint256', type: 'uint256' },
    ],
    name: 'ERC20InsufficientAllowance',
  },
  {
    type: 'error',
    inputs: [
      { name: 'sender', internalType: 'address', type: 'address' },
      { name: 'balance', internalType: 'uint256', type: 'uint256' },
      { name: 'needed', internalType: 'uint256', type: 'uint256' },
    ],
    name: 'ERC20InsufficientBalance',
  },
  {
    type: 'error',
    inputs: [{ name: 'approver', internalType: 'address', type: 'address' }],
    name: 'ERC20InvalidApprover',
  },
  {
    type: 'error',
    inputs: [{ name: 'receiver', internalType: 'address', type: 'address' }],
    name: 'ERC20InvalidReceiver',
  },
  {
    type: 'error',
    inputs: [{ name: 'sender', internalType: 'address', type: 'address' }],
    name: 'ERC20InvalidSender',
  },
  {
    type: 'error',
    inputs: [{ name: 'spender', internalType: 'address', type: 'address' }],
    name: 'ERC20InvalidSpender',
  },
  {
    type: 'error',
    inputs: [{ name: 'deadline', internalType: 'uint256', type: 'uint256' }],
    name: 'ERC2612ExpiredSignature',
  },
  {
    type: 'error',
    inputs: [
      { name: 'signer', internalType: 'address', type: 'address' },
      { name: 'owner', internalType: 'address', type: 'address' },
    ],
    name: 'ERC2612InvalidSigner',
  },
  {
    type: 'error',
    inputs: [
      { name: 'account', internalType: 'address', type: 'address' },
      { name: 'currentNonce', internalType: 'uint256', type: 'uint256' },
    ],
    name: 'InvalidAccountNonce',
  },
  {
    type: 'error',
    inputs: [{ name: 'gateway', internalType: 'address', type: 'address' }],
    name: 'InvalidGateway',
  },
  { type: 'error', inputs: [], name: 'InvalidShortString' },
  {
    type: 'error',
    inputs: [{ name: 'caller', internalType: 'address', type: 'address' }],
    name: 'OnlyGateway',
  },
  {
    type: 'error',
    inputs: [{ name: 'str', internalType: 'string', type: 'string' }],
    name: 'StringTooLong',
  },
  {
    type: 'event',
    anonymous: false,
    inputs: [
      { name: 'owner', internalType: 'address', type: 'address', indexed: true },
      { name: 'spender', internalType: 'address', type: 'address', indexed: true },
      { name: 'value', internalType: 'uint256', type: 'uint256', indexed: false },
    ],
    name: 'Approval',
  },
  { type: 'event', anonymous: false, inputs: [], name: 'EIP712DomainChanged' },
  {
    type: 'event',
    anonymous: false,
    inputs: [
      { name: 'from', internalType: 'address', type: 'address', indexed: true },
      { name: 'to', internalType: 'address', type: 'address', indexed: true },
      { name: 'value', internalType: 'uint256', type: 'uint256', indexed: false },
    ],
    name: 'Transfer',
  },
  {
    type: 'function',
    inputs: [],
    name: 'DOMAIN_SEPARATOR',
    outputs: [{ name: '', internalType: 'bytes32', type: 'bytes32' }],
    stateMutability: 'view',
  },
  {
    type: 'function',
    inputs: [
      { name: 'owner', internalType: 'address', type: 'address' },
      { name: 'spender', internalType: 'address', type: 'address' },
    ],
    name: 'allowance',
    outputs: [{ name: '', internalType: 'uint256', type: 'uint256' }],
    stateMutability: 'view',
  },
  {
    type: 'function',
    inputs: [
      { name: 'spender', internalType: 'address', type: 'address' },
      { name: 'value', internalType: 'uint256', type: 'uint256' },
    ],
    name: 'approve',
    outputs: [{ name: '', internalType: 'bool', type: 'bool' }],
    stateMutability: 'nonpayable',
  },
  {
    type: 'function',
    inputs: [{ name: 'account', internalType: 'address', type: 'address' }],
    name: 'balanceOf',
    outputs: [{ name: '', internalType: 'uint256', type: 'uint256' }],
    stateMutability: 'view',
  },
  {
    type: 'function',
    inputs: [
      { name: 'account', internalType: 'address', type: 'address' },
      { name: 'amount', internalType: 'uint256', type: 'uint256' },
    ],
    name: 'burnFrom',
    outputs: [],
    stateMutability: 'nonpayable',
  },
  {
    type: 'function',
    inputs: [],
    name: 'decimals',
    outputs: [{ name: '', internalType: 'uint8', type: 'uint8' }],
    stateMutability: 'view',
  },
  {
    type: 'function',
    inputs: [],
    name: 'eip712Domain',
    outputs: [
      { name: 'fields', internalType: 'bytes1', type: 'bytes1' },
      { name: 'name', internalType: 'string', type: 'string' },
      { name: 'version', internalType: 'string', type: 'string' },
      { name: 'chainId', internalType: 'uint256', type: 'uint256' },
      { name: 'verifyingContract', internalType: 'address', type: 'address' },
      { name: 'salt', internalType: 'bytes32', type: 'bytes32' },
      { name: 'extensions', internalType: 'uint256[]', type: 'uint256[]' },
    ],
    stateMutability: 'view',
  },
  {
    type: 'function',
    inputs: [],
    name: 'gateway',
    outputs: [{ name: '', internalType: 'address', type: 'address' }],
    stateMutability: 'view',
  },
  {
    type: 'function',
    inputs: [
      { name: 'to', internalType: 'address', type: 'address' },
      { name: 'amount', internalType: 'uint256', type: 'uint256' },
    ],
    name: 'mint',
    outputs: [],
    stateMutability: 'nonpayable',
  },
  {
    type: 'function',
    inputs: [],
    name: 'name',
    outputs: [{ name: '', internalType: 'string', type: 'string' }],
    stateMutability: 'view',
  },
  {
    type: 'function',
    inputs: [{ name: 'owner', internalType: 'address', type: 'address' }],
    name: 'nonces',
    outputs: [{ name: '', internalType: 'uint256', type: 'uint256' }],
    stateMutability: 'view',
  },
  {
    type: 'function',
    inputs: [
      { name: 'owner', internalType: 'address', type: 'address' },
      { name: 'spender', internalType: 'address', type: 'address' },
      { name: 'value', internalType: 'uint256', type: 'uint256' },
      { name: 'deadline', internalType: 'uint256', type: 'uint256' },
      { name: 'v', internalType: 'uint8', type: 'uint8' },
      { name: 'r', internalType: 'bytes32', type: 'bytes32' },
      { name: 's', internalType: 'bytes32', type: 'bytes32' },
    ],
    name: 'permit',
    outputs: [],
    stateMutability: 'nonpayable',
  },
  {
    type: 'function',
    inputs: [],
    name: 'symbol',
    outputs: [{ name: '', internalType: 'string', type: 'string' }],
    stateMutability: 'view',
  },
  {
    type: 'function',
    inputs: [],
    name: 'totalSupply',
    outputs: [{ name: '', internalType: 'uint256', type: 'uint256' }],
    stateMutability: 'view',
  },
  {
    type: 'function',
    inputs: [
      { name: 'to', internalType: 'address', type: 'address' },
      { name: 'value', internalType: 'uint256', type: 'uint256' },
    ],
    name: 'transfer',
    outputs: [{ name: '', internalType: 'bool', type: 'bool' }],
    stateMutability: 'nonpayable',
  },
  {
    type: 'function',
    inputs: [
      { name: 'from', internalType: 'address', type: 'address' },
      { name: 'to', internalType: 'address', type: 'address' },
      { name: 'value', internalType: 'uint256', type: 'uint256' },
    ],
    name: 'transferFrom',
    outputs: [{ name: '', internalType: 'bool', type: 'bool' }],
    stateMutability: 'nonpayable',
  },
] as const;

//////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////
// CanonicalMintableBurnableERC20
//////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////

export const canonicalMintableBurnableErc20Abi = [
  { type: 'error', inputs: [], name: 'ECDSAInvalidSignature' },
  {
    type: 'error',
    inputs: [{ name: 'length', internalType: 'uint256', type: 'uint256' }],
    name: 'ECDSAInvalidSignatureLength',
  },
  {
    type: 'error',
    inputs: [{ name: 's', internalType: 'bytes32', type: 'bytes32' }],
    name: 'ECDSAInvalidSignatureS',
  },
  {
    type: 'error',
    inputs: [
      { name: 'spender', internalType: 'address', type: 'address' },
      { name: 'allowance', internalType: 'uint256', type: 'uint256' },
      { name: 'needed', internalType: 'uint256', type: 'uint256' },
    ],
    name: 'ERC20InsufficientAllowance',
  },
  {
    type: 'error',
    inputs: [
      { name: 'sender', internalType: 'address', type: 'address' },
      { name: 'balance', internalType: 'uint256', type: 'uint256' },
      { name: 'needed', internalType: 'uint256', type: 'uint256' },
    ],
    name: 'ERC20InsufficientBalance',
  },
  {
    type: 'error',
    inputs: [{ name: 'approver', internalType: 'address', type: 'address' }],
    name: 'ERC20InvalidApprover',
  },
  {
    type: 'error',
    inputs: [{ name: 'receiver', internalType: 'address', type: 'address' }],
    name: 'ERC20InvalidReceiver',
  },
  {
    type: 'error',
    inputs: [{ name: 'sender', internalType: 'address', type: 'address' }],
    name: 'ERC20InvalidSender',
  },
  {
    type: 'error',
    inputs: [{ name: 'spender', internalType: 'address', type: 'address' }],
    name: 'ERC20InvalidSpender',
  },
  {
    type: 'error',
    inputs: [{ name: 'deadline', internalType: 'uint256', type: 'uint256' }],
    name: 'ERC2612ExpiredSignature',
  },
  {
    type: 'error',
    inputs: [
      { name: 'signer', internalType: 'address', type: 'address' },
      { name: 'owner', internalType: 'address', type: 'address' },
    ],
    name: 'ERC2612InvalidSigner',
  },
  {
    type: 'error',
    inputs: [
      { name: 'account', internalType: 'address', type: 'address' },
      { name: 'currentNonce', internalType: 'uint256', type: 'uint256' },
    ],
    name: 'InvalidAccountNonce',
  },
  {
    type: 'error',
    inputs: [{ name: 'gateway', internalType: 'address', type: 'address' }],
    name: 'InvalidGateway',
  },
  { type: 'error', inputs: [], name: 'InvalidShortString' },
  {
    type: 'error',
    inputs: [{ name: 'caller', internalType: 'address', type: 'address' }],
    name: 'OnlyGateway',
  },
  {
    type: 'error',
    inputs: [{ name: 'str', internalType: 'string', type: 'string' }],
    name: 'StringTooLong',
  },
  {
    type: 'event',
    anonymous: false,
    inputs: [
      { name: 'owner', internalType: 'address', type: 'address', indexed: true },
      { name: 'spender', internalType: 'address', type: 'address', indexed: true },
      { name: 'value', internalType: 'uint256', type: 'uint256', indexed: false },
    ],
    name: 'Approval',
  },
  { type: 'event', anonymous: false, inputs: [], name: 'EIP712DomainChanged' },
  {
    type: 'event',
    anonymous: false,
    inputs: [
      { name: 'from', internalType: 'address', type: 'address', indexed: true },
      { name: 'to', internalType: 'address', type: 'address', indexed: true },
      { name: 'value', internalType: 'uint256', type: 'uint256', indexed: false },
    ],
    name: 'Transfer',
  },
  {
    type: 'function',
    inputs: [],
    name: 'DOMAIN_SEPARATOR',
    outputs: [{ name: '', internalType: 'bytes32', type: 'bytes32' }],
    stateMutability: 'view',
  },
  {
    type: 'function',
    inputs: [
      { name: 'owner', internalType: 'address', type: 'address' },
      { name: 'spender', internalType: 'address', type: 'address' },
    ],
    name: 'allowance',
    outputs: [{ name: '', internalType: 'uint256', type: 'uint256' }],
    stateMutability: 'view',
  },
  {
    type: 'function',
    inputs: [
      { name: 'spender', internalType: 'address', type: 'address' },
      { name: 'value', internalType: 'uint256', type: 'uint256' },
    ],
    name: 'approve',
    outputs: [{ name: '', internalType: 'bool', type: 'bool' }],
    stateMutability: 'nonpayable',
  },
  {
    type: 'function',
    inputs: [{ name: 'account', internalType: 'address', type: 'address' }],
    name: 'balanceOf',
    outputs: [{ name: '', internalType: 'uint256', type: 'uint256' }],
    stateMutability: 'view',
  },
  {
    type: 'function',
    inputs: [
      { name: 'account', internalType: 'address', type: 'address' },
      { name: 'amount', internalType: 'uint256', type: 'uint256' },
    ],
    name: 'burnFrom',
    outputs: [],
    stateMutability: 'nonpayable',
  },
  {
    type: 'function',
    inputs: [],
    name: 'decimals',
    outputs: [{ name: '', internalType: 'uint8', type: 'uint8' }],
    stateMutability: 'view',
  },
  {
    type: 'function',
    inputs: [],
    name: 'eip712Domain',
    outputs: [
      { name: 'fields', internalType: 'bytes1', type: 'bytes1' },
      { name: 'name', internalType: 'string', type: 'string' },
      { name: 'version', internalType: 'string', type: 'string' },
      { name: 'chainId', internalType: 'uint256', type: 'uint256' },
      { name: 'verifyingContract', internalType: 'address', type: 'address' },
      { name: 'salt', internalType: 'bytes32', type: 'bytes32' },
      { name: 'extensions', internalType: 'uint256[]', type: 'uint256[]' },
    ],
    stateMutability: 'view',
  },
  {
    type: 'function',
    inputs: [],
    name: 'gateway',
    outputs: [{ name: '', internalType: 'address', type: 'address' }],
    stateMutability: 'view',
  },
  {
    type: 'function',
    inputs: [
      { name: 'to', internalType: 'address', type: 'address' },
      { name: 'amount', internalType: 'uint256', type: 'uint256' },
    ],
    name: 'mint',
    outputs: [],
    stateMutability: 'nonpayable',
  },
  {
    type: 'function',
    inputs: [],
    name: 'name',
    outputs: [{ name: '', internalType: 'string', type: 'string' }],
    stateMutability: 'view',
  },
  {
    type: 'function',
    inputs: [{ name: 'owner', internalType: 'address', type: 'address' }],
    name: 'nonces',
    outputs: [{ name: '', internalType: 'uint256', type: 'uint256' }],
    stateMutability: 'view',
  },
  {
    type: 'function',
    inputs: [
      { name: 'owner', internalType: 'address', type: 'address' },
      { name: 'spender', internalType: 'address', type: 'address' },
      { name: 'value', internalType: 'uint256', type: 'uint256' },
      { name: 'deadline', internalType: 'uint256', type: 'uint256' },
      { name: 'v', internalType: 'uint8', type: 'uint8' },
      { name: 'r', internalType: 'bytes32', type: 'bytes32' },
      { name: 's', internalType: 'bytes32', type: 'bytes32' },
    ],
    name: 'permit',
    outputs: [],
    stateMutability: 'nonpayable',
  },
  {
    type: 'function',
    inputs: [],
    name: 'symbol',
    outputs: [{ name: '', internalType: 'string', type: 'string' }],
    stateMutability: 'view',
  },
  {
    type: 'function',
    inputs: [],
    name: 'totalSupply',
    outputs: [{ name: '', internalType: 'uint256', type: 'uint256' }],
    stateMutability: 'view',
  },
  {
    type: 'function',
    inputs: [
      { name: 'to', internalType: 'address', type: 'address' },
      { name: 'value', internalType: 'uint256', type: 'uint256' },
    ],
    name: 'transfer',
    outputs: [{ name: '', internalType: 'bool', type: 'bool' }],
    stateMutability: 'nonpayable',
  },
  {
    type: 'function',
    inputs: [
      { name: 'from', internalType: 'address', type: 'address' },
      { name: 'to', internalType: 'address', type: 'address' },
      { name: 'value', internalType: 'uint256', type: 'uint256' },
    ],
    name: 'transferFrom',
    outputs: [{ name: '', internalType: 'bool', type: 'bool' }],
    stateMutability: 'nonpayable',
  },
] as const;

//////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////
// ICanonicalToken
//////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////

export const iCanonicalTokenAbi = [
  {
    type: 'event',
    anonymous: false,
    inputs: [
      { name: 'owner', internalType: 'address', type: 'address', indexed: true },
      { name: 'spender', internalType: 'address', type: 'address', indexed: true },
      { name: 'value', internalType: 'uint256', type: 'uint256', indexed: false },
    ],
    name: 'Approval',
  },
  {
    type: 'event',
    anonymous: false,
    inputs: [
      { name: 'from', internalType: 'address', type: 'address', indexed: true },
      { name: 'to', internalType: 'address', type: 'address', indexed: true },
      { name: 'value', internalType: 'uint256', type: 'uint256', indexed: false },
    ],
    name: 'Transfer',
  },
  {
    type: 'function',
    inputs: [],
    name: 'DOMAIN_SEPARATOR',
    outputs: [{ name: '', internalType: 'bytes32', type: 'bytes32' }],
    stateMutability: 'view',
  },
  {
    type: 'function',
    inputs: [
      { name: 'owner', internalType: 'address', type: 'address' },
      { name: 'spender', internalType: 'address', type: 'address' },
    ],
    name: 'allowance',
    outputs: [{ name: '', internalType: 'uint256', type: 'uint256' }],
    stateMutability: 'view',
  },
  {
    type: 'function',
    inputs: [
      { name: 'spender', internalType: 'address', type: 'address' },
      { name: 'value', internalType: 'uint256', type: 'uint256' },
    ],
    name: 'approve',
    outputs: [{ name: '', internalType: 'bool', type: 'bool' }],
    stateMutability: 'nonpayable',
  },
  {
    type: 'function',
    inputs: [{ name: 'account', internalType: 'address', type: 'address' }],
    name: 'balanceOf',
    outputs: [{ name: '', internalType: 'uint256', type: 'uint256' }],
    stateMutability: 'view',
  },
  {
    type: 'function',
    inputs: [
      { name: 'account', internalType: 'address', type: 'address' },
      { name: 'amount', internalType: 'uint256', type: 'uint256' },
    ],
    name: 'burnFrom',
    outputs: [],
    stateMutability: 'nonpayable',
  },
  {
    type: 'function',
    inputs: [],
    name: 'decimals',
    outputs: [{ name: '', internalType: 'uint8', type: 'uint8' }],
    stateMutability: 'view',
  },
  {
    type: 'function',
    inputs: [
      { name: 'to', internalType: 'address', type: 'address' },
      { name: 'amount', internalType: 'uint256', type: 'uint256' },
    ],
    name: 'mint',
    outputs: [],
    stateMutability: 'nonpayable',
  },
  {
    type: 'function',
    inputs: [{ name: 'owner', internalType: 'address', type: 'address' }],
    name: 'nonces',
    outputs: [{ name: '', internalType: 'uint256', type: 'uint256' }],
    stateMutability: 'view',
  },
  {
    type: 'function',
    inputs: [
      { name: 'owner', internalType: 'address', type: 'address' },
      { name: 'spender', internalType: 'address', type: 'address' },
      { name: 'value', internalType: 'uint256', type: 'uint256' },
      { name: 'deadline', internalType: 'uint256', type: 'uint256' },
      { name: 'v', internalType: 'uint8', type: 'uint8' },
      { name: 'r', internalType: 'bytes32', type: 'bytes32' },
      { name: 's', internalType: 'bytes32', type: 'bytes32' },
    ],
    name: 'permit',
    outputs: [],
    stateMutability: 'nonpayable',
  },
  {
    type: 'function',
    inputs: [],
    name: 'totalSupply',
    outputs: [{ name: '', internalType: 'uint256', type: 'uint256' }],
    stateMutability: 'view',
  },
  {
    type: 'function',
    inputs: [
      { name: 'to', internalType: 'address', type: 'address' },
      { name: 'value', internalType: 'uint256', type: 'uint256' },
    ],
    name: 'transfer',
    outputs: [{ name: '', internalType: 'bool', type: 'bool' }],
    stateMutability: 'nonpayable',
  },
  {
    type: 'function',
    inputs: [
      { name: 'from', internalType: 'address', type: 'address' },
      { name: 'to', internalType: 'address', type: 'address' },
      { name: 'value', internalType: 'uint256', type: 'uint256' },
    ],
    name: 'transferFrom',
    outputs: [{ name: '', internalType: 'bool', type: 'bool' }],
    stateMutability: 'nonpayable',
  },
] as const;

//////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////
// MintingGateway
//////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////

export const mintingGatewayAbi = [
  {
    type: 'constructor',
    inputs: [
      { name: 'argonTokenAddress', internalType: 'address', type: 'address' },
      { name: 'argonotTokenAddress', internalType: 'address', type: 'address' },
    ],
    stateMutability: 'nonpayable',
  },
  { type: 'error', inputs: [], name: 'ArrayLengthMismatch' },
  { type: 'error', inputs: [], name: 'EnforcedPause' },
  { type: 'error', inputs: [], name: 'ExpectedPause' },
  { type: 'error', inputs: [], name: 'InvalidInitialization' },
  {
    type: 'error',
    inputs: [{ name: 'caller', internalType: 'address', type: 'address' }],
    name: 'NotGuardianOrOwner',
  },
  { type: 'error', inputs: [], name: 'NotInitializing' },
  {
    type: 'error',
    inputs: [{ name: 'owner', internalType: 'address', type: 'address' }],
    name: 'OwnableInvalidOwner',
  },
  {
    type: 'error',
    inputs: [{ name: 'account', internalType: 'address', type: 'address' }],
    name: 'OwnableUnauthorizedAccount',
  },
  {
    type: 'error',
    inputs: [{ name: 'token', internalType: 'address', type: 'address' }],
    name: 'UnsupportedToken',
  },
  { type: 'error', inputs: [], name: 'ZeroAdminSafe' },
  { type: 'error', inputs: [], name: 'ZeroAmount' },
  { type: 'error', inputs: [], name: 'ZeroGuardian' },
  {
    type: 'error',
    inputs: [{ name: 'index', internalType: 'uint256', type: 'uint256' }],
    name: 'ZeroRecipient',
  },
  {
    type: 'event',
    anonymous: false,
    inputs: [
      { name: 'token', internalType: 'address', type: 'address', indexed: true },
      { name: 'recipientCount', internalType: 'uint256', type: 'uint256', indexed: false },
      { name: 'totalAmount', internalType: 'uint64', type: 'uint64', indexed: false },
    ],
    name: 'AdminMintBatch',
  },
  {
    type: 'event',
    anonymous: false,
    inputs: [
      { name: 'from', internalType: 'address', type: 'address', indexed: true },
      { name: 'token', internalType: 'address', type: 'address', indexed: true },
      { name: 'amountBaseUnits', internalType: 'uint256', type: 'uint256', indexed: false },
      { name: 'argonDestination', internalType: 'bytes32', type: 'bytes32', indexed: false },
      { name: 'accountNonce', internalType: 'uint64', type: 'uint64', indexed: false },
    ],
    name: 'BurnForTransfer',
  },
  {
    type: 'event',
    anonymous: false,
    inputs: [
      { name: 'previousGuardian', internalType: 'address', type: 'address', indexed: true },
      { name: 'newGuardian', internalType: 'address', type: 'address', indexed: true },
    ],
    name: 'GuardianUpdated',
  },
  {
    type: 'event',
    anonymous: false,
    inputs: [{ name: 'version', internalType: 'uint64', type: 'uint64', indexed: false }],
    name: 'Initialized',
  },
  {
    type: 'event',
    anonymous: false,
    inputs: [
      { name: 'previousOwner', internalType: 'address', type: 'address', indexed: true },
      { name: 'newOwner', internalType: 'address', type: 'address', indexed: true },
    ],
    name: 'OwnershipTransferred',
  },
  {
    type: 'event',
    anonymous: false,
    inputs: [{ name: 'account', internalType: 'address', type: 'address', indexed: false }],
    name: 'Paused',
  },
  {
    type: 'event',
    anonymous: false,
    inputs: [{ name: 'account', internalType: 'address', type: 'address', indexed: false }],
    name: 'Unpaused',
  },
  {
    type: 'function',
    inputs: [],
    name: 'RUNTIME_DECIMALS',
    outputs: [{ name: '', internalType: 'uint8', type: 'uint8' }],
    stateMutability: 'view',
  },
  {
    type: 'function',
    inputs: [],
    name: 'RUNTIME_TO_ERC20_SCALE',
    outputs: [{ name: '', internalType: 'uint256', type: 'uint256' }],
    stateMutability: 'view',
  },
  {
    type: 'function',
    inputs: [],
    name: 'TOKEN_DECIMALS',
    outputs: [{ name: '', internalType: 'uint8', type: 'uint8' }],
    stateMutability: 'view',
  },
  {
    type: 'function',
    inputs: [{ name: 'account', internalType: 'address', type: 'address' }],
    name: 'accountNonces',
    outputs: [{ name: 'nonce', internalType: 'uint64', type: 'uint64' }],
    stateMutability: 'view',
  },
  {
    type: 'function',
    inputs: [
      { name: 'token', internalType: 'address', type: 'address' },
      { name: 'recipients', internalType: 'address[]', type: 'address[]' },
      { name: 'amounts', internalType: 'uint64[]', type: 'uint64[]' },
    ],
    name: 'adminMintBatch',
    outputs: [],
    stateMutability: 'nonpayable',
  },
  {
    type: 'function',
    inputs: [],
    name: 'argonToken',
    outputs: [{ name: '', internalType: 'address', type: 'address' }],
    stateMutability: 'view',
  },
  {
    type: 'function',
    inputs: [],
    name: 'argonotToken',
    outputs: [{ name: '', internalType: 'address', type: 'address' }],
    stateMutability: 'view',
  },
  {
    type: 'function',
    inputs: [
      { name: 'token', internalType: 'address', type: 'address' },
      { name: 'amount', internalType: 'uint64', type: 'uint64' },
      { name: 'argonDestination', internalType: 'bytes32', type: 'bytes32' },
      { name: 'deadline', internalType: 'uint256', type: 'uint256' },
      { name: 'v', internalType: 'uint8', type: 'uint8' },
      { name: 'r', internalType: 'bytes32', type: 'bytes32' },
      { name: 's', internalType: 'bytes32', type: 'bytes32' },
    ],
    name: 'burnForTransfer',
    outputs: [],
    stateMutability: 'nonpayable',
  },
  {
    type: 'function',
    inputs: [],
    name: 'guardian',
    outputs: [{ name: '', internalType: 'address', type: 'address' }],
    stateMutability: 'view',
  },
  {
    type: 'function',
    inputs: [
      { name: 'adminSafe', internalType: 'address', type: 'address' },
      { name: 'guardianAddress', internalType: 'address', type: 'address' },
    ],
    name: 'initialize',
    outputs: [],
    stateMutability: 'nonpayable',
  },
  {
    type: 'function',
    inputs: [],
    name: 'owner',
    outputs: [{ name: '', internalType: 'address', type: 'address' }],
    stateMutability: 'view',
  },
  { type: 'function', inputs: [], name: 'pause', outputs: [], stateMutability: 'nonpayable' },
  {
    type: 'function',
    inputs: [],
    name: 'paused',
    outputs: [{ name: '', internalType: 'bool', type: 'bool' }],
    stateMutability: 'view',
  },
  {
    type: 'function',
    inputs: [],
    name: 'renounceOwnership',
    outputs: [],
    stateMutability: 'nonpayable',
  },
  {
    type: 'function',
    inputs: [{ name: 'guardianAddress', internalType: 'address', type: 'address' }],
    name: 'setGuardian',
    outputs: [],
    stateMutability: 'nonpayable',
  },
  {
    type: 'function',
    inputs: [{ name: 'amount', internalType: 'uint64', type: 'uint64' }],
    name: 'toTokenAmount',
    outputs: [{ name: '', internalType: 'uint256', type: 'uint256' }],
    stateMutability: 'pure',
  },
  {
    type: 'function',
    inputs: [{ name: 'newOwner', internalType: 'address', type: 'address' }],
    name: 'transferOwnership',
    outputs: [],
    stateMutability: 'nonpayable',
  },
  { type: 'function', inputs: [], name: 'unpause', outputs: [], stateMutability: 'nonpayable' },
] as const;

//////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////
// MintingGatewayV2
//////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////

export const mintingGatewayV2Abi = [
  {
    type: 'constructor',
    inputs: [
      { name: 'argonTokenAddress', internalType: 'address', type: 'address' },
      { name: 'argonotTokenAddress', internalType: 'address', type: 'address' },
    ],
    stateMutability: 'nonpayable',
  },
  { type: 'error', inputs: [], name: 'ArrayLengthMismatch' },
  { type: 'error', inputs: [], name: 'ECDSAInvalidSignature' },
  {
    type: 'error',
    inputs: [{ name: 'length', internalType: 'uint256', type: 'uint256' }],
    name: 'ECDSAInvalidSignatureLength',
  },
  {
    type: 'error',
    inputs: [{ name: 's', internalType: 'bytes32', type: 'bytes32' }],
    name: 'ECDSAInvalidSignatureS',
  },
  { type: 'error', inputs: [], name: 'EnforcedPause' },
  { type: 'error', inputs: [], name: 'ExpectedPause' },
  { type: 'error', inputs: [], name: 'GlobalIssuanceCouncilNotBootstrapped' },
  { type: 'error', inputs: [], name: 'GlobalIssuanceCouncilQuorumNotMet' },
  {
    type: 'error',
    inputs: [
      { name: 'authorized', internalType: 'uint256', type: 'uint256' },
      { name: 'required', internalType: 'uint256', type: 'uint256' },
    ],
    name: 'InsufficientAuthorizedCollateral',
  },
  {
    type: 'error',
    inputs: [
      { name: 'expected', internalType: 'uint64', type: 'uint64' },
      { name: 'provided', internalType: 'uint64', type: 'uint64' },
    ],
    name: 'InvalidChainId',
  },
  {
    type: 'error',
    inputs: [
      { name: 'expected', internalType: 'bytes32', type: 'bytes32' },
      { name: 'provided', internalType: 'bytes32', type: 'bytes32' },
    ],
    name: 'InvalidCurrentCouncilSnapshot',
  },
  {
    type: 'error',
    inputs: [{ name: 'index', internalType: 'uint256', type: 'uint256' }],
    name: 'InvalidGlobalIssuanceCouncilMember',
  },
  { type: 'error', inputs: [], name: 'InvalidInitialization' },
  { type: 'error', inputs: [], name: 'InvalidMicrogonCollateralForArgonotPayout' },
  {
    type: 'error',
    inputs: [{ name: 'mintingAuthorityId', internalType: 'bytes32', type: 'bytes32' }],
    name: 'InvalidMintingAuthority',
  },
  {
    type: 'error',
    inputs: [
      { name: 'expectedSigner', internalType: 'address', type: 'address' },
      { name: 'recoveredSigner', internalType: 'address', type: 'address' },
    ],
    name: 'InvalidMintingAuthorityDeactivationSigner',
  },
  {
    type: 'error',
    inputs: [
      { name: 'expected', internalType: 'uint64', type: 'uint64' },
      { name: 'provided', internalType: 'uint64', type: 'uint64' },
    ],
    name: 'InvalidQueueNonce',
  },
  {
    type: 'error',
    inputs: [
      { name: 'expected', internalType: 'uint256', type: 'uint256' },
      { name: 'provided', internalType: 'uint256', type: 'uint256' },
    ],
    name: 'InvalidSignatureCount',
  },
  { type: 'error', inputs: [], name: 'InvalidSignatureOrder' },
  {
    type: 'error',
    inputs: [{ name: 'kind', internalType: 'uint8', type: 'uint8' }],
    name: 'InvalidUpdateKind',
  },
  { type: 'error', inputs: [], name: 'LatestGatewayUpdateCannotDeactivate' },
  { type: 'error', inputs: [], name: 'MigrationAlreadyCompleted' },
  {
    type: 'error',
    inputs: [{ name: 'signingKey', internalType: 'address', type: 'address' }],
    name: 'MintingAuthorityAlreadyActive',
  },
  {
    type: 'error',
    inputs: [{ name: 'signingKey', internalType: 'address', type: 'address' }],
    name: 'MintingAuthorityTransferCapacityExceeded',
  },
  {
    type: 'error',
    inputs: [{ name: 'caller', internalType: 'address', type: 'address' }],
    name: 'NotGuardianOrOwner',
  },
  { type: 'error', inputs: [], name: 'NotInitializing' },
  {
    type: 'error',
    inputs: [{ name: 'owner', internalType: 'address', type: 'address' }],
    name: 'OwnableInvalidOwner',
  },
  {
    type: 'error',
    inputs: [{ name: 'account', internalType: 'address', type: 'address' }],
    name: 'OwnableUnauthorizedAccount',
  },
  {
    type: 'error',
    inputs: [{ name: 'amount', internalType: 'uint256', type: 'uint256' }],
    name: 'RuntimeAmountOverflow',
  },
  {
    type: 'error',
    inputs: [{ name: 'transferId', internalType: 'bytes32', type: 'bytes32' }],
    name: 'TransferOutOfArgonAlreadyFinalized',
  },
  {
    type: 'error',
    inputs: [
      { name: 'currentBlock', internalType: 'uint256', type: 'uint256' },
      { name: 'validUntilBlock', internalType: 'uint256', type: 'uint256' },
    ],
    name: 'TransferOutOfArgonExpired',
  },
  {
    type: 'error',
    inputs: [
      { name: 'currentBlock', internalType: 'uint256', type: 'uint256' },
      { name: 'validUntilBlock', internalType: 'uint256', type: 'uint256' },
    ],
    name: 'TransferOutOfArgonNotExpired',
  },
  {
    type: 'error',
    inputs: [{ name: 'councilNumber', internalType: 'uint64', type: 'uint64' }],
    name: 'UnknownCouncilNumber',
  },
  {
    type: 'error',
    inputs: [{ name: 'token', internalType: 'address', type: 'address' }],
    name: 'UnsupportedToken',
  },
  { type: 'error', inputs: [], name: 'ZeroAdminSafe' },
  { type: 'error', inputs: [], name: 'ZeroAmount' },
  { type: 'error', inputs: [], name: 'ZeroGuardian' },
  { type: 'error', inputs: [], name: 'ZeroMicrogonsPerArgonot' },
  {
    type: 'error',
    inputs: [{ name: 'index', internalType: 'uint256', type: 'uint256' }],
    name: 'ZeroRecipient',
  },
  { type: 'error', inputs: [], name: 'ZeroSigningKey' },
  {
    type: 'event',
    anonymous: false,
    inputs: [{ name: 'councilHash', internalType: 'bytes32', type: 'bytes32', indexed: true }],
    name: 'GlobalIssuanceCouncilBootstrapped',
  },
  {
    type: 'event',
    anonymous: false,
    inputs: [
      { name: 'previousCouncilHash', internalType: 'bytes32', type: 'bytes32', indexed: true },
      { name: 'councilHash', internalType: 'bytes32', type: 'bytes32', indexed: true },
    ],
    name: 'GlobalIssuanceCouncilForceUpdated',
  },
  {
    type: 'event',
    anonymous: false,
    inputs: [
      { name: 'councilHash', internalType: 'bytes32', type: 'bytes32', indexed: false },
      { name: 'relayerArgonAccountId', internalType: 'bytes32', type: 'bytes32', indexed: false },
      {
        name: 'gatewayState',
        internalType: 'struct MintingGatewayV2.GatewayActivityState',
        type: 'tuple',
        components: [
          { name: 'gatewayActivityNonce', internalType: 'uint64', type: 'uint64' },
          { name: 'argonApprovalsNonce', internalType: 'uint64', type: 'uint64' },
          { name: 'argonCirculation', internalType: 'uint128', type: 'uint128' },
          { name: 'argonotCirculation', internalType: 'uint128', type: 'uint128' },
        ],
        indexed: false,
      },
    ],
    name: 'GlobalIssuanceCouncilRotated',
  },
  {
    type: 'event',
    anonymous: false,
    inputs: [
      { name: 'previousGuardian', internalType: 'address', type: 'address', indexed: true },
      { name: 'newGuardian', internalType: 'address', type: 'address', indexed: true },
    ],
    name: 'GuardianUpdated',
  },
  {
    type: 'event',
    anonymous: false,
    inputs: [{ name: 'version', internalType: 'uint64', type: 'uint64', indexed: false }],
    name: 'Initialized',
  },
  {
    type: 'event',
    anonymous: false,
    inputs: [
      { name: 'argonRecipientCount', internalType: 'uint256', type: 'uint256', indexed: false },
      { name: 'argonTotalAmount', internalType: 'uint256', type: 'uint256', indexed: false },
      { name: 'argonotRecipientCount', internalType: 'uint256', type: 'uint256', indexed: false },
      { name: 'argonotTotalAmount', internalType: 'uint256', type: 'uint256', indexed: false },
    ],
    name: 'MigrationCompleted',
  },
  {
    type: 'event',
    anonymous: false,
    inputs: [
      { name: 'mintingAuthorityId', internalType: 'bytes32', type: 'bytes32', indexed: true },
      { name: 'microgonCollateral', internalType: 'uint128', type: 'uint128', indexed: false },
      { name: 'micronotCollateral', internalType: 'uint128', type: 'uint128', indexed: false },
      { name: 'signingKey', internalType: 'address', type: 'address', indexed: false },
      { name: 'relayerArgonAccountId', internalType: 'bytes32', type: 'bytes32', indexed: false },
      {
        name: 'gatewayState',
        internalType: 'struct MintingGatewayV2.GatewayActivityState',
        type: 'tuple',
        components: [
          { name: 'gatewayActivityNonce', internalType: 'uint64', type: 'uint64' },
          { name: 'argonApprovalsNonce', internalType: 'uint64', type: 'uint64' },
          { name: 'argonCirculation', internalType: 'uint128', type: 'uint128' },
          { name: 'argonotCirculation', internalType: 'uint128', type: 'uint128' },
        ],
        indexed: false,
      },
    ],
    name: 'MintingAuthorityActivated',
  },
  {
    type: 'event',
    anonymous: false,
    inputs: [
      { name: 'signingKey', internalType: 'address', type: 'address', indexed: true },
      { name: 'microgonCollateral', internalType: 'uint128', type: 'uint128', indexed: false },
      { name: 'micronotCollateral', internalType: 'uint128', type: 'uint128', indexed: false },
      { name: 'relayerArgonAccountId', internalType: 'bytes32', type: 'bytes32', indexed: false },
      {
        name: 'gatewayState',
        internalType: 'struct MintingGatewayV2.GatewayActivityState',
        type: 'tuple',
        components: [
          { name: 'gatewayActivityNonce', internalType: 'uint64', type: 'uint64' },
          { name: 'argonApprovalsNonce', internalType: 'uint64', type: 'uint64' },
          { name: 'argonCirculation', internalType: 'uint128', type: 'uint128' },
          { name: 'argonotCirculation', internalType: 'uint128', type: 'uint128' },
        ],
        indexed: false,
      },
    ],
    name: 'MintingAuthorityDeactivated',
  },
  {
    type: 'event',
    anonymous: false,
    inputs: [
      { name: 'previousOwner', internalType: 'address', type: 'address', indexed: true },
      { name: 'newOwner', internalType: 'address', type: 'address', indexed: true },
    ],
    name: 'OwnershipTransferred',
  },
  {
    type: 'event',
    anonymous: false,
    inputs: [{ name: 'account', internalType: 'address', type: 'address', indexed: false }],
    name: 'Paused',
  },
  {
    type: 'event',
    anonymous: false,
    inputs: [
      { name: 'transferId', internalType: 'bytes32', type: 'bytes32', indexed: false },
      {
        name: 'gatewayState',
        internalType: 'struct MintingGatewayV2.GatewayActivityState',
        type: 'tuple',
        components: [
          { name: 'gatewayActivityNonce', internalType: 'uint64', type: 'uint64' },
          { name: 'argonApprovalsNonce', internalType: 'uint64', type: 'uint64' },
          { name: 'argonCirculation', internalType: 'uint128', type: 'uint128' },
          { name: 'argonotCirculation', internalType: 'uint128', type: 'uint128' },
        ],
        indexed: false,
      },
    ],
    name: 'TransferOutOfArgonCanceled',
  },
  {
    type: 'event',
    anonymous: false,
    inputs: [
      { name: 'transferId', internalType: 'bytes32', type: 'bytes32', indexed: false },
      {
        name: 'mintingCollateral',
        internalType: 'struct MintingGatewayV2.MintingAuthorityCollateral[]',
        type: 'tuple[]',
        components: [
          { name: 'signingKey', internalType: 'address', type: 'address' },
          { name: 'microgonCollateral', internalType: 'uint128', type: 'uint128' },
          { name: 'micronotCollateral', internalType: 'uint128', type: 'uint128' },
        ],
        indexed: false,
      },
      {
        name: 'gatewayState',
        internalType: 'struct MintingGatewayV2.GatewayActivityState',
        type: 'tuple',
        components: [
          { name: 'gatewayActivityNonce', internalType: 'uint64', type: 'uint64' },
          { name: 'argonApprovalsNonce', internalType: 'uint64', type: 'uint64' },
          { name: 'argonCirculation', internalType: 'uint128', type: 'uint128' },
          { name: 'argonotCirculation', internalType: 'uint128', type: 'uint128' },
        ],
        indexed: false,
      },
    ],
    name: 'TransferOutOfArgonFinalized',
  },
  {
    type: 'event',
    anonymous: false,
    inputs: [
      { name: 'from', internalType: 'address', type: 'address', indexed: true },
      { name: 'token', internalType: 'address', type: 'address', indexed: true },
      { name: 'amount', internalType: 'uint128', type: 'uint128', indexed: false },
      { name: 'argonAccountId', internalType: 'bytes32', type: 'bytes32', indexed: false },
      {
        name: 'gatewayState',
        internalType: 'struct MintingGatewayV2.GatewayActivityState',
        type: 'tuple',
        components: [
          { name: 'gatewayActivityNonce', internalType: 'uint64', type: 'uint64' },
          { name: 'argonApprovalsNonce', internalType: 'uint64', type: 'uint64' },
          { name: 'argonCirculation', internalType: 'uint128', type: 'uint128' },
          { name: 'argonotCirculation', internalType: 'uint128', type: 'uint128' },
        ],
        indexed: false,
      },
    ],
    name: 'TransferToArgonStarted',
  },
  {
    type: 'event',
    anonymous: false,
    inputs: [{ name: 'account', internalType: 'address', type: 'address', indexed: false }],
    name: 'Unpaused',
  },
  {
    type: 'function',
    inputs: [],
    name: 'RUNTIME_DECIMALS',
    outputs: [{ name: '', internalType: 'uint8', type: 'uint8' }],
    stateMutability: 'view',
  },
  {
    type: 'function',
    inputs: [],
    name: 'RUNTIME_TO_ERC20_SCALE',
    outputs: [{ name: '', internalType: 'uint256', type: 'uint256' }],
    stateMutability: 'view',
  },
  {
    type: 'function',
    inputs: [],
    name: 'TOKEN_DECIMALS',
    outputs: [{ name: '', internalType: 'uint8', type: 'uint8' }],
    stateMutability: 'view',
  },
  {
    type: 'function',
    inputs: [{ name: 'locatorIndex', internalType: 'uint64', type: 'uint64' }],
    name: 'activityBlockLocators',
    outputs: [
      { name: 'blockNumber', internalType: 'uint64', type: 'uint64' },
      { name: 'startGatewayActivityNonce', internalType: 'uint64', type: 'uint64' },
      { name: 'endGatewayActivityNonce', internalType: 'uint64', type: 'uint64' },
    ],
    stateMutability: 'view',
  },
  {
    type: 'function',
    inputs: [
      {
        name: 'currentCouncil',
        internalType: 'struct MintingGatewayV2.CouncilSnapshot',
        type: 'tuple',
        components: [
          { name: 'signers', internalType: 'address[]', type: 'address[]' },
          { name: 'weights', internalType: 'uint256[]', type: 'uint256[]' },
        ],
      },
      {
        name: 'updates',
        internalType: 'struct MintingGatewayV2.GatewayUpdate[]',
        type: 'tuple[]',
        components: [
          { name: 'queueNonce', internalType: 'uint64', type: 'uint64' },
          { name: 'kind', internalType: 'enum MintingGatewayV2.GatewayUpdateKind', type: 'uint8' },
          { name: 'payload', internalType: 'bytes', type: 'bytes' },
          { name: 'signatures', internalType: 'bytes[]', type: 'bytes[]' },
        ],
      },
      { name: 'relayerArgonAccountId', internalType: 'bytes32', type: 'bytes32' },
    ],
    name: 'applyGatewayUpdates',
    outputs: [],
    stateMutability: 'nonpayable',
  },
  {
    type: 'function',
    inputs: [],
    name: 'argonApprovalsHash',
    outputs: [{ name: '', internalType: 'bytes32', type: 'bytes32' }],
    stateMutability: 'view',
  },
  {
    type: 'function',
    inputs: [],
    name: 'argonApprovalsNonce',
    outputs: [{ name: '', internalType: 'uint64', type: 'uint64' }],
    stateMutability: 'view',
  },
  {
    type: 'function',
    inputs: [],
    name: 'argonCirculation',
    outputs: [{ name: '', internalType: 'uint128', type: 'uint128' }],
    stateMutability: 'view',
  },
  {
    type: 'function',
    inputs: [],
    name: 'argonToken',
    outputs: [{ name: '', internalType: 'address', type: 'address' }],
    stateMutability: 'view',
  },
  {
    type: 'function',
    inputs: [],
    name: 'argonotCirculation',
    outputs: [{ name: '', internalType: 'uint128', type: 'uint128' }],
    stateMutability: 'view',
  },
  {
    type: 'function',
    inputs: [],
    name: 'argonotToken',
    outputs: [{ name: '', internalType: 'address', type: 'address' }],
    stateMutability: 'view',
  },
  {
    type: 'function',
    inputs: [
      {
        name: 'request',
        internalType: 'struct MintingGatewayV2.TransferOutOfArgonRequest',
        type: 'tuple',
        components: [
          { name: 'argonAccountId', internalType: 'bytes32', type: 'bytes32' },
          { name: 'argonTransferNonce', internalType: 'uint64', type: 'uint64' },
          { name: 'chainId', internalType: 'uint64', type: 'uint64' },
          { name: 'councilNumber', internalType: 'uint64', type: 'uint64' },
          { name: 'recipient', internalType: 'address', type: 'address' },
          { name: 'validUntilBlock', internalType: 'uint64', type: 'uint64' },
          { name: 'token', internalType: 'address', type: 'address' },
          { name: 'amount', internalType: 'uint128', type: 'uint128' },
          { name: 'finalizationTip', internalType: 'uint128', type: 'uint128' },
        ],
      },
    ],
    name: 'cancelTransferOutOfArgon',
    outputs: [],
    stateMutability: 'nonpayable',
  },
  {
    type: 'function',
    inputs: [
      {
        name: 'request',
        internalType: 'struct MintingGatewayV2.TransferOutOfArgonRequest',
        type: 'tuple',
        components: [
          { name: 'argonAccountId', internalType: 'bytes32', type: 'bytes32' },
          { name: 'argonTransferNonce', internalType: 'uint64', type: 'uint64' },
          { name: 'chainId', internalType: 'uint64', type: 'uint64' },
          { name: 'councilNumber', internalType: 'uint64', type: 'uint64' },
          { name: 'recipient', internalType: 'address', type: 'address' },
          { name: 'validUntilBlock', internalType: 'uint64', type: 'uint64' },
          { name: 'token', internalType: 'address', type: 'address' },
          { name: 'amount', internalType: 'uint128', type: 'uint128' },
          { name: 'finalizationTip', internalType: 'uint128', type: 'uint128' },
        ],
      },
      {
        name: 'proof',
        internalType: 'struct MintingGatewayV2.TransferOutOfArgonProof',
        type: 'tuple',
        components: [
          {
            name: 'authorizations',
            internalType: 'struct MintingGatewayV2.MintingAuthorization[]',
            type: 'tuple[]',
            components: [
              { name: 'microgonCollateral', internalType: 'uint128', type: 'uint128' },
              { name: 'micronotCollateral', internalType: 'uint128', type: 'uint128' },
              { name: 'signature', internalType: 'bytes', type: 'bytes' },
            ],
          },
        ],
      },
    ],
    name: 'finalizeTransferOutOfArgon',
    outputs: [],
    stateMutability: 'nonpayable',
  },
  {
    type: 'function',
    inputs: [{ name: 'transferId', internalType: 'bytes32', type: 'bytes32' }],
    name: 'finalizedTransferOutOfArgonIds',
    outputs: [{ name: 'isFinalized', internalType: 'bool', type: 'bool' }],
    stateMutability: 'view',
  },
  {
    type: 'function',
    inputs: [
      {
        name: 'nextCouncil',
        internalType: 'struct MintingGatewayV2.CouncilSnapshot',
        type: 'tuple',
        components: [
          { name: 'signers', internalType: 'address[]', type: 'address[]' },
          { name: 'weights', internalType: 'uint256[]', type: 'uint256[]' },
        ],
      },
    ],
    name: 'forceUpdateActiveCouncil',
    outputs: [],
    stateMutability: 'nonpayable',
  },
  {
    type: 'function',
    inputs: [],
    name: 'gatewayActivityNonce',
    outputs: [{ name: '', internalType: 'uint64', type: 'uint64' }],
    stateMutability: 'view',
  },
  {
    type: 'function',
    inputs: [],
    name: 'globalIssuanceCouncil',
    outputs: [
      { name: 'totalWeight', internalType: 'uint256', type: 'uint256' },
      { name: 'memberCount', internalType: 'uint256', type: 'uint256' },
      { name: 'councilHash', internalType: 'bytes32', type: 'bytes32' },
      { name: 'councilNumber', internalType: 'uint64', type: 'uint64' },
    ],
    stateMutability: 'view',
  },
  {
    type: 'function',
    inputs: [],
    name: 'guardian',
    outputs: [{ name: '', internalType: 'address', type: 'address' }],
    stateMutability: 'view',
  },
  {
    type: 'function',
    inputs: [
      { name: 'adminSafe', internalType: 'address', type: 'address' },
      { name: 'guardianAddress', internalType: 'address', type: 'address' },
      { name: 'councilHash', internalType: 'bytes32', type: 'bytes32' },
      { name: 'councilMemberCount', internalType: 'uint256', type: 'uint256' },
      { name: 'councilTotalWeight', internalType: 'uint256', type: 'uint256' },
      { name: 'initialMicrogonsPerArgonot', internalType: 'uint128', type: 'uint128' },
    ],
    name: 'initialize',
    outputs: [],
    stateMutability: 'nonpayable',
  },
  {
    type: 'function',
    inputs: [],
    name: 'latestActivityBlockLocatorIndex',
    outputs: [{ name: '', internalType: 'uint64', type: 'uint64' }],
    stateMutability: 'view',
  },
  {
    type: 'function',
    inputs: [],
    name: 'microgonsPerArgonot',
    outputs: [{ name: '', internalType: 'uint128', type: 'uint128' }],
    stateMutability: 'view',
  },
  {
    type: 'function',
    inputs: [
      {
        name: 'argonMigration',
        internalType: 'struct MintingGatewayV2.MigrationAssetDistribution',
        type: 'tuple',
        components: [
          { name: 'recipients', internalType: 'address[]', type: 'address[]' },
          { name: 'amounts', internalType: 'uint256[]', type: 'uint256[]' },
        ],
      },
      {
        name: 'argonotMigration',
        internalType: 'struct MintingGatewayV2.MigrationAssetDistribution',
        type: 'tuple',
        components: [
          { name: 'recipients', internalType: 'address[]', type: 'address[]' },
          { name: 'amounts', internalType: 'uint256[]', type: 'uint256[]' },
        ],
      },
    ],
    name: 'migrate',
    outputs: [],
    stateMutability: 'nonpayable',
  },
  {
    type: 'function',
    inputs: [],
    name: 'migrationCompleted',
    outputs: [{ name: '', internalType: 'bool', type: 'bool' }],
    stateMutability: 'view',
  },
  {
    type: 'function',
    inputs: [{ name: 'signingKey', internalType: 'address', type: 'address' }],
    name: 'mintingAuthorityCollateralRemaining',
    outputs: [
      { name: 'microgonCollateral', internalType: 'uint128', type: 'uint128' },
      { name: 'micronotCollateral', internalType: 'uint128', type: 'uint128' },
    ],
    stateMutability: 'view',
  },
  {
    type: 'function',
    inputs: [],
    name: 'owner',
    outputs: [{ name: '', internalType: 'address', type: 'address' }],
    stateMutability: 'view',
  },
  { type: 'function', inputs: [], name: 'pause', outputs: [], stateMutability: 'nonpayable' },
  {
    type: 'function',
    inputs: [],
    name: 'paused',
    outputs: [{ name: '', internalType: 'bool', type: 'bool' }],
    stateMutability: 'view',
  },
  {
    type: 'function',
    inputs: [],
    name: 'previousMicrogonsPerArgonot',
    outputs: [{ name: '', internalType: 'uint128', type: 'uint128' }],
    stateMutability: 'view',
  },
  {
    type: 'function',
    inputs: [],
    name: 'renounceOwnership',
    outputs: [],
    stateMutability: 'nonpayable',
  },
  {
    type: 'function',
    inputs: [{ name: 'guardianAddress', internalType: 'address', type: 'address' }],
    name: 'setGuardian',
    outputs: [],
    stateMutability: 'nonpayable',
  },
  {
    type: 'function',
    inputs: [
      { name: 'token', internalType: 'address', type: 'address' },
      { name: 'amount', internalType: 'uint128', type: 'uint128' },
      { name: 'argonAccountId', internalType: 'bytes32', type: 'bytes32' },
      { name: 'deadline', internalType: 'uint256', type: 'uint256' },
      { name: 'v', internalType: 'uint8', type: 'uint8' },
      { name: 'r', internalType: 'bytes32', type: 'bytes32' },
      { name: 's', internalType: 'bytes32', type: 'bytes32' },
    ],
    name: 'startTransferToArgon',
    outputs: [],
    stateMutability: 'nonpayable',
  },
  {
    type: 'function',
    inputs: [{ name: 'amount', internalType: 'uint128', type: 'uint128' }],
    name: 'toTokenAmount',
    outputs: [{ name: '', internalType: 'uint256', type: 'uint256' }],
    stateMutability: 'pure',
  },
  {
    type: 'function',
    inputs: [{ name: 'newOwner', internalType: 'address', type: 'address' }],
    name: 'transferOwnership',
    outputs: [],
    stateMutability: 'nonpayable',
  },
  { type: 'function', inputs: [], name: 'unpause', outputs: [], stateMutability: 'nonpayable' },
] as const;

//////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////
// ProxyAdmin
//////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////

export const proxyAdminAbi = [
  {
    type: 'constructor',
    inputs: [{ name: 'initialOwner', internalType: 'address', type: 'address' }],
    stateMutability: 'nonpayable',
  },
  {
    type: 'error',
    inputs: [{ name: 'owner', internalType: 'address', type: 'address' }],
    name: 'OwnableInvalidOwner',
  },
  {
    type: 'error',
    inputs: [{ name: 'account', internalType: 'address', type: 'address' }],
    name: 'OwnableUnauthorizedAccount',
  },
  {
    type: 'event',
    anonymous: false,
    inputs: [
      { name: 'previousOwner', internalType: 'address', type: 'address', indexed: true },
      { name: 'newOwner', internalType: 'address', type: 'address', indexed: true },
    ],
    name: 'OwnershipTransferred',
  },
  {
    type: 'function',
    inputs: [],
    name: 'UPGRADE_INTERFACE_VERSION',
    outputs: [{ name: '', internalType: 'string', type: 'string' }],
    stateMutability: 'view',
  },
  {
    type: 'function',
    inputs: [],
    name: 'owner',
    outputs: [{ name: '', internalType: 'address', type: 'address' }],
    stateMutability: 'view',
  },
  {
    type: 'function',
    inputs: [],
    name: 'renounceOwnership',
    outputs: [],
    stateMutability: 'nonpayable',
  },
  {
    type: 'function',
    inputs: [{ name: 'newOwner', internalType: 'address', type: 'address' }],
    name: 'transferOwnership',
    outputs: [],
    stateMutability: 'nonpayable',
  },
  {
    type: 'function',
    inputs: [
      { name: 'proxy', internalType: 'contract ITransparentUpgradeableProxy', type: 'address' },
      { name: 'implementation', internalType: 'address', type: 'address' },
      { name: 'data', internalType: 'bytes', type: 'bytes' },
    ],
    name: 'upgradeAndCall',
    outputs: [],
    stateMutability: 'payable',
  },
] as const;

//////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////
// TransparentUpgradeableProxy
//////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////

export const transparentUpgradeableProxyAbi = [
  {
    type: 'constructor',
    inputs: [
      { name: 'logic', internalType: 'address', type: 'address' },
      { name: 'initialOwner', internalType: 'address', type: 'address' },
      { name: 'data', internalType: 'bytes', type: 'bytes' },
    ],
    stateMutability: 'payable',
  },
  {
    type: 'error',
    inputs: [{ name: 'target', internalType: 'address', type: 'address' }],
    name: 'AddressEmptyCode',
  },
  {
    type: 'error',
    inputs: [{ name: 'admin', internalType: 'address', type: 'address' }],
    name: 'ERC1967InvalidAdmin',
  },
  {
    type: 'error',
    inputs: [{ name: 'implementation', internalType: 'address', type: 'address' }],
    name: 'ERC1967InvalidImplementation',
  },
  { type: 'error', inputs: [], name: 'ERC1967NonPayable' },
  { type: 'error', inputs: [], name: 'ERC1967ProxyUninitialized' },
  { type: 'error', inputs: [], name: 'FailedCall' },
  { type: 'error', inputs: [], name: 'ProxyDeniedAdminAccess' },
  {
    type: 'event',
    anonymous: false,
    inputs: [
      { name: 'previousAdmin', internalType: 'address', type: 'address', indexed: false },
      { name: 'newAdmin', internalType: 'address', type: 'address', indexed: false },
    ],
    name: 'AdminChanged',
  },
  {
    type: 'event',
    anonymous: false,
    inputs: [{ name: 'implementation', internalType: 'address', type: 'address', indexed: true }],
    name: 'Upgraded',
  },
  { type: 'fallback', stateMutability: 'payable' },
] as const;
