# The Disk Guard's Scenarios

The guard's tests against the built program, one binary so they build once,
as [`main.rs`][1] lists them.

Many entries because each file holds the scenarios of the source module of
the same name, so a change to `src/caches.rs` is proven in `caches.rs` here.
The few with no module of their name cover what several do together
(`collecting.rs`, `entries.rs`, `procs.rs`, `variants.rs`) or hold what
the others share (`sandbox.rs`).

[1]: main.rs
