CREATE TABLE IF NOT EXISTS accounts
(
    id                     INTEGER  NOT NULL PRIMARY KEY AUTOINCREMENT,
    address                TEXT     NOT NULL,
    hd_path                TEXT, -- the derivation path of the account if applicable
    account_id32           BLOB     NOT NULL,
    account_type           INT      NOT NULL,
    notary_id              INT      NOT NULL,
    origin_notebook_number INT,
    origin_uid             INT,
    created_at             DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at             DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE UNIQUE INDEX IF NOT EXISTS accounts_address_idx ON accounts (address, account_type, notary_id);
-- only one tax/deposit per hd_path per notary. Meaning, only a single primary account allowed
CREATE UNIQUE INDEX IF NOT EXISTS accounts_path_idx ON accounts (account_type, notary_id, hd_path);

-- optional embedded key store.
CREATE TABLE IF NOT EXISTS key
(
    address     TEXT NOT NULL PRIMARY KEY,
    crypto_type INT  NOT NULL,
    data        BLOB NOT NULL
);

-- The notarizations stores the notarization json and the new account origins json if the notarization was done by this wallet.
-- If the notarization was done by another wallet, the notarization will be populated once it is found in a wallet for any pending balance changes.
CREATE TABLE IF NOT EXISTS notarizations
(
    id              INTEGER  NOT NULL PRIMARY KEY AUTOINCREMENT,
    json            TEXT     NOT NULL,
    notary_id       INT      NOT NULL,
    notebook_number INT,
    tick            INT,
    timestamp       DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE IF NOT EXISTS transactions
(
    id               INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT,
    transaction_type INT     NOT NULL
);

CREATE TABLE IF NOT EXISTS balance_changes
(
    id                     INTEGER  NOT NULL PRIMARY KEY AUTOINCREMENT,
    account_id             INT      NOT NULL,
    change_number          INT      NOT NULL,
    balance                TEXT     NOT NULL,
    net_balance_change     TEXT     NOT NULL,
    channel_hold_note_json  TEXT,
    notary_id              INT      NOT NULL,
    notes_json             TEXT     NOT NULL,
    status                 INT      NOT NULL,
    transaction_id         INT,
    proof_json             TEXT,
    finalized_block_number INT,
    notarization_id        INT,
    timestamp              DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (account_id) REFERENCES accounts (id),
    FOREIGN KEY (notarization_id) REFERENCES notarizations (id),
    FOREIGN KEY (transaction_id) REFERENCES transactions (id)
);

CREATE TABLE IF NOT EXISTS mainchain_transfers_in
(
    id                     INTEGER  NOT NULL PRIMARY KEY AUTOINCREMENT,
    address                TEXT     NOT NULL,
    amount                 TEXT     NOT NULL,
    transfer_id            INT      NOT NULL,
    expiration_tick       INT      NOT NULL,
    notary_id              INT      NOT NULL,
    balance_change_id      INT,
    first_block_hash       TEXT     NOT NULL,
    extrinsic_hash         TEXT     NOT NULL,
    finalized_block_number INT,
    created_at             DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (balance_change_id) REFERENCES balance_changes (id)
);

-- ChannelHolds pending claim by this localchain. The settled_amount and settled_signature fields are only updated in the json when the ChannelHold is settled.
CREATE TABLE IF NOT EXISTS open_channel_holds
(
    id                          TEXT     NOT NULL PRIMARY KEY, -- the hash of the initial balance change
    is_client                   BOOLEAN  NOT NULL,
    initial_balance_change_json TEXT     NOT NULL,
    balance_change_number       INT      NOT NULL,
    from_address                TEXT     NOT NULL,
    delegated_signer_address    TEXT     NULL,
    expiration_tick             INT      NOT NULL,
    settled_amount              TEXT     NOT NULL,             -- write the latest amount settled to this field. Only information that needs to be updated
    settled_signature           BLOB     NOT NULL,             -- store latest signature here. Only information that needs to be updated
    created_at                  DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    notarization_id             INT,
    missed_claim_window         BOOLEAN  NOT NULL DEFAULT 0,
    updated_at                  DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (notarization_id) REFERENCES notarizations (id)
);

CREATE TABLE IF NOT EXISTS domains
(
    id                    INTEGER  NOT NULL PRIMARY KEY AUTOINCREMENT,
    name                  TEXT     NOT NULL,
    top_level                   TEXT     NOT NULL,
    registered_to_address TEXT     NOT NULL,
    registered_at_tick    INT      NOT NULL,
    notarization_id       INT      NOT NULL,
    created_at            DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (notarization_id) REFERENCES notarizations (id)
);
