# The Sound Daemon

The sound server's outputs and inputs, held ready for the launcher over
`$XDG_RUNTIME_DIR/daemons/sound.sock`, in the shell's daemon interface
(version 1, as the windows and machine daemons speak it). The shell
imports it, so a machine with the shell runs it.

It hears PipeWire through its own API for as long as it runs: the
registry says each node as it comes and goes, each output's and input's
node says its properties and its Props as they change, and the default
metadata says which output and input are chosen. A watch is told only
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
| `row:sound:<node name>` | the launcher's row for it, ready to draw        | Sound's place           |

A volume is the mean of each channel's cube root, as Quickshell says it,
since PipeWire holds each channel's share cubed.

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

A card's own device takes its levels on the route that plays it, saved,
as the session manager keeps a route's levels and as Quickshell sets
them; any other device, a filter's or a virtual one, on its node's own
Props. Use goes into the default metadata's configured key, which the
session manager keeps and follows. An act on a device not heard is
refused as `no-device`, one missing what it needs as `bad-act`, a port
its card doesn't offer it as `no-port`, and any while the server is
away as `not-answering`.

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

## What Comes Next

The launcher's Sound lists these rows rather than making its own, so
it no longer reads desk-facts' file and its service's control group to
list, nor `pw-dump` for ports and `wpctl` to choose one; then each
device's parts and programs are served ready too, so `pw-top`,
`pw-metadata` and the per-move processes go.
