# nixpkgs' settings on every machine, NixOS and nix-darwin alike.
#
# An unfree package is allowed only by name, so each one is a choice someone
# made and reviewed, not something a new dependency brings in unnoticed.
# When a package turns unfree, evaluation fails and names it. NixOS and
# nix-darwin join the lists every module gives, so a system module names its
# unfree packages beside the code that installs them, and a machine or a
# layer adds its own the same way: nixpkgs.config.allowUnfreePackages =
# [ "name" ]; with the name lib.getName gives, the package's pname. Broken
# and insecure packages stay refused.
{
  # home-manager uses the system's nixpkgs, so a home module can't set
  # nixpkgs' settings itself. The unfree packages core's home modules install
  # are named here instead, which allows each on every machine, whether or
  # not it imports the module that installs it.
  nixpkgs.config.allowUnfreePackages = [
    # Claude Code, which the claude and pi modules install.
    "claude-code"
    # The Spotify client, from the spotify module.
    "spotify"
    # Tuple, for pairing, from the tuple module.
    "tuple"
    # zapp, ZSA's flasher, which the ergodox module installs.
    "zapp"
    # Zoom's own client, from the zoom module.
    "zoom"
  ];
}
