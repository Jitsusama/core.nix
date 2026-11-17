# The Programs Daemon's Source

Two halves share this folder. The library (`lib.rs`) holds what makes rows
and acts: the rows of each place (`tab_rows.rs`, `code_rows.rs`,
`taken_rows.rs`), what is read for them (`source.rs`, `kitty_tabs.rs`,
`repositories.rs`) and the acts on them (`pull.rs`, `switch.rs`). The daemon
(`main.rs`) holds what serves them: its wiring, when and how a source is
read (`runs.rs`, `read.rs`) and each place's rows as it serves them
(`tabs.rs`, `taken.rs`, `code.rs`). How they fit is in
[the module's README][1].

Many entries because each place's two halves sit side by side, `tab_rows.rs`
beside `tabs.rs` and `taken_rows.rs` beside `taken.rs`: grouping them into
folders by half would split each place across two, and a change to one place
often touches both. A place that grows parts of its own takes a
folder.

[1]: ../README.md
