# What house-report is told on either system: where the shaped machine
# hears it, and which uplink to count. house-report (src/main.rs) tells
# the shaped machine each second what this one's uplink sent, so its
# shaper holds its own upload under the house's ceiling less what this one
# sends (jitsusama.shaper.networks.<router>.house there). nixos.nix and
# darwin.nix run it as a system service, which needs no one logged in, and
# on a Mac is let reach the local network without asking: macOS blocks an
# account's own agent there, with no prompt, for a program Nix built.
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
  # An address, not a name: the reporter would exit on a name each time it
  # started, and its service start it again forever. Its own parse judges
  # the rest.
  octet = "(25[0-5]|2[0-4][0-9]|1[0-9][0-9]|[1-9]?[0-9])";
  address =
    lib.types.addCheck lib.types.str (
      text:
      builtins.match "${octet}([.]${octet}){3}" text != null
      || builtins.match "[0-9a-fA-F:]*:[0-9a-fA-F:]*" text != null
    )
    // {
      description = "IPv4 or IPv6 address";
    };
  # An IPv6 address goes in brackets before its port.
  to = if lib.hasInfix ":" cfg.to then "[${cfg.to}]:${port}" else "${cfg.to}:${port}";
in
{
  options.jitsusama.house-report = {
    to = lib.mkOption {
      type = address;
      example = "192.0.2.3";
      description = ''
        The shaped machine's address on the network the two share, where
        its shaper hears this machine's reports.
      '';
    };
    interface = lib.mkOption {
      type = lib.types.nullOr lib.types.str;
      default = null;
      example = "en0";
      description = ''
        The interface whose sent bytes are reported. Left unset, it's the
        one this machine's routes send to the shaped machine by, read each
        second, so a cable plugged in beside the Wi-Fi is followed.
      '';
    };
    # What the service runs, for nixos.nix and darwin.nix to start.
    arguments = lib.mkOption {
      type = lib.types.listOf lib.types.str;
      internal = true;
      readOnly = true;
      default = [
        (lib.getExe package)
        to
      ]
      ++ lib.optional (cfg.interface != null) cfg.interface;
    };
  };
}
