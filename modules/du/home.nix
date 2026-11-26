# du on a Mac: macOS's own, run so a count never walks into the folders
# ripgrep's ignore file guards, as ripgrep and fd already don't, since du
# reads no ignore file. Linux gets nothing, since nothing there asks.
{
  config,
  lib,
  pkgs,
  ...
}:
let
  # The data volume is where macOS mounts the volume the home folder is on.
  # The home folder is the account's own, so a session with HOME moved
  # still leaves it alone.
  du = pkgs.writeShellScriptBin "du" ''
    exec ${lib.getExe pkgs.bash} ${./du.sh} /usr/bin/du ${../ripgrep/ignore} \
      /System/Volumes/Data ${lib.escapeShellArg config.home.homeDirectory} "$@"
  '';
in
{
  # Ahead of any other du in the account's own profile, coreutils' among
  # them, so an agent's du of the home folder is this one. An account that
  # installs coreutils for GNU's du gets macOS's under this name instead,
  # whose options are BSD's.
  home.packages = lib.mkIf pkgs.stdenv.hostPlatform.isDarwin [ (lib.hiPrio du) ];
}
