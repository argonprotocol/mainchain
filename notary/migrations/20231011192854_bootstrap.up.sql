CREATE TABLE IF NOT EXISTS balance_tips
(
    key                   bytea PRIMARY KEY,
    value                 bytea   NOT NULL,
    last_changed_notebook integer NOT NULL,
    last_changed_tick     integer NOT NULL
);


CREATE TABLE IF NOT EXISTS block_sync_lock
(
    key integer PRIMARY KEY
);

INSERT INTO block_sync_lock (key)
VALUES (1);

CREATE TABLE IF NOT EXISTS blocks
(
    block_hash             bytea       NOT NULL PRIMARY KEY,
    parent_hash            bytea       NOT NULL,
    block_number           integer     NOT NULL,
    block_vote_minimum     varchar     NOT NULL,
    latest_notebook_number integer,
    notebook_digests       jsonb       NULL,
    is_finalized           boolean     NOT NULL,
    finalized_time         timestamptz,
    received_time          timestamptz NOT NULL
);

CREATE TABLE IF NOT EXISTS mainchain_identity (
    chain varchar NOT NULL PRIMARY KEY,
    genesis_hash    bytea NOT NULL,
    created_at timestamptz NOT NULL
);

CREATE TABLE IF NOT EXISTS notebook_headers
(
    notebook_number         integer PRIMARY KEY NOT NULL,
    version                 integer             NOT NULL,
    hash                    bytea,
    signature               bytea,
    tick                    integer             NOT NULL,
    notary_id               integer             NOT NULL,
    tax                     varchar,
    chain_transfers         jsonb               NOT NULL,
    changed_accounts_root   bytea               NOT NULL,
    changed_account_origins jsonb               NOT NULL,
    block_votes_root        bytea               NOT NULL,
    block_votes_count       integer             NOT NULL,
    block_voting_power      varchar             NOT NULL,
    blocks_with_votes       bytea[]             NOT NULL,
    secret_hash             bytea               NOT NULL,
    parent_secret           bytea               NULL,
    domains                 jsonb               NOT NULL,
    last_updated            timestamptz         NOT NULL default now()
);

CREATE TABLE IF NOT EXISTS chain_transfers
(
    to_localchain               boolean NOT NULL,
    account_id                  bytea   NOT NULL,
    transfer_id                 integer NULL,
    amount                      varchar NOT NULL,
    finalized_block_number      integer NULL,
    expiration_tick             integer NULL,
    included_in_notebook_number integer NULL REFERENCES notebook_headers (notebook_number)
);

CREATE INDEX IF NOT EXISTS chain_transfers_included_in_notebook_number ON chain_transfers (included_in_notebook_number);


CREATE TABLE IF NOT EXISTS registered_keys
(
    public                 bytea PRIMARY KEY,
    effective_tick         integer NOT NULL
);
CREATE TABLE IF NOT EXISTS notarizations
(
    notebook_number integer NOT NULL REFERENCES notebook_headers (notebook_number),
    balance_changes jsonb   NOT NULL,
    block_votes     jsonb   NOT NULL,
    domains    jsonb   NOT NULL,
    account_lookups bytea[] NOT NULL -- combined key of <account_id::hex>_<account_type>_<change_number>
);

CREATE INDEX IF NOT EXISTS notarizations_account_lookup ON notarizations (notebook_number, account_lookups);
CREATE INDEX IF NOT EXISTS notarizations_notebook_number ON notarizations (notebook_number);


CREATE TABLE IF NOT EXISTS notebook_origins
(
    account_id      bytea   NOT NULL,
    account_type    integer NOT NULL,
    uid             integer NOT NULL,
    notebook_number integer NOT NULL REFERENCES notebook_headers (notebook_number),
    PRIMARY KEY (account_id, account_type)
);

CREATE UNIQUE INDEX IF NOT EXISTS notebook_origins_uid_notebook_number ON notebook_origins (uid, notebook_number);

