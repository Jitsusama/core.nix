# The XPS 14's Sound

What the laptop's speakers and microphones need beyond core's
PipeWire module. [docs/hardware.md][hardware] says how the tuning was
made and measured.

| File              | Holds                                                     |
| ----------------- | --------------------------------------------------------- |
| `speakers.nix`    | the speaker tuning, run as a PipeWire filter of its own   |
| `speakers.conf`   | the tuning itself, in PipeWire's own format               |
| `microphones.nix` | the built-in microphones, kept ready once opened          |

The amplifiers' kernel patches live with the laptop's other kernel
patches in [`../kernel/local/`][local].

[hardware]: ../../../docs/hardware.md
[local]: ../kernel/local/
