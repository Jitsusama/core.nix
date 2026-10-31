# Dell XPS 14 (DA14260)

What this laptop needs beyond core's modules, by part. `nixos.nix`
says what the laptop is and imports the rest, and
[docs/hardware.md][hardware] says what works, where each workaround
comes from, and what's left.

| Here         | Holds                                                           |
| ------------ | --------------------------------------------------------------- |
| `nixos.nix`  | the laptop's settings: firmware, GPU, power, ports, sensors     |
| `checks.nix` | what proves the patches and settings here                       |
| `kernel/`    | every kernel patch, filed by where it stands upstream           |
| `camera/`    | the colour and IR cameras' HAL, driver and pipeline              |
| `sound/`     | the speakers' tuning and the microphones                        |
| `programs/`  | the NPU's compiler, and patches to programs nixpkgs builds      |

A file belongs to the part it serves; one that serves only this
laptop's kernel goes in `kernel/` whichever part it fixes.

[hardware]: ../../docs/hardware.md
