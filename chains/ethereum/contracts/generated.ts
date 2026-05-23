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
    inputs: [{ name: 'signingKey', internalType: 'address', type: 'address' }],
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
    inputs: [{ name: 'councilHash', internalType: 'bytes32', type: 'bytes32' }],
    name: 'UnknownCouncilHash',
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
        internalType: 'struct MintingGateway.GatewayActivityState',
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
      { name: 'signingKey', internalType: 'address', type: 'address', indexed: true },
      { name: 'microgonCollateral', internalType: 'uint128', type: 'uint128', indexed: false },
      { name: 'micronotCollateral', internalType: 'uint128', type: 'uint128', indexed: false },
      { name: 'coactivationCount', internalType: 'uint32', type: 'uint32', indexed: false },
      { name: 'sharedSignatureCount', internalType: 'uint32', type: 'uint32', indexed: false },
      { name: 'relayerArgonAccountId', internalType: 'bytes32', type: 'bytes32', indexed: false },
      {
        name: 'gatewayState',
        internalType: 'struct MintingGateway.GatewayActivityState',
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
        internalType: 'struct MintingGateway.GatewayActivityState',
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
        internalType: 'struct MintingGateway.GatewayActivityState',
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
      { name: 'token', internalType: 'address', type: 'address', indexed: false },
      { name: 'amount', internalType: 'uint128', type: 'uint128', indexed: false },
      {
        name: 'mintingCollateral',
        internalType: 'struct MintingGateway.MintingAuthorityCollateral[]',
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
        internalType: 'struct MintingGateway.GatewayActivityState',
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
        internalType: 'struct MintingGateway.GatewayActivityState',
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
        internalType: 'struct MintingGateway.CouncilSnapshot',
        type: 'tuple',
        components: [
          { name: 'signers', internalType: 'address[]', type: 'address[]' },
          { name: 'weights', internalType: 'uint256[]', type: 'uint256[]' },
        ],
      },
      {
        name: 'updates',
        internalType: 'struct MintingGateway.GatewayUpdate[]',
        type: 'tuple[]',
        components: [
          { name: 'queueNonce', internalType: 'uint64', type: 'uint64' },
          { name: 'kind', internalType: 'enum MintingGateway.GatewayUpdateKind', type: 'uint8' },
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
        internalType: 'struct MintingGateway.TransferOutOfArgonRequest',
        type: 'tuple',
        components: [
          { name: 'argonAccountId', internalType: 'bytes32', type: 'bytes32' },
          { name: 'argonTransferNonce', internalType: 'uint64', type: 'uint64' },
          { name: 'chainId', internalType: 'uint64', type: 'uint64' },
          { name: 'councilHash', internalType: 'bytes32', type: 'bytes32' },
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
        internalType: 'struct MintingGateway.TransferOutOfArgonRequest',
        type: 'tuple',
        components: [
          { name: 'argonAccountId', internalType: 'bytes32', type: 'bytes32' },
          { name: 'argonTransferNonce', internalType: 'uint64', type: 'uint64' },
          { name: 'chainId', internalType: 'uint64', type: 'uint64' },
          { name: 'councilHash', internalType: 'bytes32', type: 'bytes32' },
          { name: 'recipient', internalType: 'address', type: 'address' },
          { name: 'validUntilBlock', internalType: 'uint64', type: 'uint64' },
          { name: 'token', internalType: 'address', type: 'address' },
          { name: 'amount', internalType: 'uint128', type: 'uint128' },
          { name: 'finalizationTip', internalType: 'uint128', type: 'uint128' },
        ],
      },
      {
        name: 'proof',
        internalType: 'struct MintingGateway.TransferOutOfArgonProof',
        type: 'tuple',
        components: [
          {
            name: 'authorizations',
            internalType: 'struct MintingGateway.MintingAuthorization[]',
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
        internalType: 'struct MintingGateway.CouncilSnapshot',
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
      { name: 'epochMicrogonsPerArgonot', internalType: 'uint128', type: 'uint128' },
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
    inputs: [
      {
        name: 'argonMigration',
        internalType: 'struct MintingGateway.MigrationAssetDistribution',
        type: 'tuple',
        components: [
          { name: 'recipients', internalType: 'address[]', type: 'address[]' },
          { name: 'amounts', internalType: 'uint256[]', type: 'uint256[]' },
        ],
      },
      {
        name: 'argonotMigration',
        internalType: 'struct MintingGateway.MigrationAssetDistribution',
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
    name: 'previousGlobalIssuanceCouncilHash',
    outputs: [{ name: '', internalType: 'bytes32', type: 'bytes32' }],
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

//////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////
// EventMetadataConstants
//////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////

import type { Hex } from 'viem';

export type ArgonTokenApproval = {
  owner: Hex;
  spender: Hex;
  value: bigint;
};

export type ArgonTokenEIP712DomainChanged = {};

export type ArgonTokenTransfer = {
  from: Hex;
  to: Hex;
  value: bigint;
};

export type ArgonotTokenApproval = {
  owner: Hex;
  spender: Hex;
  value: bigint;
};

export type ArgonotTokenEIP712DomainChanged = {};

export type ArgonotTokenTransfer = {
  from: Hex;
  to: Hex;
  value: bigint;
};

export type CanonicalMintableBurnableERC20Approval = {
  owner: Hex;
  spender: Hex;
  value: bigint;
};

export type CanonicalMintableBurnableERC20EIP712DomainChanged = {};

export type CanonicalMintableBurnableERC20Transfer = {
  from: Hex;
  to: Hex;
  value: bigint;
};

export type ICanonicalTokenApproval = {
  owner: Hex;
  spender: Hex;
  value: bigint;
};

export type ICanonicalTokenTransfer = {
  from: Hex;
  to: Hex;
  value: bigint;
};

export type MintingGatewayGlobalIssuanceCouncilBootstrapped = {
  councilHash: Hex;
};

export type MintingGatewayGlobalIssuanceCouncilForceUpdated = {
  previousCouncilHash: Hex;
  councilHash: Hex;
};

export type MintingGatewayActivityState = {
  gatewayActivityNonce: bigint;
  argonApprovalsNonce: bigint;
  argonCirculation: bigint;
  argonotCirculation: bigint;
};

export type MintingGatewayGlobalIssuanceCouncilRotated = {
  councilHash: Hex;
  relayerArgonAccountId: Hex;
  gatewayState: MintingGatewayActivityState;
};

export type MintingGatewayGuardianUpdated = {
  previousGuardian: Hex;
  newGuardian: Hex;
};

export type MintingGatewayInitialized = {
  version: bigint;
};

export type MintingGatewayMigrationCompleted = {
  argonRecipientCount: bigint;
  argonTotalAmount: bigint;
  argonotRecipientCount: bigint;
  argonotTotalAmount: bigint;
};

export type MintingGatewayMintingAuthorityActivated = {
  signingKey: Hex;
  microgonCollateral: bigint;
  micronotCollateral: bigint;
  coactivationCount: bigint;
  sharedSignatureCount: bigint;
  relayerArgonAccountId: Hex;
  gatewayState: MintingGatewayActivityState;
};

export type MintingGatewayMintingAuthorityDeactivated = {
  signingKey: Hex;
  microgonCollateral: bigint;
  micronotCollateral: bigint;
  relayerArgonAccountId: Hex;
  gatewayState: MintingGatewayActivityState;
};

export type MintingGatewayOwnershipTransferred = {
  previousOwner: Hex;
  newOwner: Hex;
};

export type MintingGatewayPaused = {
  account: Hex;
};

export type MintingGatewayTransferOutOfArgonCanceled = {
  transferId: Hex;
  gatewayState: MintingGatewayActivityState;
};

export type MintingGatewayMintingAuthorityCollateral = {
  signingKey: Hex;
  microgonCollateral: bigint;
  micronotCollateral: bigint;
};

export type MintingGatewayTransferOutOfArgonFinalized = {
  transferId: Hex;
  token: Hex;
  amount: bigint;
  mintingCollateral: readonly MintingGatewayMintingAuthorityCollateral[];
  gatewayState: MintingGatewayActivityState;
};

export type MintingGatewayTransferToArgonStarted = {
  from: Hex;
  token: Hex;
  amount: bigint;
  argonAccountId: Hex;
  gatewayState: MintingGatewayActivityState;
};

export type MintingGatewayUnpaused = {
  account: Hex;
};

export type ProxyAdminOwnershipTransferred = {
  previousOwner: Hex;
  newOwner: Hex;
};

export type TransparentUpgradeableProxyAdminChanged = {
  previousAdmin: Hex;
  newAdmin: Hex;
};

export type TransparentUpgradeableProxyUpgraded = {
  implementation: Hex;
};

export const ArgonTokenEvents = {
  Approval: {
    name: 'Approval',
    topic: '0x8c5be1e5ebec7d5bd14f71427d1e84f3dd0314c0f7b2291e5b200ac8c7c3b925',
  },
  EIP712DomainChanged: {
    name: 'EIP712DomainChanged',
    topic: '0x0a6387c9ea3628b88a633bb4f3b151770f70085117a15f9bf3787cda53f13d31',
  },
  Transfer: {
    name: 'Transfer',
    topic: '0xddf252ad1be2c89b69c2b068fc378daa952ba7f163c4a11628f55a4df523b3ef',
  },
} as const;

export const ArgonotTokenEvents = {
  Approval: {
    name: 'Approval',
    topic: '0x8c5be1e5ebec7d5bd14f71427d1e84f3dd0314c0f7b2291e5b200ac8c7c3b925',
  },
  EIP712DomainChanged: {
    name: 'EIP712DomainChanged',
    topic: '0x0a6387c9ea3628b88a633bb4f3b151770f70085117a15f9bf3787cda53f13d31',
  },
  Transfer: {
    name: 'Transfer',
    topic: '0xddf252ad1be2c89b69c2b068fc378daa952ba7f163c4a11628f55a4df523b3ef',
  },
} as const;

export const CanonicalMintableBurnableERC20Events = {
  Approval: {
    name: 'Approval',
    topic: '0x8c5be1e5ebec7d5bd14f71427d1e84f3dd0314c0f7b2291e5b200ac8c7c3b925',
  },
  EIP712DomainChanged: {
    name: 'EIP712DomainChanged',
    topic: '0x0a6387c9ea3628b88a633bb4f3b151770f70085117a15f9bf3787cda53f13d31',
  },
  Transfer: {
    name: 'Transfer',
    topic: '0xddf252ad1be2c89b69c2b068fc378daa952ba7f163c4a11628f55a4df523b3ef',
  },
} as const;

export const ICanonicalTokenEvents = {
  Approval: {
    name: 'Approval',
    topic: '0x8c5be1e5ebec7d5bd14f71427d1e84f3dd0314c0f7b2291e5b200ac8c7c3b925',
  },
  Transfer: {
    name: 'Transfer',
    topic: '0xddf252ad1be2c89b69c2b068fc378daa952ba7f163c4a11628f55a4df523b3ef',
  },
} as const;

export const MintingGatewayEvents = {
  GlobalIssuanceCouncilBootstrapped: {
    name: 'GlobalIssuanceCouncilBootstrapped',
    topic: '0xac3566d46c25f65b48d269870da4b9349704c79de13ad49b115532834d7e7ed1',
  },
  GlobalIssuanceCouncilForceUpdated: {
    name: 'GlobalIssuanceCouncilForceUpdated',
    topic: '0x33e53e90d61fdc868629a6234670f92c4445196f3f98ebd71935bbc76c7d9153',
  },
  GlobalIssuanceCouncilRotated: {
    name: 'GlobalIssuanceCouncilRotated',
    topic: '0xb3dbfeab9c5013403dc2f2300318b92643b1cb9323077d549b4f93bbddded963',
  },
  GuardianUpdated: {
    name: 'GuardianUpdated',
    topic: '0x064d28d3d3071c5cbc271a261c10c2f0f0d9e319390397101aa0eb23c6bad909',
  },
  Initialized: {
    name: 'Initialized',
    topic: '0xc7f505b2f371ae2175ee4913f4499e1f2633a7b5936321eed1cdaeb6115181d2',
  },
  MigrationCompleted: {
    name: 'MigrationCompleted',
    topic: '0xc255ddc45163282f24909556f7475ab4ec8eb1d51763b25a4560cee9f43d0645',
  },
  MintingAuthorityActivated: {
    name: 'MintingAuthorityActivated',
    topic: '0xc0f81072946eba75f62302dad0fe56dd420d0b9d311e15b61bbe6e056c29908d',
  },
  MintingAuthorityDeactivated: {
    name: 'MintingAuthorityDeactivated',
    topic: '0xf92f117a698345a1b681cf04280fb5da65a05a9ac88aaacad3e9ee273ce570fd',
  },
  OwnershipTransferred: {
    name: 'OwnershipTransferred',
    topic: '0x8be0079c531659141344cd1fd0a4f28419497f9722a3daafe3b4186f6b6457e0',
  },
  Paused: {
    name: 'Paused',
    topic: '0x62e78cea01bee320cd4e420270b5ea74000d11b0c9f74754ebdbfc544b05a258',
  },
  TransferOutOfArgonCanceled: {
    name: 'TransferOutOfArgonCanceled',
    topic: '0x3cb8789e9bf6f532eec2bc8ffee9b622fc0d6e6daefb7fb6bb2e2f7b2d62e452',
  },
  TransferOutOfArgonFinalized: {
    name: 'TransferOutOfArgonFinalized',
    topic: '0x64d79900ed82270f744ae62db527f4e328e336d8b51e77bc72601391fe779d06',
  },
  TransferToArgonStarted: {
    name: 'TransferToArgonStarted',
    topic: '0x3ab970a848f33363427e47cfbd7a627af0c6e72757f150137e38cef3cee24d93',
  },
  Unpaused: {
    name: 'Unpaused',
    topic: '0x5db9ee0a495bf2e6ff9c91a7834c1ba4fdd244a5e8aa4e537bd38aeae4b073aa',
  },
} as const;

export const ProxyAdminEvents = {
  OwnershipTransferred: {
    name: 'OwnershipTransferred',
    topic: '0x8be0079c531659141344cd1fd0a4f28419497f9722a3daafe3b4186f6b6457e0',
  },
} as const;

export const TransparentUpgradeableProxyEvents = {
  AdminChanged: {
    name: 'AdminChanged',
    topic: '0x7e644d79422f17c01e4894b5f4f588d331ebfa28653d42ae832dc59e38c9798f',
  },
  Upgraded: {
    name: 'Upgraded',
    topic: '0xbc7cd75a20ee27fd9adebab32041f755214dbc6bffa90cc0225b39da2e5c2d3b',
  },
} as const;
