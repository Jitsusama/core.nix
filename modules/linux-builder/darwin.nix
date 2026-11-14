# Linux builds on a Mac: nixpkgs' builder VM on Apple's Virtualization
# framework, which builds aarch64-linux natively and x86_64-linux through
# Rosetta, in its own store. So a session working on the Mac builds its
# Linux checks there, rather than on whatever Linux machine it would
# otherwise send them to. The builder comes prebuilt from the Nix cache.
#
# It's written for a Mac whose Nix nix-darwin doesn't manage (`nix.enable =
# false`), where nix-darwin's own `nix.linux-builder` can't run and nix-darwin
# writes no nix.conf. So it runs the builder as nix-darwin would, and writes
# its line to /etc/nix/linux-builder.conf, which the machine's own
# nix.custom.conf includes:
#
#   !include linux-builder.conf
#
# It offers `big-parallel` and `benchmark`, never `kvm`: Rosetta runs no
# x86_64 VM, so a VM test is never sent there to crawl. Nor `uid-range` and
# `devnet`, which nixpkgs's guest offers since 26.11 for tests in nspawn
# containers: this line leaves those tests, x86_64 ones under Rosetta
# among them, to the Linux builders that run them natively. A machine adds
# features of its own in `features`, such as one that marks code allowed
# only on its own builders.
#
# Only the Mac reaches the VM, on a port of its own loopback. Another machine
# that builds there jumps through the Mac's SSH to that port, and signs in to
# the VM with a key of its own, given in `authorizedKeys`.
#
# A full store is freed from another machine with a key given in
# `collectableBy`: the VM lets it in only to run collect.sh, which collects
# garbage and says how much is free, with no shell, pty or forwarding. Every
# other key the VM takes is root's, on the Mac or elsewhere, so on 9 November
# a store down to 6 GB free, too little for the builds sent there, waited on
# Joel's password.
#
# A VM that wedges is cured by restarting it and the Nix daemon, which takes
# sudo. The accounts in `restartableBy` may run exactly those two restarts
# with no password, so someone at another machine can cure it over SSH.
{
  config,
  lib,
  pkgs,
  ...
}:
let
  cfg = config.jitsusama.linux-builder;
  # Whether the guest is changed from the one nixpkgs caches.
  changed = cfg.features != [ ] || cfg.authorizedKeys != [ ] || cfg.collectableBy != [ ];

  # Sizing it changes only the Mac's side of it, which the Mac builds in
  # seconds; the guest's system still comes from the cache. A key from
  # another machine changes the guest's, which the running VM then builds.
  builder = pkgs.darwin.linux-builder-vz.override {
    modules = [
      {
        virtualisation.cores = cfg.cores;
        virtualisation.darwin-builder = {
          memorySize = cfg.memory;
          diskSize = cfg.disk;
        };
      }
      # The guest's clock stops while the Mac sleeps, and nothing set it
      # again soon: on 8 November a build ran 2.8 hours behind its own
      # files, and make looped 80 minutes on "Clock skew detected". So
      # chrony asks each minute and steps any error past a second at any
      # time, which the module's own option can't say, rather than slew
      # hours away over days. The guest's real-time clock is the Mac's
      # passed through, so chrony doesn't trim it. Only a guest already
      # changed gets it: one given nothing stays the cached one, which a
      # Mac with no Linux builder yet can run without building it.
      (lib.mkIf changed {
        services.chrony = {
          enable = true;
          servers = [ ];
          makestep.enable = false;
          enableRTCTrimming = false;
          extraConfig = ''
            pool pool.ntp.org iburst maxpoll 6
            makestep 1 -1
          '';
        };
      })
      # nixpkgs' guest collects garbage only once 1 GiB is free, and stops
      # at 3: on 9 November a 60 GiB store sat 90 to 93% full and builds
      # there failed for disk, with no account but root able to collect.
      # So a changed guest starts at a sixth of its disk free and collects
      # back to five twelfths, 10 and 25 GiB of 60, as shares so a bigger
      # disk keeps as much room for its bigger builds.
      (lib.mkIf changed {
        virtualisation.darwin-builder = {
          min-free = cfg.disk * 1024 * 1024 / 6;
          max-free = cfg.disk * 1024 * 1024 * 5 / 12;
        };
      })
      (lib.mkIf (cfg.features != [ ]) {
        # Added to what the guest's Nix already offers, so it builds a piece
        # asking for one rather than refuse it.
        nix.settings.extra-system-features = cfg.features;
      })
      (lib.mkIf (cfg.authorizedKeys != [ ] || cfg.collectableBy != [ ]) {
        # nixpkgs' VM forces its SSH to read only the Mac's key, so the
        # file NixOS writes these to is forced beside it.
        services.openssh.authorizedKeysFiles = lib.mkForce [ "/etc/ssh/authorized_keys.d/%u" ];
      })
      { users.users.builder.openssh.authorizedKeys.keys = cfg.authorizedKeys; }
      (
        { config, pkgs, ... }:
        let
          collect = pkgs.writeShellApplication {
            name = "collect";
            runtimeInputs = [
              config.nix.package
              pkgs.coreutils
            ];
            text = builtins.readFile ./collect.sh;
          };
        in
        {
          # restrict turns off forwarding, the pty and ~/.ssh/rc, and the
          # command runs whatever the key's holder asked for.
          users.users.builder.openssh.authorizedKeys.keys = map (
            key: ''restrict,command="${lib.getExe collect}" ${key}''
          ) cfg.collectableBy;
        }
      )
    ];
  };
  guest = builder.nixosConfig;

  # The two restarts, each the whole command with its arguments, so sudo lets
  # nothing else through. sudoers reads these characters in an argument as
  # its own.
  restarts =
    map (label: "/bin/launchctl kickstart -k system/${lib.escape [ "\\" "," ":" "=" ] label}")
      [
        config.launchd.daemons.linux-builder-vz.serviceConfig.Label
        cfg.nixDaemon
      ];

  # Not nix-darwin's /var/lib/linux-builder, which it empties at every switch
  # while its own builder is off.
  home = "/var/lib/linux-builder-vz";

  line = lib.concatStringsSep " " [
    "ssh-ng://linux-builder"
    "x86_64-linux,aarch64-linux"
    "/etc/nix/builder_ed25519"
    (toString guest.virtualisation.cores)
    "1"
    (lib.concatStringsSep "," (
      [
        "benchmark"
        "big-parallel"
      ]
      ++ cfg.features
    ))
    "-"
    # nixpkgs' builder image carries this host key, as nix-darwin's says.
    "c3NoLWVkMjU1MTkgQUFBQUMzTnphQzFsWkRJMU5URTVBQUFBSUpCV2N4Yi9CbGFxdDFhdU90RStGOFFVV3JVb3RpQzVxQkorVXVFV2RWQ2Igcm9vdEBuaXhvcwo="
  ];
