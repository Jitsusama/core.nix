# The Proof's Parts

What `shaper proof` (`../proof.rs`) splits off from the run itself:

- `gate.rs`: when it may load the link: Joel away, no call or audio, and
  nothing the machine names busy.
- `load.rs`: the agents' uploads it loads the link with, and how they end:
  at its word, or on their own once it's gone.
- `probes.rs`: Joel's probes in each block, and how each ends: on its
  own, at a stop, or once it has run well past its block.
- `stop.rs`: how it stops and puts the shaper back: a signal heard as a
  stop, progress said safely, the shaper's state kept on disk while a
  window owns it, and a keeper that puts it back if the proof is killed.
- `verdict.rs`: what each block measured, which pairs are void, and the
  verdict on the shaper over the rest.
