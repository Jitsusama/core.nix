# nix-offload's Source

One module for each thing nix-offload decides, reads or runs: when a
builder is lent and given back (`decide.rs`), what Nix and the machine
queue read of it (`machines.rs`), whether it still builds (`proof.rs`,
`standing.rs`), the machines their owners hold (`holds.rs`), and each
command it answers (`there.rs`, `command.rs`, `relay.rs`, `lines.rs`). What
it keeps is apart too: what passed at a commit (`passed.rs`) and what the
builder did (`journal.rs`). A part with parts of its own has a folder
beside its module, as `gate/`, `keep/` and `changed/` do.

Many entries because each module answers one question a run or the lender
asks, and nearly every step reads several of them: grouping them by kind
would split the commands from the builders' standing they read together.
