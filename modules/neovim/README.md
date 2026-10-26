# Neovim

The account's editor, as [home.nix][home] sets it up. Beside its files:

- [`colors/`][colors]: the theme as Neovim's colour scheme, each colour one
  of the theme's, so changing the theme changes Neovim with the rest.
- [`patches/`][patches]: changes to plugins that are waiting on upstream,
  each saying why it's needed.
- [`plugins/`][plugins]: one Lua file for each plugin that needs setting up,
  named for the plugin and run as it starts or once it's wanted.
- [`probes/`][probes]: what the checks run inside a headless Neovim, each
  answering one question about it through [probe.sh][probe].

[home]: home.nix
[colors]: colors
[patches]: patches
[plugins]: plugins
[probes]: probes
[probe]: probe.sh
