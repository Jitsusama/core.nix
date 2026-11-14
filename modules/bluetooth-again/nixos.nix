# The command that starts a Bluetooth controller again once it stops
# answering, which bluetooth-links' notice and Machine › Bluetooth tell Joel
# to type: a controller wedged in its firmware answers nothing until its
# driver lets go of it and takes it back (src/main.rs says how). It writes
# root's files through sudo, so it runs as whoever types it and holds no
# privilege of its own.
{ pkgs, ... }:
{
  environment.systemPackages = [ (pkgs.callPackage ./package.nix { }) ];
}
