CREATE TABLE IF NOT EXISTS balance_tips
(
    key bytea PRIMARY KEY,
    value bytea NOT NULL,
    last_changed_notebook integer NOT NULL
);

CREATE TABLE IF NOT EXISTS balance_changes (
    notebook_number integer NOT NULL,
    changeset jsonb NOT NULL
);

CREATE INDEX IF NOT EXISTS balance_changes_notebook_number ON balance_changes (notebook_number);

CREATE TABLE  IF NOT EXISTS blocks (
    block_number integer PRIMARY KEY ,
    block_hash bytea NOT NULL,
    parent_hash bytea NOT NULL,
    received_time timestamptz NOT NULL
);

CREATE TABLE IF NOT EXISTS block_meta
(
    key integer PRIMARY KEY,
    finalized_block_number integer NOT NULL,
    finalized_block_hash bytea NOT NULL,
    best_block_number integer NOT NULL,
    best_block_hash bytea NOT NULL
);

CREATE TABLE IF NOT EXISTS chain_transfers
(
    to_localchain boolean NOT NULL,
    account_id bytea NOT NULL,
    account_nonce integer NULL,
    amount varchar NOT NULL,
    finalized_block integer NULL,
    included_in_notebook_number integer NULL
);

CREATE INDEX IF NOT EXISTS chain_transfers_included_in_notebook_number ON chain_transfers (included_in_notebook_number);

CREATE TABLE IF NOT EXISTS notebook_origins (
    account_id bytea NOT NULL,
    account_type INTEGER NOT NULL,
    uid INTEGER NOT NULL,
    notebook_number INTEGER NOT NULL,
    PRIMARY KEY (account_id, account_type)
);

CREATE UNIQUE INDEX IF NOT EXISTS notebook_origins_uid_notebook_number ON notebook_origins (uid, notebook_number);


CREATE TABLE IF NOT EXISTS notebooks (
    notebook_number INTEGER PRIMARY KEY NOT NULL,
    new_account_origins jsonb NOT NULL,
    change_merkle_leafs BYTEA[] NOT NULL
);

CREATE TABLE IF NOT EXISTS notebook_auditors (
    notebook_number INTEGER NOT NULL,
    public  bytea NOT NULL,
	rpc_urls varchar[] NOT NULL,
	signature bytea NULL,
	auditor_order integer not null,
	attempts integer NOT NULL default 0,
	last_attempt timestamptz NULL,
    PRIMARY KEY (notebook_number, public)
);

CREATE TABLE IF NOT EXISTS notebook_headers (
    notebook_number INTEGER PRIMARY KEY NOT NULL,
    version INTEGER NOT NULL,
    hash BYTEA,
    finalized_block_number INTEGER,
    pinned_to_block_number INTEGER,
    starting_best_block_number INTEGER NOT NULL,
    start_time timestamptz NOT NULL,
    end_time timestamptz NULL,
    notary_id INTEGER NOT NULL,
    tax varchar,
    chain_transfers jsonb NOT NULL,
    changed_accounts_root BYTEA NOT NULL,
    changed_account_origins jsonb NOT NULL
);

CREATE TABLE IF NOT EXISTS notebook_status (
    notebook_number INTEGER NOT NULL,
    chain_transfers INTEGER NOT NULL default 0,
    step INTEGER NOT NULL,
    open_time timestamptz NOT NULL,
    ready_for_close_time timestamptz NULL,
    closed_time timestamptz NULL,
    get_auditors_time timestamptz NULL,
    audited_time timestamptz NULL,
    submitted_time timestamptz NULL,
    finalized_time timestamptz NULL
);

CREATE UNIQUE INDEX idx_one_open_notebook
    ON notebook_status (step)
    WHERE step = 1;

-- Do not allow a notebook to be modified once it has been finalized
CREATE OR REPLACE FUNCTION check_notebook_finalized()
    RETURNS TRIGGER AS $$
BEGIN
    IF OLD.end_time IS NOT NULL THEN
        RAISE EXCEPTION 'This notebook header is finalized and immutable';
    END IF;
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER immutable_finalized_notebook
    BEFORE UPDATE ON notebook_headers
    FOR EACH ROW
EXECUTE PROCEDURE check_notebook_finalized();

CREATE TABLE  IF NOT EXISTS registered_keys (
    public bytea PRIMARY KEY,
    finalized_block_number integer NOT NULL
);

-- Create 5 sequences so that we can safely close a notebook without any overlap
-- The sequence in use at any given moment is notebook_number % 5

CREATE SEQUENCE IF NOT EXISTS uid_seq_0;
CREATE SEQUENCE IF NOT EXISTS uid_seq_1;
CREATE SEQUENCE IF NOT EXISTS uid_seq_2;
CREATE SEQUENCE IF NOT EXISTS uid_seq_3;
CREATE SEQUENCE IF NOT EXISTS uid_seq_4;

-- TODO: need to know the roles
-- GRANT USAGE ON SEQUENCE uid_seq_0 TO ulx;
-- GRANT USAGE ON SEQUENCE uid_seq_1 TO ulx;
-- GRANT USAGE ON SEQUENCE uid_seq_2 TO ulx;
-- GRANT USAGE ON SEQUENCE uid_seq_3 TO ulx;
-- GRANT USAGE ON SEQUENCE uid_seq_4 TO ulx;
