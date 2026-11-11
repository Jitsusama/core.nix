# Nix's side of a builder lent to the machine while its machine queue has
# builds one could take: Nix's build hook reads the builder's line from
# /run/nix-offload/machines once, as each build command starts, and an
# empty list keeps that whole command here. The account's nix-offload
# (home.nix) writes it, and the queue reads it to count the builder's room.
# The file is empty while nothing is lent, so Nix never waits on a builder
# that isn't there. Standing builders are in machines-with-standing alone,
# which only a build that names it reads. The lent builder's store, which
# a build sent there as a command reaches it by, is named in
# /run/nix-offload/store, and the
# builder being made in /run/nix-offload/lending, so the queue holds the
# builds it could take until the line is there.
#
# Root reaches the builder with a key of its own, made once here and never
# held in a Nix value; where the builder is, and how root's ssh gets there,
# is the machine's to say.
#
# Writing the file chooses the builders root's Nix trusts, whose outputs
# come back unsigned. That's a trusted user's power already, since Nix lets
# one name builders for any build, so only a trusted account may write it:
# the file then gives it nothing it didn't have.
#
# Once a machine says what `lines` trusts, root reads the lines from
# /run/nix-offload-lines instead, which an account of their own writes
# (src/lines.rs): it copies the lender's lines there, passing only those
# naming a builder it can vouch for, so the lender's account needs no
# trust to keep root safe.
{
  config,
  pkgs,
  lib,
  ...
}:
let
  keys = config.jitsusama.nix-offload.builderKeys;
  keysFile = "/run/nix-offload/builder-keys";
  lines = config.jitsusama.nix-offload.lines;
  linesFolder = "/run/nix-offload-lines";
  lineFiles = [
    "machines"
    "machines-with-standing"
    "machines-beside"
    "machines-for-anything"
  ];
  offload = pkgs.callPackage ./offload/package.nix { };
  # What src/lines.rs reads, as JSON.
  linesSettings = {
    from = "/run/nix-offload";
    into = linesFolder;
    files = lineFiles;
    key = "/etc/nix/offload_ed25519";
    inherit (lines)
      project
      prefix
      zones
      images
      ;
    startupScript = if lines.startupScript == null then null else "${lines.startupScript}";
    hookKey = "/etc/nix/offload_ed25519.pub";
    standing = map (builder: {
      inherit (builder) uri key hostKey;
    }) lines.standing;
    lent = map (builder: {
      inherit (builder) uri;
      instance = "${builder.folder}/instance";
      token = "${builder.folder}/cloud-token";
    }) lines.lent;
    gcloud = lib.getExe' lines.gcloud "gcloud";
  };
