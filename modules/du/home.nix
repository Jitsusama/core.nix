# du on a Mac: macOS's own, run so a count of the home folder leaves the
# folders ripgrep's ignore file guards alone, as ripgrep and fd already do,
# since du reads no ignore file. Linux gets nothing, since nothing there asks.
{ lib, pkgs, ... }:
let
  # The data volume is where macOS mounts the volume the home folder is on.
  du = pkgs.writeShellScriptBin "du" ''
    exec ${lib.getExe pkgs.bash} ${./du.sh} /usr/bin/du ${../ripgrep/ignore} \
      /System/Volumes/Data "$@"
  '';
in
{
  # Ahead of anything else named du, coreutils' among them, so an agent's du
  # of the home folder is this one, whatever else the account installs.
  home.packages = lib.mkIf pkgs.stdenv.hostPlatform.isDarwin [ (lib.hiPrio du) ];
}
