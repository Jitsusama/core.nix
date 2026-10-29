# pi-packages installs each git package pi loads into a version of its own
# and swaps it in whole (src/lib.rs says how), so no update changes a file
# a running pi has loaded. pi is given each one as a local path, the link
# to its current version, and pi never installs or changes a package named
# that way, so `pi update` can't reach it either.
#
# It installs a package with no version yet at a switch, before pi's
# settings are written, and updates every fifteen minutes in the
# background, a switch starting a run without waiting on it; `pi-packages update` runs it now,
# after a merge, and `pi-packages status` says where each package is. A
# package whose first install fails, as with no network at the switch,
# has no link yet, and pi warns that it is missing until a later run
# installs it.
#
# A version goes once no pi runs it, keeping `current` and the version
# before it. A pi says which version it loaded in the package's `loaded/`
# folder, and each change there starts `pi-packages sweep`, which removes
# what no pi runs any more, so a version goes with its last pi.
{
  config,
  lib,
  pkgs,
  ...
}:
let
  cfg = config.jitsusama.pi-packages;
  package = pkgs.callPackage ./package.nix { };
  # The node and pnpm a version is installed and checked with, and git with
  # them, whatever a shell has on its PATH; pnpm runs install scripts
  # through sh, and its shims for a dependency's programs use sed. A native
  # module with no prebuilt binding for this node would need a compiler
  # too, which isn't given: its build fails, and the version is never
  # swapped in.
  tools = [
    pkgs.git
    cfg.node
    pkgs.pnpm
    pkgs.bash
    pkgs.coreutils
    pkgs.gnused
  ];
  gitPackages = builtins.filter (source: lib.hasPrefix "git:" source) cfg.sources;
  urlOf = source: "https://${lib.removePrefix "git:" source}";
  # Where each package's pis say what they loaded (src/loaded.rs).
  loadedDirs = map (source: "${cfg.root}/${lib.removePrefix "git:" source}/loaded") gitPackages;
  configFile = pkgs.writeText "pi-packages.json" (
    builtins.toJSON {
      inherit (cfg) root;
      packages = map (source: { url = urlOf source; }) gitPackages;
      git = lib.getExe pkgs.git;
      node = lib.getExe cfg.node;
      pnpm = lib.getExe pkgs.pnpm;
      path = map (tool: "${lib.getBin tool}/bin") tools ++ cfg.extraPath;
      # The account's one pnpm store (modules/pnpm), so a version clones
      # its files from there rather than filling a store of its own.
      env = lib.filterAttrs (
        name: _: name == "npm_config_store_dir" || name == "pnpm_config_store_dir"
      ) config.home.sessionVariables;
    }
  );
  command = "${lib.getExe package} update --config ${configFile}";
  # The command, told its settings, for the account to run.
  configured = pkgs.writeShellScriptBin "pi-packages" ''
    case "''${1:-status}" in
      update | sweep | status) exec ${lib.getExe package} "''${1:-status}" --config ${configFile} ;;
      *) echo "usage: pi-packages [status | update | sweep]" >&2; exit 2 ;;
    esac
  '';