in
{
  options.jitsusama.nix-offload.account = lib.mkOption {
    type = lib.types.str;
    description = "The account whose nix-offload lends the builder, and so writes its line.";
  };

  options.jitsusama.nix-offload.builderKeys = lib.mkOption {
    type = lib.types.listOf lib.types.str;
    default = [ ];
    example = [ "builders-1:AAAA...=" ];
    description = ''
      The public keys the lent builders sign what they build with. Nix
      here trusts them, and `nix-offload there` brings what a lent builder
      built back into this store only when it's signed, rather than taking
      it unchecked as an account Nix trusts may. With none, it's taken
      unchecked.
    '';
  };

  options.jitsusama.nix-offload.lines = lib.mkOption {
    type = lib.types.nullOr (
      lib.types.submodule {
        options = {
          project = lib.mkOption {
            type = lib.types.str;
            description = "The cloud project lent builders are made in.";
          };
          prefix = lib.mkOption {
            type = lib.types.str;
            example = "offload-";
            description = "What every lent builder's instance name starts with.";
          };
          zones = lib.mkOption {
            type = lib.types.listOf lib.types.str;
            description = "The zones a lent builder may be made in.";
          };
          startupScript = lib.mkOption {
            type = lib.types.nullOr lib.types.path;
            default = null;
            description = ''
              The startup script every lent builder is made with. An instance
              running any other, or started again since root first trusted
              it, lends nothing. Unset, a lent builder is trusted on its host
              key alone, which anything able to change the instance can get
              round, and the machine warns so.
            '';
          };
          images = lib.mkOption {
            type = lib.types.listOf lib.types.str;
            default = [ ];
            example = [ "https://compute.example/projects/debian-cloud/global/images/debian-12-" ];
            description = "What a lent builder's boot disk's source image may start with.";
          };
          lent = lib.mkOption {
            type = lib.types.listOf (
              lib.types.submodule {
                options.uri = lib.mkOption {
                  type = lib.types.str;
                  example = "ssh-ng://nix-offload?compress=true";
                  description = "The URI its lender's lines name it by, whole.";
                };
                options.folder = lib.mkOption {
                  type = lib.types.str;
                  example = "/run/nix-offload";
                  description = ''
                    Its lender's folder, where the lender names its instance in
                    `instance`, `<name> <zone>`, and the token gcloud asks with in
                    `cloud-token`.
                  '';
                };
              }
            );
            description = "The lent builders whose lines may reach root, the first's and its stand-ins'.";
          };
          standing = lib.mkOption {
            type = lib.types.listOf (
              lib.types.submodule {
                options.uri = lib.mkOption { type = lib.types.str; };
                options.key = lib.mkOption { type = lib.types.str; };
                options.hostKey = lib.mkOption {
                  type = lib.types.str;
                  description = "Its host key as its line carries it, in base64.";
                };
              }
            );
            default = [ ];
            description = "The standing and beside builders whose lines may reach root.";
          };
          gcloud = lib.mkPackageOption pkgs "google-cloud-sdk" { };
        };
      }
    );
    default = null;
    description = ''
      What may reach root's Nix of the lender's lines. Set, root reads them
      from ${linesFolder}, which only an account of their own writes,
      passing a line only when it names one of these builders, a lent one
      with the host key the cloud gives for its instance, while the cloud
      says that instance is still as its maker made it: the startup script
      and image named here, no metadata that makes accounts, and root's
      own key for its hook. Unset, root reads
      the lender's own, and the lender's account must be trusted.
    '';
  };

  config = {
    warnings = lib.optional (lines != null && lines.startupScript == null) ''
      jitsusama.nix-offload.lines.startupScript is unset, so root trusts a
      lent builder on its host key alone, whatever its instance became.
    '';

    assertions =
      let
        account = config.jitsusama.nix-offload.account;
        user = config.users.users.${account} or { };
        groups = [ (user.group or "") ] ++ (user.extraGroups or [ ]);
        trusted = config.nix.settings.trusted-users;
      in
      [
        {
          assertion =
            lines != null || lib.elem account trusted || lib.any (group: lib.elem "@${group}" trusted) groups;
          message = ''
            jitsusama.nix-offload.account (${account}) writes the builders root's Nix
            trusts, so it must be one of nix.settings.trusted-users, unless
            jitsusama.nix-offload.lines says which of its lines may reach root.
          '';
        }
      ];

    # NixOS gives Nix no builders unless builds are distributed. Its own
    # nix.buildMachines would go to /etc/nix/machines, which this file takes
    # the place of, so a machine's other builders belong in it too.
    nix.distributedBuilds = true;
    nix.settings = {
      builders = if lines == null then "@/run/nix-offload/machines" else "@${linesFolder}/machines";
      # The builder fetches what a cache holds itself, rather than being
      # sent it from here.
      builders-use-substitutes = true;
      # Each build's cores travel with it: Nix sends a builder this
      # machine's own `cores`, and its daemon takes it over the builder's
      # (2.34's daemon.cc), so a builder's own setting never reaches what is
      # sent from here. At the default, 0, every build took all of a
      # 32-core builder: on 2 November make and ninja ran at -j32 beside
      # five VM tests, at load 136, and the VM tests ran past their
      # timeouts. Eight lets four big builds fill such a builder, and a
      # build here still has half of a 16-thread laptop.
      cores = lib.mkDefault 8;
      # What no builder takes builds here, two at a time. At the default,
      # one per thread, a build every builder declined could put sixteen
      # of eight cores each on a laptop: on 10 November a session's
      # `nix fmt` compiled the formatter's Rust here, beside the desktop.
      # An update's builds say their own, as update-to-main gives them.
      max-jobs = lib.mkDefault 2;
      extra-trusted-public-keys = lib.mkIf (keys != [ ]) keys;
    };

    # A builder near its end hands over to a successor, whose relay shares
    # the old one's port until the old one stops listening; a connection
    # waiting on that listener as it closes then goes to the successor's,
    # rather than being refused.
    boot.kernel.sysctl."net.ipv4.tcp_migrate_req" = 1;

    systemd.tmpfiles.rules = [
      "d /run/nix-offload 0755 ${config.jitsusama.nix-offload.account} - -"
      "f /run/nix-offload/machines 0644 ${config.jitsusama.nix-offload.account} - -"
      "f /run/nix-offload/machines-with-standing 0644 ${config.jitsusama.nix-offload.account} - -"
      "f /run/nix-offload/store 0644 ${config.jitsusama.nix-offload.account} - -"
      "f /run/nix-offload/lending 0644 ${config.jitsusama.nix-offload.account} - -"
    ]
    # The keys the lent builders sign with, root's, where `there` reads
    # whether to check what it brings back: the account's tools can't read
    # the daemon's settings.
    ++ lib.optional (keys != [ ]) "f+ ${keysFile} 0644 root root - ${toString keys}"
    ++ [
      # The folder of a second lender, for VM tests alone, whose files only
      # the account's `there` reads: Nix is never given its builder.
      "d /run/nix-offload-vm 0755 ${config.jitsusama.nix-offload.account} - -"
      # And of a second for VM tests, for those past the first's room.
      "d /run/nix-offload-vm-more 0755 ${config.jitsusama.nix-offload.account} - -"
    ]
    # And of up to fourteen past it, as home.nix's vmTests.extra lends them,
    # which stay empty until one does.
    ++ map (
      let
        account = config.jitsusama.nix-offload.account;
      in
      number: "d /run/nix-offload-vm-${toString number} 0755 ${account} - -"
    ) (lib.range 3 16)
    ++ [
      # And of a third, for what `there` sends while the first is full.
      "d /run/nix-offload-more 0755 ${config.jitsusama.nix-offload.account} - -"
    ]
    # The lines root reads, which only their own account writes.
    ++ lib.optionals (lines != null) (
      [ "d ${linesFolder} 0755 nix-offload-lines nix-offload-lines -" ]
      ++ map (file: "f ${linesFolder}/${file} 0644 nix-offload-lines nix-offload-lines -") lineFiles
    );

    # The lender leaves the cloud token readable by this account alone,
    # through an ACL naming it, rather than being in its group: a process
    # joins a group only when its account next logs in, and until then
    # couldn't hand the token over (offload/src/cloud.rs).
    users.users.nix-offload-lines = lib.mkIf (lines != null) {
      isSystemUser = true;
      group = "nix-offload-lines";
    };
    users.groups.nix-offload-lines = lib.mkIf (lines != null) { };

    environment.etc."nix-offload/lines.json" = lib.mkIf (lines != null) {
      text = builtins.toJSON linesSettings;
    };

    systemd.services.nix-offload-lines = lib.mkIf (lines != null) {
      description = "Passes root's Nix only the builder lines it can vouch for";
      wantedBy = [ "multi-user.target" ];
      after = [
        "network-online.target"
        "systemd-tmpfiles-setup.service"
      ];
      wants = [ "network-online.target" ];
      restartTriggers = [ config.environment.etc."nix-offload/lines.json".source ];
      serviceConfig = {
        ExecStart = "${lib.getExe offload} lines --config /etc/nix-offload/lines.json";
        User = "nix-offload-lines";
        Group = "nix-offload-lines";
        Restart = "always";
        RestartSec = 5;
        # gcloud keeps its state here, never in a home.
        StateDirectory = "nix-offload-lines";
        StateDirectoryMode = "0700";
        ReadWritePaths = [ linesFolder ];
        ProtectSystem = "strict";
        ProtectHome = true;
        PrivateTmp = true;
        NoNewPrivileges = true;
        CapabilityBoundingSet = "";
        RestrictSUIDSGID = true;
        LockPersonality = true;
      };
    };

    # After /etc, which /etc/nix belongs to.
    system.activationScripts.nix-offload-key = {
      deps = [ "etc" ];
      text = ''
        if [ ! -e /etc/nix/offload_ed25519 ]; then
          ${pkgs.openssh}/bin/ssh-keygen -q -t ed25519 -N "" -C nix-offload \
            -f /etc/nix/offload_ed25519
        fi
      '';
    };
  };
}
