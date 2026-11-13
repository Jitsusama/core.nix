# Golden Pages

Each view's page as `golden.rs` draws it from `home-network.json`, a
minute after the record was read, in a zone five hours behind UTC: the
record as it is, and, in the `-troubled` pages, with something wrong in
each place a view marks it. The pages here are set into a pipe, whole;
each folder named for a width holds the pages a terminal that wide sets
otherwise, and a page missing from one is set there as into a pipe. A
test fails on any line that differs, on a line past its width, and on a
page left in a width's folder that no longer differs from the pipe's.
`EERO_GOLDEN=write cargo test` writes them afresh, and takes away those
that went; read what changed before committing it.

- `60/`: each page in a terminal 60 columns wide, beside an editor.
- `80/`: each page in a terminal 80 columns wide, the default.
- `100/`: each page in a terminal 100 columns wide.
- `140/`: each page in a terminal 140 columns wide, a whole screen.
