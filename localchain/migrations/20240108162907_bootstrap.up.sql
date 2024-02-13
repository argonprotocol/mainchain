CREATE TABLE IF NOT EXISTS accounts
(
    id                      INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT,
    address                 TEXT NOT NULL,
    account_id32            BLOB NOT NULL,
    account_type            INT NOT NULL,
    notary_id               INT NOT NULL,
    origin_notebook_number  INT,
    origin_uid              INT,
    created_at              DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at              DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE UNIQUE INDEX IF NOT EXISTS accounts_address_idx ON accounts(address, account_type, notary_id);

-- The notarizations stores the notarization json and the new account origins json if the notarization was done by this wallet.
-- If the notarization was done by another wallet, the notarization will be populated once it is found in a wallet for any pending balance changes.
CREATE TABLE IF NOT EXISTS notarizations
(
    id                          INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT,
    json                        TEXT NOT NULL,
    notary_id                   INT NOT NULL,
    notebook_number             INT,
    tick                        INT,
    timestamp                   DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE IF NOT EXISTS balance_changes
(
    id                      INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT,
    account_id              INT NOT NULL,
    change_number           INT NOT NULL,
    balance                 TEXT NOT NULL,
    escrow_hold_note_json  TEXT,
    notary_id               INT NOT NULL,
    notes_json              TEXT NOT NULL,
    status                  INT NOT NULL,
    proof_json              TEXT,
    finalized_block_number  INT,
    notarization_id         INT,
    timestamp               DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (account_id) REFERENCES accounts(id),
    FOREIGN KEY (notarization_id) REFERENCES notarizations(id)
);

-- Escrows pending claim by this localchain. The settled_amount and settled_signature fields are only updated in the json when the escrow is settled.
CREATE TABLE IF NOT EXISTS open_escrows
(
    id                              TEXT NOT NULL PRIMARY KEY, -- the hash of the initial balance change
    is_client                       BOOLEAN NOT NULL,
    initial_balance_change_json     TEXT NOT NULL,
    balance_change_number           INT NOT NULL,
    from_address                    TEXT NOT NULL,
    expiration_tick                 INT NOT NULL,
    settled_amount                  TEXT NOT NULL, -- write the latest amount settled to this field. Only information that needs to be updated
    settled_signature               BLOB NOT NULL, -- store latest signature here. Only information that needs to be updated
    created_at                      DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    notarization_id                 INT,
    missed_claim_window             BOOLEAN NOT NULL DEFAULT 0,
    updated_at                      DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (notarization_id)   REFERENCES notarizations(id)
);

CREATE TABLE IF NOT EXISTS data_domains
(
    id                  INTEGER NOT NULL                                             PRIMARY KEY AUTOINCREMENT,
    name                TEXT NOT NULL,
    tld                 INT NOT NULL,
    registered_to_address TEXT NOT NULL,
    registered_at_tick  INT NOT NULL,
    notarization_id     INT NOT NULL,
    created_at          DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (notarization_id) REFERENCES notarizations(id)
);
