export type EthereumBeaconHeader = {
  slot: string;
  proposer_index: string;
  parent_root: string;
  state_root: string;
  body_root: string;
};

export type EthereumBeaconBlockBody = Record<string, unknown> & {
  execution_payload: EthereumBeaconExecutionPayload;
};

export type EthereumBeaconExecutionPayload = {
  parent_hash: string;
  fee_recipient: string;
  state_root: string;
  receipts_root: string;
  logs_bloom: string;
  prev_randao: string;
  block_number: string;
  gas_limit: string;
  gas_used: string;
  timestamp: string;
  extra_data: string;
  base_fee_per_gas: string;
  block_hash: string;
  transactions: string[];
  withdrawals: Record<string, unknown>[];
  blob_gas_used?: string;
  excess_blob_gas?: string;
};

export type EthereumLightClientHeader = {
  beacon: EthereumBeaconHeader;
  execution: {
    parent_hash: string;
    fee_recipient: string;
    state_root: string;
    receipts_root: string;
    logs_bloom: string;
    prev_randao: string;
    block_number: string;
    gas_limit: string;
    gas_used: string;
    timestamp: string;
    extra_data: string;
    base_fee_per_gas: string;
    block_hash: string;
    transactions_root: string;
    withdrawals_root: string;
    blob_gas_used?: string;
    excess_blob_gas?: string;
  };
  execution_branch: string[];
};

export type EthereumLightClientBootstrapResponse = {
  data: {
    header: EthereumLightClientHeader;
    current_sync_committee: {
      pubkeys: string[];
      aggregate_pubkey: string;
    };
    current_sync_committee_branch: string[];
  };
};

export type EthereumLightClientUpdate = {
  attested_header: EthereumLightClientHeader;
  sync_aggregate: {
    sync_committee_bits: string;
    sync_committee_signature: string;
  };
  signature_slot: string;
  next_sync_committee?: EthereumLightClientBootstrapResponse['data']['current_sync_committee'];
  next_sync_committee_branch?: string[];
  finalized_header: EthereumLightClientHeader;
  finality_branch: string[];
};

export type EthereumBeaconGenesisResponse = {
  data: {
    genesis_fork_version: string;
    genesis_validators_root: string;
  };
};

export type EthereumBeaconConfigSpecResponse = {
  data: Record<string, string>;
};

export type EthereumBeaconHeaderDetailsResponse = {
  data: {
    root: string;
    canonical: boolean;
    header: {
      message: EthereumBeaconHeader;
      signature: string;
    };
  };
};

export type EthereumLightClientUpdatesResponse = {
  data: EthereumLightClientUpdate[];
};

export type EthereumBeaconBlockResponse = {
  version: string;
  data: {
    message: {
      slot: string;
      proposer_index: string;
      parent_root: string;
      state_root: string;
      body: EthereumBeaconBlockBody;
    };
  };
};
