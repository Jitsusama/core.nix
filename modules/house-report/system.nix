# What house-report is told on either system: where the shaped machine
# hears it, and which uplink to count. house-report (src/main.rs) tells
# the shaped machine each second what this one's uplink sent, so its
# shaper holds its own upload under the house's ceiling less what this one
# sends (jitsusama.shaper.networks.<router>.house there). nixos.nix and
# darwin.nix run it as a system service, which needs no one logged in, and
# on a Mac is let reach the local network without asking: macOS blocks an
# account's own agent there, with no prompt, for a program Nix built.
#
# The shaped machine hears reports only from the addresses its house
# names, and this one reports from the address of the interface it sends
# by, so give each interface it may send by (its Wi-Fi and its cable) a
# reserved address on that network, name each in the house's `from`, and
# leave the home network out of any VPN's tunnel here, or the reports
# leave by the tunnel and never arrive. The reporter says in its log which
# interface and address it sends by, and again each time that moves.
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
  # An address, not a name, as Rust's parse and the shaper's house read
  # one alike: the reporter would exit on anything else each time it
  # started.
  address = import ../shaper/address.nix lib;
  # IPv4 alone: over IPv6, macOS and NixOS alike send from a temporary
  # privacy address of their own by default, which no house can name, so
  # every report would be dropped without a word.
  ipv4 = to: !lib.hasInfix ":" to;
  to = "${cfg.to}:${port}";
in
{
  options.jitsusama.house-report = {
    to = lib.mkOption {
      type = lib.types.addCheck address ipv4 // {
        description = "IPv4 address";
      };
      example = "192.0.2.3";
      description = ''
        The shaped machine's IPv4 address on the network the two share,
        where its shaper hears this machine's reports. Not an IPv6 one:
        reports to it would go from a temporary address this machine
        makes, which the shaped machine's house can't name.
      '';
    };
    keyFile = lib.mkOption {
      type = lib.types.nullOr (
        lib.types.pathWith {
          inStore = false;
          absolute = true;
        }
      );
      default = null;
      example = "/etc/house-report/house.key";
      description = ''
        A file holding the house's key, the same bytes as the shaped
        machine's house names in its `keyFile`, at least 16, which each
        report is signed with, so a device on the line can't forge one
        from this machine's address. Make it root's alone, as a root
        shell does with `install -d -m 0755 /etc/house-report` and
        `install -m 0400 /dev/null /etc/house-report/house.key`, then
        writes the shaped machine's key into it, or makes one with
        `head -c 32 /dev/urandom > /etc/house-report/house.key` and
        writes the same bytes there: any account that can read the key
        may sign a report. A key another account could ever read is made
        again on both machines, the same way, rather than given a new
        mode, since a copy taken of it stays good (house keys round 2's
        F5 and F7). On Linux the service reads it
        through systemd's credentials, a copy made as it starts, so it
        may be root's alone: a key put there or swapped restarts the
        service, and while none is there the service fails to start,
        systemd saying why, and tries again every 5 s, writing a few
        journal lines each time, so put the key in place before naming
        it; one empty or short at start ends the service, which starts
        again on a fresh copy. On a Mac the service runs as nobody, so
        root copies the key for nobody's account, where the service reads
        it each second, at each switch and within seconds of the key or
        its folder changing, a minute at most; a copy refused or failed
        is taken away, so the service sends nothing until the key is
        right, and /var/log/house-report.log says why, once until the
        reason changes. There the build takes it only under /etc,
        /private/etc or /var/root, spelled plainly, and the copy only
        once every folder above it, every link resolved, is root's and
        written or taken by no other account, and the key itself is
        root's and read by no other. Never a link, a Nix path or a value
        in the store.
      '';
    };
    keyPath = lib.mkOption {
      type = lib.types.nullOr lib.types.str;
      internal = true;
      default = cfg.keyFile;
      description = "Where the service reads the key: on Linux its credential, on a Mac its copy.";
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
      ]
      ++ lib.optionals (cfg.keyPath != null) [
        "--key"
        cfg.keyPath
      ]
      ++ [ to ]
      ++ lib.optional (cfg.interface != null) cfg.interface;
    };
  };
}
