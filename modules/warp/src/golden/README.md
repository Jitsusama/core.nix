# Golden Pages

The page `warp` prints, as `page.rs` draws it from `../../warp.json`: the
record as it is, in `connected.txt`; its status degraded, as
`../../contract/status.degraded.json` says it, in `degraded.txt`; and
disconnected with nothing else given, in `disconnected.txt`. A test fails
on any line that differs. `WARP_GOLDEN=write cargo test` writes them
afresh; read what changed before committing it.
