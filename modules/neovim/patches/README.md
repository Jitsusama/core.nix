# Neovim's Plugin Patches

Changes to plugins as nixpkgs builds them, each a patch named for the plugin
and what it touches, which [`../home.nix`][home] adds to that plugin's own
patches.

- `aerial-fzf-lua.patch`: aerial's fzf-lua picker requires fzf-lua as its
  file loads; patched, it returns an empty module when fzf-lua can't be
  required, rather than failing.

[home]: ../home.nix
