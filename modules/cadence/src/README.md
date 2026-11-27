# Cadence's Source

One module for each thing cadence watches or does: the hour's card and its
measures (`hour.rs`), the evening's note (`day.rs`), the ledger of findings
(`findings.rs`, `owe.rs`), the mesh it reads and writes (`mesh.rs`), the
units and idle sessions it watches (`units.rs`, `idle.rs`), and the axes Joel
reads (`axes.rs`). `main.rs` says what each command does and which module
holds it.

Many entries because a module with parts of its own is both a file and a
folder of one name, so the nine that have grown parts take eighteen entries,
and each of the rest is one command or one reader that `main.rs` calls on its
own: grouping them by kind would put a command's file away from the folder
holding its parts. A part that grows parts of its own takes a folder.
