# The Merge Net's Source

One module for each part of what the net does: the commands it answers
(`command.rs`, dispatched by `main.rs`), whether a head may merge
(`ready`, `needs.rs`, `earlier`, `turn.rs`), the merge train that lands
heads together (`train.rs`, `aboard.rs`, `boarding.rs`), and the watch
that runs main's VM tests after each merge and reverts a merge that breaks
one (`watch.rs`, `candidates.rs`, `quarantine.rs`). `main.rs` names each in
a line.

Many entries because Rust keeps a module's file beside the folder of its
parts, as `train.rs` sits beside `train/`, and each module is one step that
`main.rs` or the train reaches by name: grouping them into folders by kind
would add a layer to every path without making any step easier to find,
since `ready`, the train and the watch share most of them. A part that
grows parts of its own takes a folder.
