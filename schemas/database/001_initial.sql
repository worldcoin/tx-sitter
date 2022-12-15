CREATE TABLE upstreams (
	name TEXT PRIMARY KEY,
	url TEXT UNIQUE NOT NULL,
	chainid INTEGER NOT NULL
);

CREATE TABLE signing_keys (
	id INTEGER PRIMARY KEY,
	name BLOB,
	address BLOB
	  -- note that this length check (and all others in this file) checks the BLOB
	  -- byte length. Sqlite does not perform strict type checking so it's possible to
	  -- insert a string into a BLOB column. `length()` acts differently on strings and
	  -- will return false-positive & false-negatives if that happens. If this becomes a
	  -- problem these checks can be turned into functions which also check the type
	  CHECK(length(address) == 20),
	CHECK(name IS NOT NULL or address IS NOT NULL)
);

CREATE TABLE insecure_keys (
	id INTEGER PRIMARY KEY
	           REFERENCES signing_keys(id),
	key BLOB NOT NULL
	  CHECK(length(key) == 32)
);

CREATE TABLE kms_keys (
	id INTEGER PRIMARY KEY
	           REFERENCES signing_keys(id),
	key_id BLOB NOT NULL
);

-- if this table becomes much more complicated with additional variants it should probably
-- be split into a polymorphic table a la `signing_keys`
CREATE TABLE tx_requests (
	id INTEGER PRIMARY KEY AUTOINCREMENT,
	idempotency_key BLOB UNIQUE,  -- nb. NULL != NULL

	chainid INTEGER NOT NULL,
	signing_key INTEGER REFERENCES signing_keys(id),

	-- blob because SQLite has no other type which can handle 256 bit integers,
	-- this is encoded in big-endian
	value BLOB NOT NULL
	  CHECK(length(value) == 32),

	-- TEXT because sqlite does not support enums
	-- [tag:check_variant]
	variant TEXT NOT NULL
	  CHECK(variant == "call" or variant == "deploy"),

	receiver BLOB  -- NULL for Deploy
	  CHECK(length(receiver) == 20 or receiver IS NULL),

	-- for Call this becomes "calldata", for Deploy this becomes "initcode"
	data BLOB NOT NULL,

	gas_limit BLOB
	  CHECK(length(gas_limit) == 32)
);

-- an ethereum transaction which we have sent to an upstream
-- there may be multiple of these per tx_request; we may have to perform gas escalation
CREATE TABLE submitted_eth_tx (
	id INTEGER PRIMARY KEY AUTOINCREMENT,
	tx_request INTEGER NOT NULL
	           REFERENCES tx_requests(id),
	tx_hash BLOB NOT NULL
	  CHECK(length(tx_hash) == 32),
	eth_tx BLOB NOT NULL,  -- the rlp-encoded transaction, including signature

	submitted_at INTEGER NOT NULL, -- unix timestamp, seconds

	-- below are receipt fields, null until the transaction is mined
	-- note that post-merge reorgs are still possible (though unlikely),
	-- so these fields may change after they are set

	block_number INTEGER,
	block_hash BLOB
	  CHECK(length(block_hash) == 32),
	deployed_address BLOB  -- only populated if this tx deployed a contract
	  CHECK(length(deployed_address) == 20 or deployed_address IS NULL),
	failed BOOL,  -- maps to the `status` field (1 for success, 0 for failure),
	              -- there are occasional EIPs which attempt to add more codes so we
		      -- might have to migrate some day, but they seem unlikely to ever
		      -- make it through, and this bool is much easier to understand than
	 	      -- an int which inverts POSIX convention
	gas_used BLOB
	  CHECK(length(gas_used) == 32),
	effective_gas_price BLOB
	  CHECK(length(effective_gas_price) == 32)

	-- eventually we might want logs but the tx_hash is enough for clients to look
	-- those up themselves
);

