# find on a Mac: macOS's own, run so a search of the home folder leaves the
# folders ripgrep's ignore file guards alone, as ripgrep and fd already do,
# since find reads no ignore file. Linux gets nothing, since nothing there
# asks.
{ lib, pkgs, ... }:
let
  # The data volume is where macOS mounts the volume the home folder is on.
  find = pkgs.writeShellScriptBin "find" ''
    exec ${lib.getExe pkgs.bash} ${./find.sh} /usr/bin/find ${../ripgrep/ignore} \
      /System/Volumes/Data "$@"
  '';
in
{
  # Ahead of anything else named find the account installs. Any program that
  # runs find by name from the account's profile, a script or an agent, gets
  # this one: a note on stderr, and the guarded folders left unsearched.
  home.packages = lib.mkIf pkgs.stdenv.hostPlatform.isDarwin [ (lib.hiPrio find) ];
}
