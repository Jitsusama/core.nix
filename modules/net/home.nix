# net reads the whole home network, every device and how they link, from
# each vendor's own source (src/main.rs says how). Its daemon reads the
# eero daemon's record and the cable modem's status pages once a minute and
# keeps $XDG_RUNTIME_DIR/net/network.json, the record every view of net
# reads (tests/network.json is its shape), and keeps a day of the figures
# that swing in $XDG_STATE_HOME/net/day.jsonl. It listens on nothing, and its
# only requests are plain reads of the modem's pages on the home network,
# so it's background work, in background.slice, behind everything Joel
# uses. Away from home the modem doesn't answer, and the record says so.
{ lib, pkgs, ... }:
let
  net = pkgs.callPackage ./package.nix { };
in
{
  home.packages = [ net ];

  systemd.user.services.net-daemon = {
    Unit = {
      Description = "Keeps the whole home network's record for net's views";
      After = [ "network-online.target" ];
    };
    Service = {
      Slice = "background.slice";
      ExecStart = "${lib.getExe net} daemon";
      Restart = "on-failure";
      RestartSec = 60;
    };
    Install.WantedBy = [ "default.target" ];
  };
}
