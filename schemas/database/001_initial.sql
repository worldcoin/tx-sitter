CREATE TABLE upstreams (
	name TEXT PRIMARY KEY,
	url TEXT UNIQUE NOT NULL,
	chainid INTEGER NOT NULL
);

CREATE TABLE signing_keys (
	id INTEGER PRIMARY KEY
);

CREATE TABLE insecure_keys (
	id INTEGER PRIMARY KEY
	           REFERENCES signing_keys(id),
	key BLOB NOT NULL
);

CREATE TABLE tx_requests (
	id INTEGER PRIMARY KEY AUTOINCREMENT,
	signing_key INTEGER REFERENCES signing_keys(id),
	chainid INTEGER NOT NULL,
	to_addr BLOB NOT NULL,
	value INTEGER NOT NULL,
	calldata BLOB NOT NULL
);