in
{
  options.jitsusama.pi-packages = {
    sources = lib.mkOption {
      description = ''
        pi's packages as its settings name them. Each `git:` one is installed
        here; anything else is handed to pi as it is.
      '';
      type = lib.types.listOf lib.types.str;
      default = [ ];
    };
    root = lib.mkOption {
      description = "Where each package's mirror, versions and current link are kept.";
      type = lib.types.str;
      default = "${config.xdg.dataHome}/pi-packages";
      defaultText = lib.literalExpression ''"''${config.xdg.dataHome}/pi-packages"'';
    };
    extraPath = lib.mkOption {
      description = ''
        Folders a build may run programs from after its own, such as the
        credential helper the account's git config names for a private
        package. Its own git, node and pnpm always come first, and npm's
        settings from a shell never reach it.
      '';
      type = lib.types.listOf lib.types.str;
      default = [
        "${config.home.profileDirectory}/bin"
      ]
      ++ lib.optional pkgs.stdenv.hostPlatform.isLinux "/run/current-system/sw/bin";
      defaultText = lib.literalExpression ''[ "''${config.home.profileDirectory}/bin" "/run/current-system/sw/bin" ]'';
    };
    node = lib.mkOption {
      description = ''
        The node pi runs on, which a version's native modules are built for
        and loaded with before it is swapped in.
      '';
      type = lib.types.package;
      default = pkgs.nodejs;
      defaultText = lib.literalExpression "pkgs.nodejs";
    };
    links = lib.mkOption {
      description = "The sources pi is given: each `git:` one as its current link.";
      type = lib.types.listOf lib.types.str;
      readOnly = true;
      default = map (
        source:
        if lib.hasPrefix "git:" source then
          "${cfg.root}/${lib.removePrefix "git:" source}/current"
        else
          source
      ) cfg.sources;
    };
  };

  config = lib.mkIf (gitPackages != [ ]) {
    home.packages = [ configured ];

    # Before pi's settings name the links: a package with no version yet is
    # installed then and there, so a first switch has it, and a run that
    # fails leaves the switch to go on. Once each has one, a switch only
    # starts the service and goes on, rather than waiting on a fetch.
    home.activation.piPackagesInstall = lib.hm.dag.entryAfter [ "writeBoundary" ] ''
      missing=
      for link in ${lib.escapeShellArgs (builtins.filter (lib.hasSuffix "/current") cfg.links)}; do
        [ -e "$link" ] || missing=1
      done
      if [ -n "$missing" ]; then
        run ${command} || echo "pi-packages: not every package was installed; the next run tries again" >&2
      ${lib.optionalString pkgs.stdenv.hostPlatform.isLinux ''
        else
          run ${config.systemd.user.systemctlPath} --user start --no-block pi-packages.service || true
      ''}
      fi
    '';

    systemd.user.services.pi-packages = lib.mkIf pkgs.stdenv.hostPlatform.isLinux {
      Unit = {
        Description = "Installs pi's packages as whole versions and swaps them in";
        X-RestartIfChanged = false;
      };
      Service = {
        Type = "oneshot";
        ExecStart = command;
        Slice = "background.slice";
      };
    };
    systemd.user.timers.pi-packages = lib.mkIf pkgs.stdenv.hostPlatform.isLinux {
      Unit.Description = "Updates pi's packages every fifteen minutes";
      Timer = {
        OnBootSec = "5min";
        OnUnitActiveSec = "15min";
      };
      Install.WantedBy = [ "timers.target" ];
    };

    # A pi starting, reloading or ending changes what it says it loaded,
    # and a version no pi runs any more goes then, not at the next update.
    systemd.user.services.pi-packages-sweep = lib.mkIf pkgs.stdenv.hostPlatform.isLinux {
      Unit = {
        Description = "Removes the versions of pi's packages that no pi runs";
        X-RestartIfChanged = false;
      };
      Service = {
        Type = "oneshot";
        ExecStart = "${lib.getExe package} sweep --config ${configFile}";
        Slice = "background.slice";
      };
    };
    systemd.user.paths.pi-packages-sweep = lib.mkIf pkgs.stdenv.hostPlatform.isLinux {
      Unit.Description = "Watches what each pi says it loaded of its packages";
      Path = {
        PathChanged = loadedDirs;
        MakeDirectory = true;
      };
      Install.WantedBy = [ "paths.target" ];
    };

    launchd.agents.pi-packages = lib.mkIf pkgs.stdenv.hostPlatform.isDarwin {
      enable = true;
      config = {
        ProgramArguments = [
          (lib.getExe package)
          "update"
          "--config"
          "${configFile}"
        ];
        StartInterval = 900;
        ProcessType = "Background";
        LowPriorityIO = true;
      };
    };
    launchd.agents.pi-packages-sweep = lib.mkIf pkgs.stdenv.hostPlatform.isDarwin {
      enable = true;
      config = {
        ProgramArguments = [
          (lib.getExe package)
          "sweep"
          "--config"
          "${configFile}"
        ];
        WatchPaths = loadedDirs;
        ProcessType = "Background";
        LowPriorityIO = true;
      };
    };
  };
}
