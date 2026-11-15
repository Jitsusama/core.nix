# wifid's halves in the account's session. Its sign-in window, which the
# shell opens when Joel asks to sign in to a network that wants it, and
# never by itself; it wears the GTK theme the session does. And what tells
# wifid when a call is on, which only the session can know, since a call
# is a program recording from a microphone through the session's PipeWire.
{ lib, pkgs, ... }:
{
  home.packages = [ (pkgs.callPackage ./sign-in/package.nix { }) ];

  # It follows PipeWire, as pause-for-calls does, since it reads calls from
  # pw-dump and has nothing to read without it. It's started again whenever
  # it ends, so a call is never left unsaid for long; wifid lets a call
  # nobody repeats lapse after ten minutes in any case (calls/src/main.rs).
  systemd.user.services.wifid-calls = lib.mkIf pkgs.stdenv.hostPlatform.isLinux {
    Unit = {
      Description = "Tells wifid when a call is on, so Wi-Fi holds its access point";
      After = [ "pipewire.service" ];
      BindsTo = [ "pipewire.service" ];
    };
    Service = {
      ExecStart = lib.getExe (pkgs.callPackage ./calls/package.nix { });
      Restart = "always";
      RestartSec = 2;
    };
    # The session wants it too, so a switch that brings it in starts it
    # while PipeWire already runs: on 15 Nov 2026 one didn't, since only
    # PipeWire starting would have, and wifid heard no calls.
    Install.WantedBy = [
      "pipewire.service"
      "default.target"
    ];
  };
}
