# Shared pi hosts: for each realm a machine names in jitsusama.pi.hosts, two
# long-lived processes, one for the conversations Joel watches and one for
# those that run in the background, each holding many pi conversations
# behind a socket of its own. The harness's host program is the process,
# run by the node the host's pi was built with; the pi it hosts carries the
# host's patches (checks.nix, pi-hosted-keeps-turns-its-own), and every pi
# an account starts by hand stays unpatched. A machine that names no realm
# gets no host. Linux only, for now.
#
# No pi reaches a host unproven. The harness's scripts/host-prove.ts writes a
# runtime's proof once `pnpm test:host` passes on it with no case skipped,
# and a realm names that proof beside its pi. A proof that doesn't name the
# runtime the pi is in fails evaluation, so a bump without its proof can't
# build a generation on any machine, and a check that evaluates one refuses
# it before it merges. The host program checks the proof again as it starts.
{
  config,
  lib,
  pkgs,
  ...
}:
let
  cfg = config.jitsusama.pi.hosts;

  # The harness's host program, in the version pi-packages calls current.
  # node follows the link as it loads it, so a running host keeps the version
  # it started with through an update, and the next start takes the new one.
  entry = "${config.jitsusama.pi-packages.root}/github.com/Jitsusama/agentic-harness.pi/current/scripts/pi-host.ts";

  homeRealm = lib.attrByPath [ "jitsusama" "mycelium" "realm" ] null config;

  # Why a realm's proof doesn't prove its pi, or null when it does. The pi is
  # a folder inside the runtime the proof names, so the runtime is compared
  # with a slash after it, and one whose name merely starts the same fails.
  proofRefusal =
    realm: host:
    let
      proof = builtins.fromJSON (builtins.readFile host.proof);
    in
    if (proof.kind or null) != "pi-host-runtime-proof" then
      "jitsusama.pi.hosts.${realm}.proof isn't a pi-host-runtime-proof"
    else if (proof.version or null) != 1 then
      "jitsusama.pi.hosts.${realm}.proof is version ${toString (proof.version or "none")}, not 1"
    else if !(lib.hasPrefix "${proof.runtime or ""}/" host.pi) then
      "jitsusama.pi.hosts.${realm}.proof proves ${proof.runtime or "nothing"}, not the runtime ${host.pi} is in; prove it with the harness's node scripts/host-prove.ts"
    else
      null;

  # Says, as a notice and in the journal, that a host was left down
  # (host-down.sh says how), so it is heard of at once.
  down = pkgs.writeShellApplication {
    name = "pi-host-down";
    runtimeInputs = [
      pkgs.systemd
      pkgs.libnotify
    ];
    text = builtins.readFile ./host-down.sh;
  };

  kinds = {
    attended = "the pi conversations Joel watches";
    background = "the pi conversations that run in the background";
  };

  hostOf = realm: host: kind: {
    name = "pi-host-${realm}-${kind}";
    value = {
      Unit = {
        Description = "Holds ${kinds.${kind}} in the ${realm} realm";
        # Until the harness ships the program there is nothing to run. systemd
        # judges this only as the unit starts, so a realm named before then
        # waits for the next login.
        ConditionPathExists = entry;
        # A restart parks every conversation the host holds and ends the turns
        # they are taking, so a switch leaves a running host be. A new pi or a
        # new harness reaches it at its next start: a login, a crash, or a
        # restart its bump gate chooses.
        X-SwitchMethod = "keep-old";
        # A host that crashes again each time it comes back would park and
        # resume every conversation it holds every few seconds, so a fourth
        # start within ten minutes is refused and the unit stays failed,
        # saying start-limit-hit in its status, until it is fixed. Every start
        # counts, a restart by hand included; `systemctl --user reset-failed`
        # clears the count.
        StartLimitIntervalSec = 600;
        StartLimitBurst = 3;
        # A host left down, at its start limit or on the exit it isn't
        # restarted for, holds its conversations out of reach until someone
        # starts it, so Joel is told at once. A crash or a watchdog's kill it
        # is restarted from doesn't reach this, since RestartMode below skips
        # the failed state a restart would pass through; the journal and
        # cadence's evening read keep those.
        OnFailure = [ "pi-host-down@%N.service" ];
      };
      Service = {
        # The host says it's ready once its socket listens, which is when a
        # conversation can be sent to it; it says so from a child process.
        Type = "notify";
        NotifyAccess = "all";
        ExecStart = lib.escapeShellArgs (
          # Environment= can only replace PATH, so env puts the realm's folders
          # before the user manager's; systemd expands ${PATH}, and env execs
          # node in its place, the pid systemd watches. A folder's % and $ are
          # doubled, so systemd reads neither as a specifier or a variable. The
          # manager always has a PATH on NixOS, so the list never ends in the
          # empty entry that would mean the working folder.
          lib.optionals (host.path != [ ]) [
            "${pkgs.coreutils}/bin/env"
            "PATH=${
              lib.concatMapStringsSep ":" (lib.replaceStrings [ "%" "$" ] [ "%%" "$$" ]) host.path
            }:\${PATH}"
          ]
          ++ [
            host.node
            # The harness's package.json names no module type, on purpose.
            "--disable-warning=MODULE_TYPELESS_PACKAGE_JSON"
            entry
            kind
          ]
        );
        # Quoted, as systemd splits an unquoted entry at its spaces.
        Environment = lib.mapAttrsToList (name: value: ''"${name}=${lib.escape [ "\\" "\"" ] value}"'') (
          host.environment
          // {
            MYCELIUM_REALM = realm;
            # pi reads its own files from here, and a subagent a hosted
            # conversation starts runs this pi too.
            PI_PACKAGE_DIR = host.pi;
            # The host checks it against its pi as it starts.
            PI_HOST_PROOF = "${host.proof}";
          }
          // lib.optionalAttrs (homeRealm != null) { PI_HOME_REALM = homeRealm; }
        );
        # The host parks its conversations and keeps their records when told
        # to stop, so only its own failure restarts it; a crash's next start
        # takes back what it held.
        Restart = "on-failure";
        RestartSec = "5s";
        # A restart goes straight back to starting rather than through
        # failed, which would tell Joel the host is down at every crash it
        # comes back from; once no restart comes, it fails as before.
        RestartMode = "direct";
        # The commands its conversations run share its unit, so systemd's
        # default, stopping the whole unit when the kernel kills any of them
        # for memory, would park every conversation over one build. A plain
        # pi outlives its command, and so does the host; the host's own death
        # still restarts it.
        OOMPolicy = "continue";
        # A host whose loop stops turning answers nobody yet stays active, so
        # once ready it pings from its own loop four times in each of these,
        # and one gone quiet for the whole of it is killed and restarted, to
        # resume what it holds. A turn is awaited, never run on the loop, so
        # no turn's length stops the pings. Probing both hosts' sockets once
        # a second through a night's work on 12 November, the longest an
        # answer waited was 6.5 s; on 11 November a starved machine ran 13 s
        # of work in 96 s, and two minutes leaves the loop that slowdown five
        # times over. systemd kills a quiet host with SIGABRT, so its core,
        # the one record of where its loop wedged, goes to systemd-coredump;
        # writing it may take TimeoutStopSec's minute before SIGKILL.
        WatchdogSec = "2min";
        # The host exits 78 when what it was given is refused (its kind, its
        # realm, a pi missing a host patch), which another start would only
        # refuse again, so it waits for a fix rather than retrying forever.
        RestartPreventExitStatus = "78";
        # Parking ends each turn in flight, the tools it runs with it; what is
        # still running after that is ended with it.
        KillMode = "mixed";
        TimeoutStopSec = "60s";
        # Its conversations are agents' sessions, so they run among the
        # sessions rather than among Joel's programs (modules/slices).
        Slice = "agents.slice";
      };
      Install.WantedBy = [ "default.target" ];
    };
  };
