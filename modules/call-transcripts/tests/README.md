# Call Transcripts' Tests

Two, one for each half of the module that reaches beyond its crate:

- `source.rs` runs transcripts-people as the people daemon runs it, over a
  stand-in calls.sock, as Cargo runs any test beside a crate's sources.
- `transcripts.qml` runs the shell's `Transcripts.js` in Qt's own engine,
  as the `the-desktop-reads-the-transcripts` check does.
