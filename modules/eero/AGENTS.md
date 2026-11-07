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
  test and every one eero kept, firmware, outages and the day's traffic.
- `nodes[]`: each eero by `id`, its `backhaul` (Ethernet, or the
  `upstream` node's location and radio over Wi-Fi), each radio's band,
  channel, width, power, how busy and its devices, each port's speed,
  and the day's traffic.
- `devices[]`: each device on now, by `mac`, its `node`, band, channel,
  signal, SNR, each direction's rate with its MCS, streams, width, guard
  interval and Wi-Fi generation, retries, packets, and its speed now.
- `profiles[]`, `events[]`: who's in which profile, and the app's
  activity list.

## How It Reads

`eero daemon` (the `eero-daemon` user unit) keeps the model, drawn by
[`src/read.rs`][read] from each read's answer and written whole, and
only when it changes. It reads the account's first network, each part
as often as its figures move ([`src/schedule.rs`][schedule]):

| Read                                           | Requests        | Every                        |
| ---------------------------------------------- | --------------- | ---------------------------- |
| the network and its devices                    | 2               | 30 s listened to, else 5 min |
| each node's connections                        | 1 a node        | 5 min                        |
| the day's usage, the network's and each node's | 1, and 1 a node | 15 min                       |
| speed tests, profiles, Thread, outages, events | 5               | 15 min                       |

Its socket, `$XDG_RUNTIME_DIR/eero/socket`, writes whoever connects the
model as one line of JSON, and another on every change. While anyone
is connected the network is read at the fast pace, and a line `read`
asks for it now, at most once every 5 seconds.

The model's `cloud.state` says whether the figures are current:

- `answering`: eero answered the last read.
- `unreachable`: eero couldn't be reached or failed; the figures are the
  last it gave, `since` says from when and `reason` why. Once it answers
  again, every part is read at once.
- `signed-out`: the session aged past refreshing; `reason` says to run
  `eero sign-in`, and the next read opens the newly sealed session.

[contract]: contract/README.md
[model]: home-network.json
[read]: src/read.rs
[schedule]: src/schedule.rs
