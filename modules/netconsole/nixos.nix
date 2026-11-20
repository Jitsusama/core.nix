# Sends the kernel's log over the local network as it's written, to a
# listener on another machine (darwin.nix beside this is one), so the last
# words of a kernel that hangs are kept even when the journal never reaches
# the disk. A hard reset loses up to half a minute of journal, which btrfs
# commits every 30 s; netconsole sends each line as the kernel prints it,
# from the kernel itself, as long as the network card can still send.
#
# It sends through each interface named, once the interface is up with an
# address, and stops when it goes, as a dock's card does. Only the lines the
# kernel prints to its consoles go, which the machine's loglevel decides:
# at the usual 4, errors and worse, and all of a panic, which turns the
# loglevel up as it starts. So a hard lockup is sent with the panic's stack,
# which runs through the stuck core, but not the registers printed before
# it; a warning isn't sent; and a hang that never panics is sent as its
# headers alone, as a task blocked for two minutes, without the stack that
# names the driver. Raising
# the loglevel would print the rest on the screen too, and slow the kernel
# while it does. They go in the clear, to the machine that last answered for
# the listener's address on the interface's own link: kept while the
# listener sleeps there, and let go when the listener can't be found there
# at all, as on another network; netconsole.sh says more. They keep going
# through a shutdown, which can hang too. At a panic the card sends from
# inside the kernel with the other cores stopped, which a USB card may not
# manage past its first few lines. A hard lockup's panic writes pstore
# before it sends, so pstore keeps it whole either way; a panic in a running
# task sends first, so a card stuck on a lock a stopped core held loses
# pstore's copy too, and the watchdog restarts the machine a minute on.
{
  config,
  lib,
  pkgs,
  utils,
  ...
}:
let
  cfg = config.jitsusama.netconsole;

  netconsole = lib.getExe (
    pkgs.writeShellApplication {
      name = "netconsole";
      runtimeInputs = [
        pkgs.coreutils
        pkgs.gawk
        pkgs.getent
        pkgs.iproute2
        pkgs.iputils
      ];
      text = builtins.readFile ./netconsole.sh;
    }
  );

  waitOnline = "${config.systemd.package}/lib/systemd/systemd-networkd-wait-online";
  device = interface: "sys-subsystem-net-devices-${utils.escapeSystemdPath interface}.device";

  arguments = interface: "${interface} ${cfg.listener} ${toString cfg.port}";
  systemctl = "${config.systemd.package}/bin/systemctl";
  timer = interface: "netconsole-${interface}-follows.timer";

  send = interface: {
    description = "Sends the kernel's log through ${interface} to ${cfg.listener}";
    # udev starts it when the interface appears (rules, below), and it stops
    # when the interface goes. A shutdown doesn't stop it, so what the kernel
    # says while one hangs is sent too; the kernel stops sending itself when
    # the interface goes at the very end.
    unitConfig.DefaultDependencies = false;
    bindsTo = [ (device interface) ];
    # Its targets are made in configfs, in a folder netconsole's module makes
    # as it loads. The boot loads it first, but a switch that adds it starts
    # the module loader beside the system's targets, which start this, so it
    # waits on the module itself.
    requires = [ "sys-kernel-config.mount" ];
    wants = [ "modprobe@netconsole.service" ];
    after = [
      (device interface)
      "systemd-networkd.service"
      "sys-kernel-config.mount"
      "modprobe@netconsole.service"
    ];
    serviceConfig = {
      Type = "oneshot";
      RemainAfterExit = true;
      # An interface that gets no address in time is left to the start,
      # which says so.
      ExecStartPre = "-${waitOnline} --interface=${interface} --ipv4 --timeout=60";
      ExecStart = "${netconsole} start ${arguments interface}";
      # Each start, a retry's included, starts its following, which a
      # restart stops; one that would undo a shutdown is refused.
      ExecStartPost = "-${systemctl} start --no-block --job-mode=fail ${timer interface}";
      ExecStop = "${netconsole} stop ${interface}";
      # A listener asleep, not yet found by name on a link just up, or not
      # on this network at all leaves the sending up with nothing sent, and
      # its following tries again. Were it a failure, a switch while the
      # listener slept would fail on it, and could be rolled back.
      SuccessExitStatus = "69 75 78";
      Restart = "on-failure";
      RestartSec = "30s";
    };
  };

  # The kernel keeps the listener's hardware address, so it's asked again
  # every few minutes: a listener that slept, while another machine answered
  # for it, or that moved to another card, is sent to again. It runs only
  # while the sending does, and stops before it, so it can't make a target
  # again once the sending has ended it.
  follows = interface: {
    description = "Follows ${cfg.listener} for the kernel's log through ${interface}";
    # The timer paces it, so no start limit is hit by asking by hand too,
    # which would leave it failed, and fail a switch.
    unitConfig.StartLimitIntervalSec = 0;
    requisite = [ "netconsole-${interface}.service" ];
    partOf = [ "netconsole-${interface}.service" ];
    after = [ "netconsole-${interface}.service" ];
    serviceConfig = {
      Type = "oneshot";
      ExecStart = "${netconsole} follow ${arguments interface}";
      # A target the kernel refused is tried again at the next follow, so it
      # fails no switch.
      SuccessExitStatus = "75";
    };
  };
  # udev starts the sending as a card appears, so a switch that adds it
  # while the card is already in starts it here.
  starts = interface: {
    description = "Starts sending the kernel's log through ${interface} if it's there";
    wantedBy = [ "multi-user.target" ];
    unitConfig.ConditionPathExists = "/sys/class/net/${interface}";
    serviceConfig = {
      Type = "oneshot";
      RemainAfterExit = true;
      ExecStart = "${systemctl} start --no-block netconsole-${interface}.service";
    };
  };
  following = interface: {
    description = "Follows ${cfg.listener} for the kernel's log through ${interface} now and then";
    bindsTo = [ "netconsole-${interface}.service" ];
    after = [ "netconsole-${interface}.service" ];
    timerConfig = {
      # Soon after a start, which may have found nothing to send to, then
      # every two minutes.
      OnActiveSec = "30s";
      OnUnitActiveSec = "2min";
    };
  };
