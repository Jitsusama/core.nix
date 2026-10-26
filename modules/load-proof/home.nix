# load-proof says, signal by signal, whether what Joel uses still feels
# idle while the work behind him fills the machine (src/main.rs says how).
# It's a command, run when a change to the order needs proving, not a
# service: it draws nothing Joel can see, but while it runs it asks niri for
# a frame at every refresh, or saves a file under ~/.cache/load-proof a few
# times a second.
{ pkgs, ... }:
{
  home.packages = [ (pkgs.callPackage ./package.nix { }) ];
}
