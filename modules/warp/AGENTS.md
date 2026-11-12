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

## Serving the Shell

Each read asks warp-cli five questions at once. Run by hand, `warp`
keeps nothing, since WARP's service holds every figure and answers in
about 20 ms. The shell spawns nothing to read or switch it (Joel,
2026-11-10: "CLI execs is not performant"), so `warp serve` holds the
link as one record, `warp:link`, over `$XDG_RUNTIME_DIR/daemons/warp.sock`
in the shell's daemon interface ([`src/serve.rs`][serve]). It reads only
while the link is watched, every two seconds, and at once after a switch
asked as an `act`, so nothing asks warp-cli while WARP isn't shown. A
watch is answered with what's held only while it's that fresh, so an old
link is never shown as now. While it's watched, the daemon also holds
`warp-cli --json --listen status` open, which writes a line of JSON on
every change of state (`contract/listen-status.jsonl`). A line telling a
status or reason other than the last has the link read at once, so a
link that drops or degrades is said as it does; a reason's figures
moving are left to the two-second reads, and a DNS log line sets nothing
off. A stream that ends by itself, as the shell's stand-in warp-cli's
refusal does, is asked for again only five seconds on, and the
two-second reads carry on meanwhile.

How WARP stands is a second kind, `warp-state`, with one record,
`warp:state`: the status and reason the listening warp-cli tells, as
words, or `{ error }` with warp-cli's code, and `since_ms` only where
the daemon saw that state begin. The margin watches it always
(`WarpState.qml`), to say when the work network ails, so a state watch
holds only the listener and sets off none of the link's reads. Where
nothing listens, as while WARP's service is down, the status is asked
once instead each time, so a link coming back is said within seconds.

## The Switch

`src/switch.rs` decides what a switch says before it asks warp-cli: a
switch the organisation holds (`switch_locked`) is left as it is and
says who holds it, ending 6; WARP already as asked is said, and nothing
asked; and WARP always on says, once disconnected, that it comes back by
itself after `auto_connect_after_min`. warp-cli refusing ends 7, in its
words. warp-cli decides the rest, as it does for anyone, so nothing here
needs root.

[contract]: contract/README.md
[serve]: src/serve.rs
[record]: warp.json
[golden]: src/golden/
