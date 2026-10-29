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
# x86_64 VM, so a VM test is never sent there to crawl.
{
  config,
  lib,
  pkgs,
  ...
}:
let
  cfg = config.jitsusama.linux-builder;

  # Sizing it changes only the Mac's side of it, which the Mac builds in
  # seconds; the guest's system still comes from the cache.
  builder = pkgs.darwin.linux-builder-vz.override {
    modules = [
      {
        virtualisation.cores = cfg.cores;
        virtualisation.darwin-builder = {
          memorySize = cfg.memory;
          diskSize = cfg.disk;
        };
      }
    ];
  };
  guest = builder.nixosConfig;

  # Not nix-darwin's /var/lib/linux-builder, which it empties at every switch
  # while its own builder is off.
  home = "/var/lib/linux-builder-vz";

  line = lib.concatStringsSep " " [
    "ssh-ng://linux-builder"
    "x86_64-linux,aarch64-linux"
    "/etc/nix/builder_ed25519"
    (toString guest.virtualisation.cores)
    "1"
    "benchmark,big-parallel"
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
        The VM's store in MiB, a sparse file it collects once under a GiB is
        free, so it grows towards this.
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
