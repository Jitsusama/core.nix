# The Disk Guard's Scenarios

The guard's tests against the built program, one binary so they build once,
as [`main.rs`][1] lists them. Cargo runs them as the guard builds, in
`disk-guard-runs-its-scenarios`.

Many entries because each file holds the scenarios of the source module of
the same name, so a change to `src/caches.rs` is proven in `caches.rs` here.
The few with no module of their name cover what several do together
(`collecting.rs`, `entries.rs`, `procs.rs`, `telling.rs`, `variants.rs`)
or hold what the others share (`sandbox.rs`). A module's file that would pass 800 lines
splits off a part named for what it holds, as `garbage_delete.rs` holds
`garbage`'s deletes, `pass_lines.rs` what a pass says and `pass_skips.rs`
what a cut reach skips.

[1]: main.rs
