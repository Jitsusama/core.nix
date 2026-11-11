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

| Kind                  | Figures                                          | Where                    |
| --------------------- | ------------------------------------------------ | ------------------------ |
| `device:<node name>`  | `sink`: whether it's an output                   | Outputs or Inputs        |
|                       | its node's id, name, description and nick        | the row's key and name   |
|                       | `volume`: as the shell shows it, and `volumes`   | the row's volume         |
|                       | `muted`                                          | the row's mute           |
|                       | `chosen`: whether the default metadata names it  | Use, and the row's mark  |
|                       | `properties`: every property of its node         | its drawer's facts       |

A volume is the mean of each channel's cube root, as Quickshell says it,
since PipeWire holds each channel's share cubed.

## Acts

An `act` ask names its device by its record's key, and answers `done`
once PipeWire's loop has sent it on, or an error saying why not. What
it moved comes to every watch as the server says it back.

| Act      | Says                                  | Does                                  |
| -------- | ------------------------------------- | ------------------------------------- |
| `volume` | `volume` as shown, and `muted` if set | scales each channel alike to it       |
| `mute`   | `muted`                               | mutes it, or lets it be heard         |
| `use`    | nothing more                          | makes it the output or input chosen   |

A card's own device takes its levels on the route that plays it, saved,
as the session manager keeps a route's levels and as Quickshell sets
them; any other device, a filter's or a virtual one, on its node's own
Props. Use goes into the default metadata's configured key, which the
session manager keeps and follows. An act on a device not heard is
refused as `no-device`, one missing what it needs as `bad-act`, and any
while the server is away as `not-answering`.

## What Comes Next

The launcher's Sound reads its devices from here and acts through it,
where the socket answers; then the daemon serves Sound's rows ready,
with their levels, programs and ports, and takes the port act too, so
`pw-top`, `pw-metadata` and the per-move processes go.
