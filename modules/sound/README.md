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

A volume is the cube root of the channels' mean share, as `wpctl` and
Quickshell say it, since PipeWire holds each channel's share cubed.

## What Comes Next

The launcher's Sound reads its devices from here, where the socket
answers; then the daemon serves Sound's rows ready, with their levels,
programs and ports, and takes its acts (use, mute, volume, port) as asks,
so `pw-top`, `pw-metadata` and the per-move processes go.
