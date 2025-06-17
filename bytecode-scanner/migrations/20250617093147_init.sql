create table bytecode (
    hash BLOB NOT NULL PRIMARY KEY,
    bytecode BLOB NOT NULL,
    call_counter INTEGER NOT NULL DEFAULT 1
);
