# Where a program's module adds a provider of its own to the launcher, as
# recents.nix is where it says what it had open lately, so a module such
# as Neovim's or Chrome's, which a machine without the shell imports too,
# can give the launcher its acts without bringing the shell along. home.nix
# declares the option whole, with its settings and what each means; this
# declares only that each provider is a set of settings, which the
# module system merges with home.nix's, so a provider set here is read
# there wherever the shell is, and is left unread wherever it isn't.
{ lib, ... }:
{
  options.jitsusama.quickshell.launcher.providers = lib.mkOption {
    type = lib.types.attrsOf (
      lib.types.submodule {
        freeformType = lib.types.attrsOf lib.types.anything;
      }
    );
  };
}
