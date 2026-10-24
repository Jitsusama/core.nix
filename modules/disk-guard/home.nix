# The account's disk guard: a daemon that keeps the disks its sources lie on
# from filling, taking what regenerates, cheapest first, and never what a
# build or a person is using. It owns everything in the home: build targets
# and dependency installs, caches, temporary leftovers, the trash, finished
# quests' workspaces, the older builds a newer one replaced, and the account's
# own Nix generations. At critical with nothing left to take it freezes the
# writer filling the disk until there's room. Below healthy it says what it
# did in one notice, replaced in place, and it writes a status file for the
# shell. AGENTS.md beside this says how it decides.
#
# jitsusama.disk-guard.settings is the guard's own config, rendered as it is,
# and the build refuses one check-config refuses. Each source core gives is a
# default, replaced whole by a machine's own, or dropped by setting it null.
{
  config,
  lib,
  pkgs,
  # The machine's own settings, which Home Manager gives when it's part of
  # one, for whether Podman keeps the account's images.
  osConfig ? { },
  ...
}:
let
  package = pkgs.callPackage ./package.nix { };
  json = pkgs.formats.json { };
  settings = config.jitsusama.disk-guard.settings;
  home = config.home.homeDirectory;
  inherit (config.xdg)
    cacheHome
    dataHome
    stateHome
    ;

  # The coding harness's trees and workspaces, where agents build in parallel
  # and leave a target behind each.
  harness = {
    trees = "${stateHome}/pi/agentic-harness.pi/work/trees";
    workspaces = "${cacheHome}/pi/agentic-harness.pi/quest-workspace";
    quests = "${dataHome}/pi/agentic-harness.pi/quest-workflow/quests";
  };
  checkouts = [
    "${home}/src"
    harness.trees
    harness.workspaces
  ];

  # A source that refills on demand, emptied from urgent on, cheapest to
  # refill first: a download before a rebuild.
  cache = rank: path: {
    kind = "cache";
    inherit path rank;
  };

  # Whether a checkout's branch was merged as it stands: a merged pull
  # request whose head is the checkout's. More work on top isn't finished.
  merged = pkgs.writeShellApplication {
    name = "disk-guard-merged-on-github";
    runtimeInputs = [
      config.programs.gh.package
      pkgs.gnugrep
      pkgs.gnused
    ];
    text = ''
      repository=$(printf '%s' "$DISK_GUARD_REMOTE" |
        sed -E 's#^(https://github\.com/|git@github\.com:|ssh://git@github\.com/)##; s#\.git$##')
      if [ -z "$DISK_GUARD_BRANCH" ]; then
        echo '{"finished": false}'
        exit
      fi
      heads=$(gh pr list --repo "$repository" --head "$DISK_GUARD_BRANCH" \
        --state merged --json headRefOid --jq '.[].headRefOid')
      if printf '%s\n' "$heads" | grep -qxF "$DISK_GUARD_HEAD"; then
        echo '{"finished": true}'
      else
        echo '{"finished": false}'
      fi
    '';
  };

  podman = osConfig.virtualisation.podman or { };
  inherit (pkgs.stdenv.hostPlatform) isDarwin isLinux;

  # Predicates and worktree pruning ask git, which the guard finds on PATH
  # as the account's own programs do.
  path = lib.makeBinPath [ config.programs.git.package ];

  rendered = settings // {
    version = 1;
    scope = "user";
    sources = lib.filterAttrs (_: source: source != null) settings.sources;
  };

  # The config as the guard reads it, refused at build by the guard itself.
  configFile =
    pkgs.runCommandLocal "disk-guard-user.json"
      {
        text = builtins.toJSON rendered;
        passAsFile = [ "text" ];
      }
      ''
        ${lib.getExe package} check-config "$textPath"
        cp "$textPath" $out
      '';
