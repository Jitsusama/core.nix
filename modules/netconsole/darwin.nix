# Keeps the kernel logs other machines send over the local network with
# netconsole (nixos.nix beside this sends them), so a machine that hangs
# leaves its last words here. Every line goes to /var/log/netconsole.log, as
# it came, and only from the network named: anything from elsewhere is
# dropped unread (listen.nix). That's judged by the address a line says it's
# from, which its sender writes, and nothing records who sent each line, so
# a line here isn't proof of who sent it. The kernel's own carry its
# sequence number and time, but anything on the network can send numbers
# too, a whole boot's from 0 or a run past the last true one. So it's read,
# by a person or an agent, as words from the network: a line counts only
# when its number appears once, and the boot it's from is matched against
# the sending machine's own journal of that boot after it restarts. Nothing
# is kept while the Mac sleeps, so a hang is kept only while it's awake;
# this doesn't keep it awake, which is the Mac's own setting. The log is in
# a folder only root writes, as the listener runs as root. Anything on that
# network can send to it, so a line may hold a terminal's escapes; it's read
# with less or cat -v, which show them, rather than tail -f, which would act
# on them.
{
  config,
  lib,
  pkgs,
  ...
}:
let
  cfg = config.jitsusama.netconsole;
in
{
  options.jitsusama.netconsole = {
    from = lib.mkOption {
      type = lib.types.str;
      example = "192.168.1.0/24";
      description = ''
        The network whose machines may send, as an IPv4 address and its
        prefix length: the local network, so nothing beyond it is kept. A
        network elsewhere numbered the same is let in too.
      '';
    };
    port = lib.mkOption {
      type = lib.types.port;
      default = 6666;
      description = "The UDP port the kernel logs come in on.";
    };
  };

  config.launchd.daemons.netconsole.serviceConfig = {
    ProgramArguments = import ./listen.nix {
      socat = lib.getExe pkgs.socat;
      inherit (cfg) port from;
      log = "/var/log/netconsole.log";
    };
    # socat says why when it can't take the port or open the log.
    StandardErrorPath = "/var/log/netconsole.err";
    RunAtLoad = true;
    KeepAlive = true;
  };
}
