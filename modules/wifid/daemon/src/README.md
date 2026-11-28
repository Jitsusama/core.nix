# wifid's Source

One module for each part of what wifid does or reads: the services it
talks to (`supplicant.rs`, `nl80211.rs`, `rfkill.rs`, `network.rs`), what
it holds of the air and its cards (`air.rs`, `state.rs`, `card.rs`), how it
chooses and moves between access points (`choose.rs`, `measure.rs`,
`moves.rs`, `fading.rs`, `arrive.rs`, `transition.rs`, `warned.rs`), the
sign-in check (`check.rs`, `page.rs`), and what it serves the shell and the
launcher (`service.rs`, `face.rs`, `rows.rs`, `words.rs`). `main.rs` opens
by saying how they fit.

Many entries because each module is one part that `main.rs` names by
itself, and most read from several of these groups at once: the chooser
reads the air, the measures and the warnings, and the rows read all of
them. Folders by group would split each part from what it reads more often
than they'd gather it. A part that grows parts of its own takes a folder.
