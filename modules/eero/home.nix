# eero reads the home network's eero from eero's cloud, which answers the
# same at home and away, since the eero itself has no local API
# (src/main.rs says how). Joel signs in once with `eero sign-in`, typing the
# code eero sends him, and the session is sealed for his account on this
# machine. Until then the daemon doesn't start.
#
# The daemon keeps $XDG_RUNTIME_DIR/eero/home-network.json, the whole home
# network as one record (home-network.json is its shape), and tells each
# change to whoever watches it on $XDG_RUNTIME_DIR/eero/socket, as the shell
# does from its start. It reads each part as often as its figures move, the
# network every 30 seconds while the Home Network or `eero --watch` looks and
# every five minutes when nothing does (src/schedule.rs
# says each pace). It keeps 30 days of what moves in history.sqlite beside
# the session, in the folder `eero sign-in` made, so the unit needs no
# state folder of its own, and times its own look at the internet with
# plain connects, which need no privilege. It's background work, a few
# small requests over the internet, so it runs in background.slice, behind
# everything Joel uses.
#
# The views (`eero status`, `eero nodes`, `eero devices` and the rest) read
# that record, and `eero <view> --watch` draws one again as the daemon tells
# its socket of each change. eero completes in zsh with the completion it
# writes itself (completion.nix), which asks eero for the views, and for
# the nodes, devices and profiles the record holds now.
{ lib, pkgs, ... }:
let
  eero = pkgs.callPackage ./package.nix { };
in
{
  home.packages = [
    eero
    (pkgs.callPackage ./completion.nix { })
  ];

  systemd.user.services.eero-daemon = {
    Unit = {
      Description = "Keeps the home network's eero for the shell";
      # %S is the account's state folder, where `eero sign-in` seals the
      # session.
      ConditionPathExists = "%S/eero/session.cred";
      After = [ "network-online.target" ];
    };
    Service = {
      Slice = "background.slice";
      ExecStart = "${lib.getExe eero} daemon";
      # systemd-creds opens the sealed session.
      Environment = [ "PATH=${lib.makeBinPath [ pkgs.systemd ]}" ];
      Restart = "on-failure";
      RestartSec = 60;
      # Its ask that Joel sign in again goes with it, however it stops, so
      # For You never shows an ask nothing is keeping current
      # (src/waiting.rs).
      ExecStopPost = "${pkgs.coreutils}/bin/rm -f %S/mission/waiting/eero.json";
    };
    Install.WantedBy = [ "default.target" ];
  };
}
