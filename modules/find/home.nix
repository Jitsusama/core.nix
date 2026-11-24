# find on a Mac: macOS's own, run so a search of the home folder leaves the
# folders ripgrep's ignore file guards alone, as ripgrep and fd already do,
# since find reads no ignore file. Linux gets nothing, since nothing there
# asks.
{ lib, pkgs, ... }:
let
  find = pkgs.writeShellScriptBin "find" ''
    exec ${lib.getExe pkgs.bash} ${./find.sh} /usr/bin/find ${../ripgrep/ignore} "$@"
  '';
in
{
  # Ahead of anything else named find, so an agent's find of the home folder
  # is this one, whatever else the account installs.
  home.packages = lib.mkIf pkgs.stdenv.hostPlatform.isDarwin [ (lib.hiPrio find) ];
}
