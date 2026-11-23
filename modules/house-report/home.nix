# The house's reporter, for a machine that shares a line with a shaped one:
# house-report (src/main.rs) tells the shaped machine each second what this
# one's uplink sent, so its shaper holds its own upload under the house's
# ceiling less what this one sends (jitsusama.shaper.networks.<router>.house
# there). It runs as the account's own agent, under launchd on a Mac and
# the user manager on Linux, and needs no administrator.
#
# The shaped machine hears reports only from the address its house names,
# so give this machine a reserved address on that network, and leave the
# home network out of any VPN's tunnel here, or the reports leave by the
# tunnel and never arrive.
{
  config,
  lib,
  pkgs,
  ...
}:
let
  package = pkgs.callPackage ./package.nix { };
  cfg = config.jitsusama.house-report;
  inherit (pkgs.stdenv.hostPlatform) isDarwin isLinux;
  # The port the shaper hears reports on, read from it so the two can't
  # part.
  port = lib.pipe ../shaper/src/house.rs [
    builtins.readFile
    (lib.splitString "\n")
    (map (builtins.match "pub const PORT: u16 = ([0-9_]+);"))
    (lib.findFirst (found: found != null) (
      throw "house-report: no `pub const PORT: u16` line in modules/shaper/src/house.rs"
    ))
    lib.head
    (lib.replaceStrings [ "_" ] [ "" ])
  ];
  # An IPv6 address goes in brackets before its port.
  to = if lib.hasInfix ":" cfg.to then "[${cfg.to}]:${port}" else "${cfg.to}:${port}";
  arguments = [
    (lib.getExe package)
    to
    cfg.interface
  ];
in
{
  options.jitsusama.house-report = {
    to = lib.mkOption {
      type = lib.types.str;
      example = "192.0.2.3";
      description = ''
        The shaped machine's address on the network the two share, where
        its shaper hears this machine's reports.
      '';
    };
    interface = lib.mkOption {
      type = lib.types.str;
      example = "en0";
      description = ''
        The interface whose sent bytes are reported: the one this machine
        sends up the shared line by, its Wi-Fi or its cable.
      '';
    };
  };

  config = {
    systemd.user.services.house-report = lib.mkIf isLinux {
      Unit.Description = "Tells the shaped machine on this line what this one sends up";
      Service = {
        ExecStart = lib.escapeShellArgs arguments;
        Restart = "always";
        RestartSec = 5;
        Slice = "background.slice";
      };
      Install.WantedBy = [ "default.target" ];
    };

    launchd.agents.house-report = lib.mkIf isDarwin {
      enable = true;
      config = {
        ProgramArguments = arguments;
        RunAtLoad = true;
        # launchd waits 10 s between starts should it end.
        KeepAlive = true;
        # Whatever the account runs comes first, as background.slice does on
        # Linux.
        ProcessType = "Background";
        StandardErrorPath = "${config.home.homeDirectory}/Library/Logs/house-report.log";
      };
    };
  };
}