in
{
  options.jitsusama.netconsole = {
    listener = lib.mkOption {
      type = lib.types.str;
      example = "listener.local";
      description = ''
        The machine that keeps the kernel's log, by name or IPv4 address,
        on the network the interfaces reach. It's looked up each time an
        interface comes up and every two minutes after, and sent to
        when a machine answers for its address on the interface's own
        link, and while it sleeps there after; where it can't be found
        at all, nothing is sent until it is. A .local name is asked for
        on every network that asks by multicast, a trusted Wi-Fi as well
        as a cable in jitsusama.network.trustedCables, and the address it
        gives is sent to on the interface's own link, and kept while the
        name goes unanswered from the address the card sent from; an address is sent to on any
        network numbered the same where something holds it. Lines sent
        while the listener sleeps are lost, as they're sent only once.
      '';
    };
    through = lib.mkOption {
      type = lib.types.listOf lib.types.str;
      example = [ "enp0s20f0u2u2" ];
      description = ''
        The interfaces to send through, by name: a wired card on the same
        network as the listener, such as a dock's. A USB card's name
        follows the port it's in, so the same dock in another port isn't
        sent through; a systemd.network.links name given by its hardware
        address follows the card instead. Wi-Fi cards often can't send
        from inside the kernel's log, and nothing is sent through them
        unless they're named.
      '';
    };
    port = lib.mkOption {
      type = lib.types.port;
      default = 6666;
      description = "The UDP port the listener takes the kernel's log on.";
    };
  };

  # Whether an interface has its address yet is networkd's to say.
  imports = [ ../systemd-networkd/nixos.nix ];

  config = {
    boot.kernelModules = [ "netconsole" ];
    # A network device starts a unit through udev's SYSTEMD_WANTS, at boot as
    # when it's plugged in later; a device unit's own wants don't. A card
    # arrives under the kernel's name, eth0 say, and udev renames it once its
    # rules have run, so NAME matches the name it's given and KERNEL one it
    # already has, as a card named in the initrd does at boot.
    services.udev.extraRules = lib.concatMapStrings (
      interface:
      lib.concatMapStrings
        (
          name:
          lib.concatStringsSep ", " [
            ''ACTION!="remove"''
            ''SUBSYSTEM=="net"''
            ''${name}=="${interface}"''
            ''TAG+="systemd"''
            ''ENV{SYSTEMD_WANTS}+="netconsole-${interface}.service"''
          ]
          + "\n"
        )
        [
          "NAME"
          "KERNEL"
        ]
    ) cfg.through;
    systemd.services = lib.listToAttrs (
      lib.concatMap (interface: [
        (lib.nameValuePair "netconsole-${interface}" (send interface))
        (lib.nameValuePair "netconsole-${interface}-follows" (follows interface))
        (lib.nameValuePair "netconsole-${interface}-starts" (starts interface))
      ]) cfg.through
    );
    systemd.timers = lib.listToAttrs (
      map (
        interface: lib.nameValuePair "netconsole-${interface}-follows" (following interface)
      ) cfg.through
    );
  };
}
