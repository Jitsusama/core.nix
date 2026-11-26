# The Agents Daemon's Source

One module for each thing the daemon reads, works out or serves: a source
it follows (`file.rs`, `sources.rs`), how a session stands (`standing.rs`,
`session.rs`), a kind of record it serves (`row.rs`, `top.rs`,
`networks.rs`, `cells.rs`), and the acts it asks of the node (`acts.rs`,
`sprout.rs`, `places.rs`, `offers.rs`). `lib.rs` names each in a line.

Many entries because each module is one piece of what the launcher is
served, and `inputs.rs` joins them all into the records it serves: grouping
them into folders by kind would split a record from the readers it is made
of, since nearly every record reads several. A part that grows parts of its
own takes a folder.
