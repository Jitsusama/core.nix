# The Disk Guard's Source

One module for each thing the guard judges or reads: a kind of source it
reclaims (`caches.rs`, `nix_builds.rs`, `trees.rs`), a rule it holds
(`band.rs`, `age.rs`, `holders.rs`), a platform it runs on (`linux.rs`,
`macos.rs`) and the daemon and command line that drive them. How they fit is
in [the guide][1].

Many entries because each module answers one question the guard asks of a
path, and a step names the one it needs: grouping them into folders by kind
would split the rules from the sources they judge, and nearly every module
reads several of each. A part that grows parts of its own takes a folder.

[1]: ../AGENTS.md
