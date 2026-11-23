# nixpkgs' settings on every machine, NixOS and nix-darwin alike.
#
# Only unfree packages named here are allowed, so each one is a choice
# someone made and reviewed, not something a new dependency brings in
# unnoticed. When a package turns unfree, evaluation fails and names it.
# Lists of names from every module are joined, so each module names its own
# packages beside the code that installs them, and a machine or a layer
# adds its own the same way: nixpkgs.config.allowUnfreePackages = [ "name" ];
# with the name lib.getName gives, the package's pname. Broken and insecure
# packages stay refused.
{
  # home-manager uses the system's nixpkgs, so a home module can't set
  # nixpkgs' settings itself. The unfree packages home modules install are
  # named here instead.
  nixpkgs.config.allowUnfreePackages = [
    # Claude Code, which the pi module installs.
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
