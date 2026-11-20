# The harness's own scripts that Linux runs on a schedule or at login,
# each from where pi-packages installs the harness: its daily measure, the
# list of sessions a boot's end took away, the windows joined to sessions
# elsewhere, and the sweep of ended sessions' work trees. Each is skipped,
# not failed, on a machine without the harness or before its first
# install, and a Mac, with no systemd, gets none. The sweep's two settings,
# the modules it loads providers from and the folders their programs are
# in, are declared here beside it.
{
  config,
  lib,
  pkgs,
  ...
}:
let
  # Where pi-packages keeps the harness's current version, and the scripts
  # in it these run.
  scripts = "${config.jitsusama.pi-packages.root}/github.com/Jitsusama/agentic-harness.pi/current/scripts";
  measure = "${scripts}/measure.ts";
  restoreFile = "${scripts}/restore-file.ts";
  windowsElsewhere = "${scripts}/windows-elsewhere.ts";
  sweepScript = "${scripts}/pi-sweep.ts";
  sweepProviders = config.jitsusama.pi.sweep.providerModules;
  # Whether pi's SSH agent proxy runs here, as home.nix and host.nix decide
  # it, so the sweep is pointed at its socket only where there is one.
  agentProxied =
    config.jitsusama.pi.agentProxy.machines != [ ] || config.jitsusama.pi.agentProxy.builderKey != null;
  # The account's own git first, so the sweep judges and removes trees
  # with the git that made them, whatever its providers' folders hold.
  sweepPath = [
    (lib.makeBinPath [ config.programs.git.package ])
  ]
  ++ config.jitsusama.pi.sweep.path
  ++ [
    "${config.home.profileDirectory}/bin"
    "/run/current-system/sw/bin"
  ];
