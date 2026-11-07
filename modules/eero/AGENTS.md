# eero

The home network's eero, read from eero's cloud, since the eero has no
local API. A Rust program, `eero`, signs in once (`src/session.rs` seals
the session), reads, and changes one setting, Optimize for Conferencing
and Gaming. Joel's bar for it (2026-11-07): "i want insanely nerdy
integration with the eero": every figure eero knows that is true and
useful, shown where he looks, never a summary that hides them.

## What It Reads

[`contract/`][contract] holds every answer the eero app's reads give,
from a real three-node network, redacted, and its README says what each
figure means and where it shows, or why it doesn't. A figure is shown
only once the contract has it and the README places it.

## The Model

[`home-network.json`][model] is the home network as one record: its
internet, its nodes with their backhaul, radios and ports, its devices
with their band, signal, rates and speed, its profiles and what
happened. It's drawn from the contract's files, so it's the parsers'
golden answer, and it's the shape the shell and the CLI read from
`$XDG_RUNTIME_DIR/eero/home-network.json`. Its words are the
glossary's Network words; a figure keeps its unit in its key
(`signal_dbm`, `busy_pct`, `up_s`), and one eero doesn't give is `null`,
never a guess.

- `read`, `cloud`: when eero last answered, and whether it's answering,
  unreachable or signed out.
- `internet`: the home network's way out, its settings, the last speed
  test and every one eero kept, firmware, outages and the day's usage.
- `nodes[]`: each eero by `id`, its `backhaul` (Ethernet, or the
  `upstream` node's location and radio over Wi-Fi), each radio's band,
  channel, width, power, how busy and its devices, each port's speed,
  and the day's usage.
- `devices[]`: each device on now, by `mac`, its `node`, band, channel,
  signal, SNR, each direction's rate with its MCS, streams, width, guard
  interval and Wi-Fi generation, retries, packets, and its speed now.
- `profiles[]`, `events[]`: who's in which profile, and the app's
  activity list.

[contract]: contract/README.md
[model]: home-network.json
