# Neovim's Plugin Settings

One Lua file for each plugin that needs setting up, named for the plugin.
[`../home.nix`][home] reads each into its plugin's config, in one of two ways:

- `configured` runs it as Neovim starts, for a plugin that draws the first
  screen.
- `waits` runs it once [`../later.lua`][later] has loaded the plugin: once the
  first screen is drawn for one marked `soon`, and otherwise the first time
  something asks for it.

A plugin that needs no settings has no file here.

[home]: ../home.nix
[later]: ../later.lua
