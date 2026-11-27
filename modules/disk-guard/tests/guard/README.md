# The Disk Guard's Scenarios

The guard's tests against the built program, one binary so they build once,
as [`main.rs`][1] lists them.

Many entries because each file holds the scenarios of the source module of
the same name, so a change to `src/caches.rs` is proven in `caches.rs` here.
The survey's two parts are proven through it: `src/survey/builds.rs` in
`survey_trees.rs` and `replaced_survey.rs`, and `src/survey/sources.rs` in
`survey.rs`, `quest_temp.rs` and `store_roots.rs`.
The few with no module of their name cover what several do together
(`collecting.rs`, `entries.rs`, `procs.rs`, `telling.rs`, `variants.rs`)
or hold what the others share (`sandbox.rs`). A module's file that would pass 800 lines
splits off a part named for what it holds, as `garbage_delete.rs` holds
`garbage`'s deletes, `pass_lines.rs` what a pass says and `pass_skips.rs`
what a cut reach skips.

[1]: main.rs
