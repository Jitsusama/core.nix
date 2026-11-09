# WARP

WARP, the VPN, read from warp-cli, WARP's own client, which asks
WARP's service over its socket. A Rust program, `warp`, reads it, and
changes only its switch, `warp connect` and `warp disconnect`, as the
launcher's `:connect` and `:disconnect` run them. Joel's bar for every
integration (2026-11-07): "every integration has an insanely high bar
of nerdiness and integration and each should be given the same care",
so every figure warp-cli gives is shown where he looks, or named as
left out with its reason.

## What It Reads

[`contract/`][contract] holds each answer warp-cli gives that `warp` reads,
in its real shape with made-up values, and its README says where each
figure shows or why it doesn't. A figure is shown only once the contract
has it and the README places it.

## The Record

[`warp.json`][record] is WARP as one record, drawn from the contract, so
it's both the Rust tests' golden answer and the input the shell's own
cases read (`tests/answers.qml`). A figure keeps its unit in its key
(`latency_ms`, `loss_fraction`, `last_handshake_s`), and one warp-cli
doesn't give is `null`, never a guess. `warp --json` prints it; `warp`
prints the page `src/page.rs` draws, held to golden pages
([`src/golden/`][golden]), written afresh with `WARP_GOLDEN=write cargo
test`.

## No Daemon

Each run asks warp-cli five questions at once and keeps nothing: WARP's
service is already the daemon, holding every figure and answering in
about 20 ms, so a copy kept here would only be older. The shell runs
`warp --json` while WARP is shown, every two seconds, and never otherwise
(`../quickshell/launcher/Warp.qml`). `warp-cli --listen status` writes a
line of JSON on every change of state, which a piece in the margin can
listen to when one is wanted, rather than anything polling.

## The Switch

`src/switch.rs` decides what a switch says before it asks warp-cli: a
switch the organisation holds (`switch_locked`) is left as it is and
says who holds it, ending 6; WARP already as asked is said, and nothing
asked; and WARP always on says, once disconnected, that it comes back by
itself after `auto_connect_after_min`. warp-cli refusing ends 7, in its
words. warp-cli decides the rest, as it does for anyone, so nothing here
needs root.

[contract]: contract/README.md
[record]: warp.json
[golden]: src/golden/
