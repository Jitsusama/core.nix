# Hardware

One folder per machine model, named as its maker names the model: what that
model needs to run well, and nothing a person chose. Each folder's `nixos.nix`
opens with what the model is and imports the rest, and `flake.nix` exports it
under the folder's name. [docs/hardware.md][guide] says what works on each,
where every workaround comes from, and what's left.

| Here                               | Holds                                   |
| ---------------------------------- | --------------------------------------- |
| `apple-macbook-air-m2-a2681/`      | the 13-inch MacBook Air with an M2      |
| `apple-macbook-pro-m4-pro-a3401/`  | the 14-inch MacBook Pro with an M4 Pro  |
| `dell-xps-14-da14260/`             | the Dell XPS 14 of 2026                 |
| `microsoft-surface-laptop-4-1958/` | the 13.5-inch Surface Laptop 4 with AMD |

Whatever only one model needs, such as its kernel patches, lives in that
model's folder, with a `checks.nix` beside it where something there needs
proving.

[guide]: ../docs/hardware.md