in
{
  options.jitsusama.disk-guard.settings = lib.mkOption {
    type = lib.types.submodule { freeformType = json.type; };
    default = { };
    description = ''
      The account's guard's config: its bands, its sources and how it treats
      the account's Nix generations. A source set to null is dropped.
    '';
  };

  config = {
    jitsusama.disk-guard.settings = {
      status_file = lib.mkDefault true;
      # The generations Home Manager and nix profile make, which no system
      # daemon may expire for the account.
      nix.profiles = lib.mkDefault "${stateHome}/nix/profiles";
      sources = {
        src = lib.mkDefault {
          kind = "cargo";
          roots = checkouts;
        };
        installs = lib.mkDefault {
          kind = "installs";
          roots = checkouts;
        };
        workspaces = lib.mkDefault {
          kind = "workspaces";
          path = harness.workspaces;
          inherit (harness) quests;
        };
        # A Mac's /tmp and /var/tmp are links into /private.
        tmp = lib.mkDefault {
          kind = "temp";
          path = if isDarwin then "/private/tmp" else "/tmp";
        };
        var-tmp = lib.mkDefault {
          kind = "temp";
          path = if isDarwin then "/private/var/tmp" else "/var/tmp";
        };
        trash = lib.mkDefault {
          kind = "trash";
          path = if isDarwin then "${home}/.Trash" else "${dataHome}/Trash";
        };
        npm = lib.mkDefault (cache 10 "${home}/.npm/_cacache");
        # On a Mac, pnpm, Go and sccache keep their caches where a Mac's
        # programs do, under ~/Library, rather than where XDG says: pnpm
        # unless XDG_DATA_HOME is set, and Go and sccache unless GOCACHE or
        # SCCACHE_DIR is. Nothing here sets any of the three.
        pnpm = lib.mkDefault (
          cache 10 (if isDarwin then "${home}/Library/pnpm/store" else "${dataHome}/pnpm/store")
        );
        nix = lib.mkDefault (cache 20 "${cacheHome}/nix");
        # Go trims its own by when each entry was last used, so the guard
        # takes the oldest first rather than all of it.
        go-build = lib.mkDefault (
          cache 40 (if isDarwin then "${home}/Library/Caches/go-build" else "${cacheHome}/go-build")
          // {
            protocol = "go-build";
          }
        );
        sccache = lib.mkDefault (
          cache 60 (if isDarwin then "${home}/Library/Caches/Mozilla.sccache" else "${cacheHome}/sccache")
        );
        # Xcode's builds, which it makes again as a target is.
        derived-data = lib.mkIf isDarwin (
          lib.mkDefault (cache 50 "${home}/Library/Developer/Xcode/DerivedData")
        );
        # Homebrew's downloads and the versions it upgraded past, which it
        # prunes itself, weekly and from pressure on.
        homebrew = lib.mkIf (osConfig.homebrew.enable or false) (
          lib.mkDefault {
            kind = "producer";
            command = [
              "${osConfig.homebrew.prefix}/bin/brew"
              "cleanup"
              "--prune=all"
            ];
            path = "${home}/Library/Caches/Homebrew";
            every = "7d";
            from = "pressure";
            timeout = "10m";
          }
        );
        github = lib.mkIf config.programs.gh.enable (
          lib.mkDefault {
            kind = "finished";
            remote = "^(https://github\\.com/|git@github\\.com:|ssh://git@github\\.com/)";
            command = [ (lib.getExe merged) ];
            timeout = "20s";
          }
        );
        podman = lib.mkIf (podman.enable or false) (
          lib.mkDefault {
            kind = "podman";
            command = lib.getExe podman.package;
            path = "${dataHome}/containers/storage";
          }
        );
      };
    };

    home.packages = [ package ];
    xdg.configFile."disk-guard/config.json".source = configFile;

    launchd.agents.disk-guard = lib.mkIf isDarwin {
      enable = true;
      config = {
        ProgramArguments = [
          (lib.getExe package)
          "user"
          "--config"
          "${configFile}"
        ];
        EnvironmentVariables.PATH = path;
        RunAtLoad = true;
        # Started again whenever it stops, but not more than every 30 s.
        KeepAlive = true;
        ThrottleInterval = 30;
        # Whatever the account runs comes first, as background.slice does
        # on Linux.
        ProcessType = "Background";
        # Where a Mac keeps an account's logs, there before the guard is.
        StandardOutPath = "${home}/Library/Logs/disk-guard.log";
        StandardErrorPath = "${home}/Library/Logs/disk-guard.log";
      };
    };

    systemd.user.services.disk-guard = lib.mkIf isLinux {
      Unit.Description = "Keeps the account's disks from filling";
      Service = {
        ExecStart = "${lib.getExe package} user --config ${configFile}";
        Environment = [ "PATH=${path}" ];
        # Whatever the account runs comes first: the guard's own passes
        # yield, and its notices wait for nothing.
        Slice = "background.slice";
        # What the guard holds itself, 17 MiB at its peak while planning a
        # pass of 14,000 candidates on a workstation, is far under the high
        # mark, which reclaims the page cache its walks fill. The ceiling is the
        # producers it runs, Podman's prune among them, which need more than
        # the guard does.
        MemoryHigh = "128M";
        MemoryMax = "512M";
        # A command that still overruns it is killed and journaled, never
        # the guard.
        OOMPolicy = "continue";
        Restart = "on-failure";
        RestartSec = "30s";
      };
      Install.WantedBy = [ "default.target" ];
    };
  };
}
