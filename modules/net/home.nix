# net reads the whole home network, every device and how they link, from
# each vendor's own source (src/main.rs says how). Its daemon reads the
# eero daemon's record and the cable modem's status pages once a minute and
# keeps $XDG_RUNTIME_DIR/net/network.json, the record every view of net
# reads (tests/network.json is its shape), and keeps a day of the figures
# that swing in $XDG_STATE_HOME/net/day.jsonl. It serves the same record,
# a record a device, link, source and concern, on its socket in the
# runtime folder, in the daemon interface every daemon speaks
# (docs/daemons.md), so a page or the launcher watches what moves rather
# than reading the file. It listens on nothing else, and its only requests
# are plain reads of the modem's pages on the home network, so it's
# background work, in background.slice, behind everything Joel uses. Away
# from home the modem doesn't answer, and the record says so.
{ lib, pkgs, ... }:
let
  net = pkgs.callPackage ./package.nix { };
in
{
  home.packages = [ net ];

  systemd.user.services.net = {
    Unit = {
      Description = "Keeps the whole home network's record for net's views";
      After = [
        "network-online.target"
        # Its socket is systemd's, handed to it as it starts.
        "net.socket"
      ];
      Requires = [ "net.socket" ];
      # Restarted after a switch reloads the units, it comes back as the
      # new one.
      X-SwitchMethod = "restart";
    };
    Service = {
      Slice = "background.slice";
      # Run by its name, so ps -ef reads net rather than a store path
      # (docs/daemons.md, Shown).
      ExecStart = "@${lib.getExe net} net daemon";
      Restart = "on-failure";
      RestartSec = 60;
    };
    Install.WantedBy = [ "default.target" ];
  };

  # Its socket, held by systemd whether or not the daemon runs, so a page
  # asking while it starts, or starts again, waits on it rather than
  # finding no socket; asking starts it, should it have stopped.
  systemd.user.sockets.net = {
    Unit.Description = "Where the shell and the sites ask net's daemon";
    Socket = {
      ListenStream = "%t/daemons/net.sock";
      SocketMode = "0600";
      DirectoryMode = "0700";
    };
    Install.WantedBy = [ "sockets.target" ];
  };
}
