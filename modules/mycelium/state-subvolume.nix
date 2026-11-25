# Gives the node's state a Btrfs subvolume of its own, beside home.nix
# rather than in it, which is long enough already. The node writes its
# claims and what it shares all day, so on Linux it waits for its state to
# move into a tree of its own as the account's manager starts, rather than
# holding the home's root lock on every write beside Chrome's
# (modules/subvolumes/home.nix says what that lock cost).
{
  config,
  lib,
  pkgs,
  ...
}:
let
  cfg = config.jitsusama.mycelium;
  # Every realm's node keeps its state under this one folder, named as a
  # path from the home, which is how a folder is given its own subvolume.
  stateFolder = lib.removePrefix "${config.home.homeDirectory}/" "${config.xdg.stateHome}/mycelium";
in
{
  imports = [ ../subvolumes/home.nix ];

  config = lib.mkIf (pkgs.stdenv.hostPlatform.isLinux && cfg.realm != null) {
    jitsusama.subvolumes.folders.${stateFolder}.before = [
      "mycelium.service"
      "mycelium-balance.service"
    ]
    ++ map (realm: "mycelium-${realm}.service") (lib.attrNames cfg.alsoJoins);
  };
}
