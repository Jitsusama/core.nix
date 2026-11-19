# The checks of the launcher's emoji, which the folder's checks.nix gathers with
# every other place's; its opening comment says why each place keeps its own.
{
  lib,
  pkgs,
  ...
}:
{
  # The clipboard's history finds emoji by what they're called and what they
  # mean, best first, over the list the launcher is built with: its tests run
  # as it builds, with EMOJI_LIST naming the list, which its search test reads
  # (clipboard/src/search.rs).
  the-launcher-finds-emoji = (pkgs.callPackage ../clipboard/package.nix { }).overrideAttrs {
    EMOJI_LIST = import ../launcher/emoji.nix { inherit lib pkgs; } "none";
  };
}