in
{
  options.jitsusama.pi.hosts = lib.mkOption {
    type = lib.types.attrsOf (
      lib.types.submodule {
        options = {
          pi = lib.mkOption {
            type = lib.types.path;
            description = ''
              The package directory of a pi built with the host's patches, where
              its dist and node_modules are. The host refuses a pi without them.
            '';
          };
          proof = lib.mkOption {
            type = lib.types.path;
            description = ''
              The proof the harness's scripts/host-prove.ts wrote once `pnpm test:host`
              passed on the runtime `pi` is in, with no case skipped. Evaluation fails
              when it names another runtime.
            '';
          };
          node = lib.mkOption {
            type = lib.types.path;
            description = ''
              The node that pi was built with, which its native modules are
              built for; the host runs on it.
            '';
          };
          path = lib.mkOption {
            type = lib.types.listOf lib.types.str;
            default = [ ];
            example = [ "/home/someone/.local/share/dev/bin" ];
            description = ''
              Folders put before the user manager's PATH, for the commands the
              realm's launcher finds on a shell's PATH and a user service's
              lacks: those the host runs for its credential, and those its
              conversations' tools run.
            '';
          };
          environment = lib.mkOption {
            type = lib.types.attrsOf lib.types.str;
            default = { };
            example = {
              PROXY_ORIGIN = "https://proxy.example.com";
            };
            description = ''
              What the realm's own pi launcher would give every pi it starts,
              as the host never runs that launcher. No credential: the host
              resolves its own, and anything set for one launch belongs to the
              conversation, not here.
            '';
          };
        };
      }
    );
    default = { };
    description = ''
      The realms whose pi conversations run in shared hosts, by realm name, each
      with an attended and a background host started with the account's session.
    '';
  };

  config.assertions = lib.mapAttrsToList (
    realm: host:
    let
      refused = proofRefusal realm host;
    in
    {
      assertion = refused == null;
      message = toString refused;
    }
  ) cfg;

  config.systemd.user.services = lib.mkIf pkgs.stdenv.hostPlatform.isLinux (
    lib.listToAttrs (
      lib.concatLists (
        lib.mapAttrsToList (realm: host: map (hostOf realm host) (lib.attrNames kinds)) cfg
      )
    )
    // lib.optionalAttrs (cfg != { }) {
      "pi-host-down@" = {
        Unit.Description = "Says that the pi host %i was left down";
        Service = {
          Type = "oneshot";
          ExecStart = "${lib.getExe down} %i";
          Slice = "background.slice";
        };
      };
    }
  );
}