in
{
  options.jitsusama.linux-builder = {
    cores = lib.mkOption {
      type = lib.types.ints.positive;
      default = 4;
      description = ''
        The VM's cores and the builds it runs at once. Every core the Mac has
        costs the person nothing, since the VM runs at utility QoS.
      '';
    };
    memory = lib.mkOption {
      type = lib.types.ints.positive;
      default = 12 * 1024;
      description = ''
        The VM's memory in MiB. The VM has no balloon, so it keeps what its
        builds have touched until it stops: this is sized to their peak.
      '';
    };
    disk = lib.mkOption {
      type = lib.types.ints.positive;
      default = 60 * 1024;
      description = ''
        The VM's store in MiB, a sparse file, so it grows towards this. A
        guest given keys or features collects garbage once a sixth of it is
        free; the cached one, once under a GiB is.
      '';
    };
    features = lib.mkOption {
      type = lib.types.listOf (lib.types.strMatching "[a-z0-9-]+");
      default = [ ];
      example = [ "own-code" ];
      description = ''
        Features the VM offers besides `benchmark` and `big-parallel`, on the
        Mac's line and in the guest's Nix, so a piece asking for one is sent
        there and built. None by default, which keeps the guest the cached one.
      '';
    };
    authorizedKeys = lib.mkOption {
      type = lib.types.listOf lib.types.singleLineStr;
      default = [ ];
      example = [ "ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAA... root@elsewhere" ];
      description = ''
        Keys besides the Mac's own that may sign in to the VM as its builder,
        and so build there: another machine's, reaching the VM through the
        Mac. None by default, which keeps the guest's system the cached one.
      '';
    };
    collectableBy = lib.mkOption {
      type = lib.types.listOf lib.types.singleLineStr;
      default = [ ];
      example = [ "ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAA... agents@elsewhere" ];
      description = ''
        Keys that may sign in to the VM as its builder only to collect its
        store's garbage, all that's dead or up to a number of GiB, and read
        how much is free (collect.sh), reaching it through the Mac. None by
        default, which keeps the guest's system the cached one.
      '';
    };
    builder = lib.mkOption {
      type = lib.types.package;
      readOnly = true;
      internal = true;
      default = builder;
      description = "The VM as the Mac runs it, its guest's system in its `nixosConfig`.";
    };
    port = lib.mkOption {
      type = lib.types.port;
      readOnly = true;
      default = guest.virtualisation.darwin-builder.hostPort;
      description = ''
        The Mac's loopback port the VM's SSH answers on, for a machine that
        jumps through the Mac to it.
      '';
    };
    restartableBy = lib.mkOption {
      type = lib.types.listOf (lib.types.strMatching "[a-z_][a-z0-9._-]*");
      default = [ ];
      example = [ "someone" ];
      description = ''
        Accounts that may restart the VM and the Nix daemon with sudo and no
        password, and run nothing else that way, so a wedged VM is cured from
        another machine. None by default, which writes no sudo rule.
      '';
    };
    nixDaemon = lib.mkOption {
      type = lib.types.str;
      default = "systems.determinate.nix-daemon";
      description = ''
        The launchd label of the Nix daemon the machine's own installer runs.
        Nix reads its settings only as it starts, so it's restarted when the
        builder's line changes, and only then, since a restart ends the builds
        it's running.
      '';
    };
  };

  config = {
    system.activationScripts.preActivation.text = lib.mkAfter ''
      mkdir -p ${home}
    '';

    # After /etc is written: restart Nix only when the line it reads differs
    # from the one it was last restarted for.
    system.activationScripts.postActivation.text = lib.mkAfter ''
      read=$(/usr/bin/shasum -a 256 /etc/nix/linux-builder.conf | /usr/bin/cut -d' ' -f1)
      if [ "$read" != "$(/bin/cat ${home}/read-by-nix 2>/dev/null)" ] &&
        /bin/launchctl print system/${cfg.nixDaemon} >/dev/null 2>&1; then
        echo "restarting ${cfg.nixDaemon}, so it reads the Linux builder's line" >&2
        /bin/launchctl kickstart -k system/${cfg.nixDaemon} && echo "$read" > ${home}/read-by-nix
      fi
    '';

    launchd.daemons.linux-builder-vz = {
      # The VM trusts the certificates the host's NIX_SSL_CERT_FILE names, and
      # none without it, so the guest could fetch nothing from a cache. It's
      # the machine's own bundle, as nix-darwin's nix.linux-builder gives it.
      environment = lib.optionalAttrs (config.environment.variables ? NIX_SSL_CERT_FILE) {
        inherit (config.environment.variables) NIX_SSL_CERT_FILE;
      };
      # It shares certificates with the VM through TMPDIR, which macOS empties
      # of files three days unread, so it gets one of its own.
      script = ''
        export TMPDIR=/run/org.nixos.linux-builder-vz USE_TMPDIR=1
        rm -rf $TMPDIR
        mkdir -p $TMPDIR
        trap "rm -rf $TMPDIR" EXIT
        ${builder}/bin/create-builder
      '';
      serviceConfig = {
        # At utility QoS, which uses every core while the person's programs
        # are idle and yields when they want them. Background keeps a process
        # on the efficiency cores even when the rest are idle: a cargo build
        # took eight times as long under it.
        ProgramArguments = lib.mkBefore [
          "/usr/sbin/taskpolicy"
          "-c"
          "utility"
        ];
        KeepAlive = true;
        RunAtLoad = true;
        WorkingDirectory = home;
      };
    };

    security.sudo.extraConfig = lib.mkIf (cfg.restartableBy != [ ]) (
      lib.concatMapStrings (
        account: "${account} ALL=(root) NOPASSWD: ${lib.concatStringsSep ", " restarts}\n"
      ) cfg.restartableBy
    );

    environment.etc."ssh/ssh_config.d/100-linux-builder.conf".text = ''
      Host linux-builder
        User builder
        Hostname localhost
        HostKeyAlias linux-builder
        Port ${toString guest.virtualisation.darwin-builder.hostPort}
        IdentityFile /etc/nix/builder_ed25519
    '';

    environment.etc."nix/linux-builder.conf".text = ''
      builders = ${line}
      builders-use-substitutes = true
    '';
  };
}
