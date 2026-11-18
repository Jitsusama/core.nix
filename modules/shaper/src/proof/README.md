# The Proof's Parts

What `shaper proof` (`../proof.rs`) splits off from the run itself:

- `gate.rs`: when it may load the link: Joel away, no call or audio, and
  nothing the machine names busy.
- `keeper.rs`: the scope beside a proof that puts the shaper back once
  the proof is gone, however it went.
- `load.rs`: the agents' uploads it loads the link with, and how they end:
  at its word, or on their own once it's gone.
- `lock.rs`: one proof at a time, by a lock held as long as it runs.
- `probes.rs`: Joel's probes in each block, and how each ends: on its
  own, at a stop, or once it has run well past its block.
- `stop.rs`: how it stops and puts the shaper back: a signal heard as a
  stop, progress said safely, the shaper's state kept on disk while a
  window owns it, and the shaper's state read once it has settled.
- `verdict.rs`: what each block measured, which pairs are void, and the
  verdict on the shaper over the rest.
- `windows.rs`: how the pairs spread across Joel's idle windows, each
  reading the shaper afresh, and how the proof ends before its verdict
  counts: a shaper left wrong, a stop, or a state systemd didn't say.