CREATE TABLE IF NOT EXISTS notebooks
(
    notebook_number     integer     NOT NULL PRIMARY KEY REFERENCES notebook_headers (notebook_number),
    new_account_origins jsonb       NOT NULL,
    change_merkle_leafs bytea[]     NOT NULL, -- pre-encoded to save time for merkle proofs
    block_votes         jsonb       NOT NULL,
    hash                bytea       NOT NULL,
    signature           bytea       NOT NULL,
    last_updated        timestamptz NOT NULL default now()
);
CREATE INDEX IF NOT EXISTS notebook_new_account_origins ON notebooks USING GIN (new_account_origins);


CREATE TABLE IF NOT EXISTS notebooks_raw
(
    notebook_number integer NOT NULL PRIMARY KEY REFERENCES notebook_headers (notebook_number),
    encoded         bytea   NOT NULL
);

CREATE TABLE IF NOT EXISTS notebook_secrets
(
    notebook_number integer NOT NULL PRIMARY KEY REFERENCES notebook_headers (notebook_number),
    secret          bytea   NOT NULL
);

CREATE TABLE IF NOT EXISTS notebook_constraints
(
    notebook_number integer NOT NULL PRIMARY KEY REFERENCES notebook_headers (notebook_number),
    chain_transfers integer NOT NULL default 0,
    block_votes     integer NOT NULL default 0,
    balance_changes integer NOT NULL default 0,
    notarizations   integer NOT NULL default 0,
    domains    integer NOT NULL default 0
);


CREATE TABLE IF NOT EXISTS notebook_status
(
    notebook_number      integer     NOT NULL PRIMARY KEY REFERENCES notebook_headers (notebook_number),
    tick                 integer     NOT NULL,
    step                 integer     NOT NULL,
    open_time            timestamptz NOT NULL,
    end_time             timestamptz NOT NULL,
    ready_for_close_time timestamptz NULL,
    closed_time          timestamptz NULL,
    finalized_time       timestamptz NULL
);

CREATE UNIQUE INDEX idx_one_open_notebook
    ON notebook_status (step)
    WHERE step = 1;

-- create a notify channel for each notebook


-- Do not allow a notebook to be modified once it has been finalized
CREATE OR REPLACE FUNCTION check_notebook_finalized()
    RETURNS TRIGGER AS
$$
BEGIN
    IF OLD.hash IS NOT NULL THEN
        RAISE EXCEPTION 'This notebook header is finalized and immutable';
    END IF;
    IF NEW.hash IS NOT NULL THEN
        PERFORM pg_notify('notebook_finalized', NEW.notebook_number::text);
    END IF;
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;
CREATE OR REPLACE FUNCTION update_last_modified()
    RETURNS TRIGGER AS
$$
BEGIN
    NEW.last_updated := NOW();
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER immutable_finalized_notebook
    BEFORE UPDATE
    ON notebook_headers
    FOR EACH ROW
EXECUTE PROCEDURE check_notebook_finalized();

CREATE TRIGGER update_header_last_modified
    BEFORE UPDATE
    ON notebook_headers
    FOR EACH ROW
EXECUTE PROCEDURE update_last_modified();

CREATE TRIGGER update_notebook_last_modified
    BEFORE UPDATE
    ON notebooks
    FOR EACH ROW
EXECUTE PROCEDURE update_last_modified();


-- Create 5 sequences so that we can safely close a notebook without any overlap
-- The sequence in use at any given moment is notebook_number % 5

CREATE SEQUENCE IF NOT EXISTS uid_seq_0;
CREATE SEQUENCE IF NOT EXISTS uid_seq_1;
CREATE SEQUENCE IF NOT EXISTS uid_seq_2;
CREATE SEQUENCE IF NOT EXISTS uid_seq_3;
CREATE SEQUENCE IF NOT EXISTS uid_seq_4;

-- TODO: need to know the roles
-- GRANT USAGE ON SEQUENCE uid_seq_0 TO argon;
-- GRANT USAGE ON SEQUENCE uid_seq_1 TO argon;
-- GRANT USAGE ON SEQUENCE uid_seq_2 TO argon;
-- GRANT USAGE ON SEQUENCE uid_seq_3 TO argon;
-- GRANT USAGE ON SEQUENCE uid_seq_4 TO argon;
