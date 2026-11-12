# The Sound Daemon

The sound server's outputs and inputs, held ready for the launcher over
`$XDG_RUNTIME_DIR/daemons/sound.sock`, in the shell's daemon interface
(version 1, as the windows and machine daemons speak it). The shell
imports it, so a machine with the shell runs it.

It hears PipeWire through its own API for as long as it runs: the
registry says each node and link as it comes and goes, each output's,
input's and program stream's node says its properties and its Props as
they change, and the default
metadata says which output and input are chosen and where each
program's stream is sent. A watch is told only
the records that changed and the keys gone, so a volume moving tells its
one device. Before it, the launcher's Sound read Quickshell's PipeWire
objects on its main thread, kept `pw-top` and `pw-metadata` running for
levels and choices, and read whole files rewritten by another daemon.

## Every Figure, and Where It Shows

| Kind                    | Figures                                         | Where                   |
| ----------------------- | ----------------------------------------------- | ----------------------- |
| `device:<node name>`    | `sink`: whether it's an output                  | Outputs or Inputs       |
|                         | its node's id, name, description and nick       | the row's key and name  |
|                         | `volume`: as the shell shows it, and `volumes`  | the row's volume        |
|                         | `muted`                                         | the row's mute          |
|                         | `chosen`: whether the default metadata names it | Use, and the row's mark |
|                         | `properties`: every property of its node        | its drawer's facts      |
|                         | `form`: what its card says it is, as BlueZ does | the row's glyph         |
|                         | `unplugged`: whether nothing is in its jack     | whether it's listed     |
|                         | `ports`: its card's ports for it, each's index, | Port                    |
|                         | name and `available`, and the one `chosen`      |                         |
|                         | `channels`: which channel each volume is of     | Channels                |
|                         | `facts`: what desk-facts says of its card, by   | its last parts, and its |
|                         | the `key` it's found by                         | drawer                  |
| `stream:<id>`           | a program's stream, by its node's PipeWire id:  | each device's programs  |
|                         | its name, `sink`, `volume`, `volumes`, `muted`  | and each program's row  |
|                         | and `properties`, as a device's are             |                         |
|                         | `ends`: the ids of the nodes it's linked to     | which device it plays   |
|                         | `target`: where it's sent, or "" by default     | Plays On, Records From  |
| `row:sound:<node name>` | the launcher's row for it, ready to draw        | Sound's place           |
| `targets:default`       | `targets`: each stream's `target.object`, by    | Plays On, Records From  |
|                         | the stream's id, where one is sent              |                         |
| `timing:<id>`           | a running node's cycle, in its driver's         | a drawer's Dropouts,    |
|                         | `quantum` and `rate`, and `ms`; `busy`, the     | Load and Cycle          |
|                         | share of it the node was busy, or `unfinished`; |                         |
|                         | `errors`, its dropouts; and whether it `drives` |                         |

A volume is the mean of each channel's cube root, as Quickshell says it,
since PipeWire holds each channel's share cubed.

How nodes keep time is read only while a watch on `timing` stands, as
the launcher's Sound watches it while a device's or a program's drawer
is out: the daemon binds PipeWire's profiler, as `pw-top` does, which
has the server measure every cycle, and lets it go once no watch wants
it. Each second, on a timer kept while it's bound, the cycles heard
stand as one frame, the figures `pw-top` prints (`src/timing.rs`), so a
node that stopped running drops out, and every node does once the whole
graph idles and the profiler says nothing.

## Acts

An `act` ask names its device by its record's key, and answers `done`
once PipeWire's loop has sent it on, or an error saying why not. What
it moved comes to every watch as the server says it back.

| Act      | Says                                  | Does                                  |
| -------- | ------------------------------------- | ------------------------------------- |
| `volume` | `volume` 0 to 1, and `muted` if set   | scales each channel alike to it       |
| `mute`   | `muted`                               | mutes it, or lets it be heard         |
| `use`    | nothing more                          | makes it the output or input chosen   |
| `port`   | `index`, one of its `ports`           | plays it through that port, saved     |
| `send`   | `target`: a device's node name, or "" | sends a stream there, or to the       |
|          |                                       | default, which it then follows        |

A card's own device takes its levels on the route that plays it, saved,
as the session manager keeps a route's levels and as Quickshell sets
them; any other device, a filter's or a virtual one, on its node's own
Props. Use goes into the default metadata's configured key, which the
session manager keeps and follows. A send names a program's stream as
`stream:<id>`, its PipeWire id, rather than a device, and goes into
the default metadata as the stream's `target.object`, which the session
manager moves it by, live. An act on a program names it as
`program:<device's node name>:<program's name>`, with its row's
`streams`, and acts on each of them: `volume` and `mute` on each
stream's own Props, and `send` on each stream's target. An act on a
device not heard is refused as `no-device`, one on a stream not heard
as `no-stream`, one missing what it needs as `bad-act`, a port its card
doesn't offer it as `no-port`, and any while the server is away as
`not-answering`.

## Sound's Rows

Each output and input is also served as the launcher's row for it
(`src/rows.rs`), in the form every daemon serves its rows in
(`../ready-rows`): named for what it is, wearing its kind's glyph,
struck through while muted, marked where it's chosen, its volume as a
bar in twentieths and its note worded, all from the launcher's
words.json, which the service names as `--vocabulary`. Outputs come
first, each way A to Z by that name, so the shell only places them by
`order` and draws them. A watch on `{ "kind": "row" }` hears them,
each sent again only when what it shows moves.

A jack with nothing plugged in, as the headphones' while none are, and
a display's output with no display aren't listed unless chosen, which
the note then says. The card says so of each route it has (its
EnumRoute), as a jack or a display comes and goes, so it's always
current; a device no card's routes play, as a USB or a Bluetooth one,
is always listed. Only the ports going its way, for its device, in the
profile the card is in count, as `wpctl` counts them.

Inside each device's row come the parts served of it (`src/parts.rs`),
a step in, in the order the launcher lists them: using it, where it
isn't the one chosen; whether it's Muted, one switch; its Volume, or a
microphone's Gain, with Louder, Quieter and four levels a step in; and
its Programs, or a microphone's Recording, each playing through it or
through a filter aimed at it (`src/programs.rs`); its Port, where its
card offers more than one, with each it can move to; its Channels, each
at its level (`src/channels.rs`); a Bluetooth device's Profile; and
last, what desk-facts says of its card (`src/desk.rs`), read from the
sound.json it keeps in `$XDG_RUNTIME_DIR/desk` and read again each time
it writes it, so the shell reads no file for Sound. Each part's act asks
the daemon's own act, so the shell runs no process to do it.

A device's programs are its streams grouped by the desktop entry each
names, as Quickshell's lookup finds it, A to Z by the entry's name and
wearing the mark the launcher gives it among Programs, from the marks
file the service names as `--marks`. Each says how loud it is, whether
it's paused, and holds Muted, Louder and Quieter; where there's more
than one device to choose from, Plays On (or Records From) with each it
can be sent to; and its first stream's Sample Rate and Delay.

## What Comes Next

The launcher's Sound lists these rows rather than making its own,
sends programs through the daemon rather than a `pw-metadata` each, and
reads how nodes keep time from it rather than from `pw-top`. A device's
drawer is still made in the shell, from Quickshell's nodes, these
timings and the parts served; a program's is served. Next come the rest
of each device's parts, its filters, hardware and Level, then its drawer
served whole.
