# Golden Pages

Each view's page as `golden.rs` draws it from `home-network.json`, a
minute after the record was read, in a zone five hours behind UTC: the
record as it is, and, in the `-troubled` pages, with something wrong in
each place a view marks it. A test fails on any line that differs.
`EERO_GOLDEN=write cargo test` writes them afresh; read what changed
before committing it.