in
{
  options.jitsusama.pi.sweep.providerModules = lib.mkOption {
    type = lib.types.listOf lib.types.str;
    default = [ ];
    example = [ "/path/to/a/package/sweep-providers.ts" ];
    description = ''
      On Linux, modules the sweep of ended sessions' work trees loads its providers
      other than git's from, each one's default export a function giving
      them, as the hosts load them from their own packages. The sweep
      judges the trees of each provider it loads, and a host stands its
      own sweep down only while the sweep has judged every provider the
      host has, so one left out here keeps that host's whole sweep on its
      thread.
    '';
  };

  options.jitsusama.pi.sweep.path = lib.mkOption {
    type = lib.types.listOf lib.types.str;
    # The folders every realm's hosts put first, where the programs its
    # providers run already are, so a machine names them once.
    default = lib.unique (lib.concatMap (host: host.path) (lib.attrValues config.jitsusama.pi.hosts));
    defaultText = lib.literalExpression "every realm's jitsusama.pi.hosts.<realm>.path";
    example = [ "/path/to/a/toolchain/bin" ];
    description = ''
      On Linux, folders the sweep of ended sessions' work trees finds what its
      providers run in, after the account's git and before its profile: a
      provider that removes its trees with a program of its own finds it
      here, since the sweep's PATH is a fixed list rather than the user
      manager's. By default, the folders each realm's hosts put first.
    '';
  };

  config = lib.mkIf pkgs.stdenv.hostPlatform.isLinux {
    # The harness's measure: its cost audit of the last day against the day
    # before, and its learning sensor over the last seven days day by day,
    # as one report in its state folder's measure/. A day rather than a
    # week, so a harness change shows the day after it lands. The run reads
    # the day's session logs, so it waits behind whatever the account is
    # doing.
    systemd.user.services.pi-measure = {
      Unit = {
        Description = "Writes the pi harness's daily measure";
        ConditionPathExists = measure;
      };
      Service = {
        Type = "oneshot";
        ExecStart = "${lib.getExe pkgs.nodejs} ${measure}";
        Slice = "background.slice";
      };
    };
    systemd.user.timers.pi-measure = {
      Unit.Description = "Writes the pi harness's measure every morning";
      Timer = {
        OnCalendar = "*-*-* 06:00:00";
        RandomizedDelaySec = "10min";
        # A morning slept through runs once the machine wakes.
        Persistent = true;
      };
      Install.WantedBy = [ "timers.target" ];
    };

    # The launcher's list of the sessions the last boot's end took away,
    # written as the account's session starts, before any pi has: each pi
    # rewrites it as it starts, but until one does the file would still
    # describe an older boot. It reads only the harness's session records.
    systemd.user.services.pi-restore-file = {
      Unit = {
        Description = "Writes which pi sessions the last boot's end took away";
        ConditionPathExists = restoreFile;
      };
      Service = {
        Type = "oneshot";
        # The harness's package.json names no module type, on purpose: two
        # of its modules load with require. Node warns about that on every
        # run, which said nothing at each switch.
        ExecStart = "${lib.getExe pkgs.nodejs} --disable-warning=MODULE_TYPELESS_PACKAGE_JSON ${restoreFile}";
        Slice = "background.slice";
      };
      Install.WantedBy = [ "default.target" ];
    };

    # A session another machine holds runs on through this machine's
    # power-off, so what the boot takes away is only the window here
    # joined to it, and only this machine knows it was here. Every five
    # minutes the harness notes those windows, from kitty's and the
    # mycelium node's answers, so the next boot's list can offer them back.
    # Each minute it cost a node of 160 to 185 MB and 0.6 to 0.8 s of CPU,
    # 60 runs an hour that nearly always found nothing new (80ECO2,
    # 2026-11-12); a window joined in the five minutes before a power-off
    # is the one it misses.
    systemd.user.services.pi-windows-elsewhere = {
      Unit = {
        Description = "Notes the windows joined to sessions other machines hold";
        ConditionPathExists = windowsElsewhere;
      };
      Service = {
        Type = "oneshot";
        ExecStart = "${lib.getExe pkgs.nodejs} --disable-warning=MODULE_TYPELESS_PACKAGE_JSON ${windowsElsewhere}";
        Slice = "background.slice";
      };
    };
    systemd.user.timers.pi-windows-elsewhere = {
      Unit.Description = "Notes every five minutes the windows joined to sessions other machines hold";
      Timer = {
        OnCalendar = "*:0/5";
        AccuracySec = "10s";
      };
      Install.WantedBy = [ "timers.target" ];
    };

    # The sweep of ended sessions' work trees, in a process of its own
    # rather than on a pi host's one thread, where its git calls and folder
    # listings held every conversation in the host for up to 262 s on 19
    # November. Each sweep that runs to its end leaves a stamp naming the
    # providers whose trees it judged, and while that stamp is under two
    # hours old a host whose providers it names, every one, stands its own
    # sweeps down, so stopping this timer, or a sweep that keeps failing, hands
    # sweeping back to the hosts. It runs at the top of each hour by the
    # wall clock the stamp is dated by, which also fires on waking when a
    # sleep took the hour.
    systemd.user.services.pi-sweep = {
      Unit = {
        Description = "Sweeps ended pi sessions' work trees";
        ConditionPathExists = sweepScript;
        # A switch would otherwise stop a sweep partway through its trees.
        X-SwitchMethod = "keep-old";
      };
      Service = {
        Type = "oneshot";
        ExecStart = "${lib.getExe pkgs.nodejs} --disable-warning=MODULE_TYPELESS_PACKAGE_JSON ${sweepScript}";
        # Where agents' own commands run, beneath everything Joel uses, and
        # a place the polkit rules and the TPM agent's gate both take as an
        # agent's: its git runs in agents' trees, whose config may name a
        # command, so what it starts is answered as an agent's would be.
        Slice = "jobs-work.slice";
        # The account's git first, for the sweep's own calls, then what its
        # providers run, then the account's, for whatever a repository's
        # filters run. Quoted, as systemd splits an unquoted entry at its
        # spaces, with each % doubled so systemd reads none as a specifier.
        # Then the agents' SSH proxy, as a host gives its conversations, so
        # a provider that reaches a remote asks through the same gate; with
        # no proxy, no agent at all, rather than whichever the manager holds.
        Environment =
          map (entry: ''"${lib.replaceStrings [ "%" ] [ "%%" ] (lib.escape [ "\\" "\"" ] entry)}"'') (
            [ "PATH=${lib.concatStringsSep ":" sweepPath}" ]
            ++ lib.optional (
              sweepProviders != [ ]
            ) "PI_SWEEP_PROVIDERS=${lib.concatStringsSep ":" sweepProviders}"
          )
          ++ lib.optional agentProxied ''"SSH_AUTH_SOCK=%t/pi-agent.sock"'';
        # What would have node run code nobody gave it, as a pi host's unit
        # clears (host.nix's loadsCode), since this sweep runs git in every
        # session's trees as a host's would, and the TPM agent's socket, which
        # a host hides from its conversations too.
        UnsetEnvironment = [
          "SSH_TPM_AUTH_SOCK"
          "NODE_OPTIONS"
          "NODE_PATH"
          "NODE_COMPILE_CACHE"
          "LD_PRELOAD"
          "LD_AUDIT"
          "LD_LIBRARY_PATH"
          "OPENSSL_CONF"
          "OPENSSL_MODULES"
        ]
        ++ lib.optional (!agentProxied) "SSH_AUTH_SOCK";
        # A sweep still going after two hours is wedged. The hosts' sweeps
        # wait on the same lock meanwhile, so none sweeps beside it, and its
        # lock goes stale an hour after it stops, for the next sweep to take.
        TimeoutStartSec = "2h";
      };
    };
    systemd.user.timers.pi-sweep = {
      Unit.Description = "Sweeps ended pi sessions' work trees every hour";
      Timer = {
        OnCalendar = "hourly";
        # An hour the machine was off or logged out through runs once it's
        # back, as waking already does for a realtime timer.
        Persistent = true;
      };
      Install.WantedBy = [ "timers.target" ];
    };
  };
}
