CREATE TABLE upstreams (
	name TEXT PRIMARY KEY,
	url TEXT UNIQUE NOT NULL,
	chainid INTEGER NOT NULL
);

CREATE TABLE signing_keys (
	id INTEGER PRIMARY KEY,
	name BLOB,
	address BLOB,
	CHECK(name IS NOT NULL or address IS NOT NULL)
);

CREATE TABLE insecure_keys (
	id INTEGER PRIMARY KEY
	           REFERENCES signing_keys(id),
	key BLOB NOT NULL
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
	idempotency_key BLOB UNIQUE,

	chainid INTEGER NOT NULL,
	signing_key INTEGER REFERENCES signing_keys(id),

	value INTEGER NOT NULL,

	-- TEXT because sqlite does not support enums,
	variant TEXT NOT NULL
	  CHECK(variant == "Call" or variant == "Deploy"),

	receiver BLOB,  -- NULL for Deploy

	-- for Call this becomes "calldata", for Deploy this becomes "initcode"
	data BLOB NOT NULL,

	-- blob because SQLite has no other type which can handle 256 bit integers,
	-- this is encoded in big-endian
	gas_limit BLOB,
);
