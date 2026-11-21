# Shared pi hosts: for each realm a machine names in jitsusama.pi.hosts, two
# long-lived processes, one for the conversations Joel watches and one for
# those that run in the background, each holding many pi conversations
# behind a socket of its own. The harness's host program is the process,
# run by the node the host's pi was built with; the pi it hosts carries the
# host's patches (checks.nix, pi-hosted-keeps-turns-its-own), and every pi
# an account starts by hand stays unpatched. Each host may run a
# conversation in a process of its own, from a template unit beside it
# (conversationOf). A machine that names no realm gets no host. Linux only,
# for now.
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

  # Whether pi's SSH agent proxy runs here, as home.nix decides it, so a
  # host is pointed at its socket only where there is one. Hosts are Linux
  # alone, so the platform needs no test of its own. A configuration
  # without home.nix (a VM test's) has no proxy.
  agentProxied =
    lib.attrByPath [ "jitsusama" "pi" "agentProxy" "machines" ] [ ] config != [ ]
    || lib.attrByPath [ "jitsusama" "pi" "agentProxy" "builderKey" ] null config != null;

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

  # How long systemd gives a host to stop at a restart or a switch, in
  # seconds. The unit tells the host the same number (PI_HOST_STOP_USEC),
  # which parks within three quarters of it, so its own limit comes first
  # there. At logout, reboot or poweroff the manager's 5 s comes first.
  stopSeconds = 150;

  # What would run code as the host as its node starts, so a host's unit
  # clears each and a realm giving one is refused: NODE_OPTIONS, whose
  # --require or --import runs a script and whose --inspect opens the
  # inspector; NODE_PATH, where a require that finds nothing beside it looks,
  # as a library's optional require does; NODE_COMPILE_CACHE, whose folder
  # wins over the host's own and whose compiled code node runs as its
  # modules' (host/service/compile-cache.ts); the loader's LD_PRELOAD, LD_AUDIT
  # and LD_LIBRARY_PATH, which load a library; and OpenSSL's OPENSSL_CONF and
  # OPENSSL_MODULES, whose configuration loads a module. mycelium keeps the
  # same list in its src/node/asker.rs, once its lock here carries the change
  # that proves a hosted session by its spawner: it then takes no host's word
  # whose environment gives any of them, but for a blank NODE_OPTIONS. The two
  # lists are kept alike by hand: nothing here reads mycelium's yet. These are
  # node's, the loader's and OpenSSL's doors only. pi's own variables, such
  # as PI_CODING_AGENT_DIR, which moves the extensions it loads, are the
  # account's doors, like HOME: a process of the account can as well write
  # the host's own code where pi-packages keeps it, or a module in node's
  # home folders (~/.node_modules), which no variable names.
  loadsCode = [
    "NODE_OPTIONS"
    "NODE_PATH"
    "NODE_COMPILE_CACHE"
    "LD_PRELOAD"
    "LD_AUDIT"
    "LD_LIBRARY_PATH"
    "OPENSSL_CONF"
    "OPENSSL_MODULES"
  ];

  # What a host's own restart waits out (the harness's host/service/quiet.ts),
  # as the JSON its unit names: each probe's words and its command, which
  # exits 0 while its reason holds and 1 while it doesn't. A % is doubled,
  # so systemd reads none in a probe's path as a specifier.
  restartWaitsOn = lib.replaceStrings [ "%" ] [ "%%" ] (
    builtins.toJSON (
      map (probe: {
        "while" = probe.why;
        run = probe.command;
      }) config.jitsusama.pi.hostRestartWaitsWhile
    )
  );

  kinds = {
    attended = "the pi conversations Joel watches";
    background = "the pi conversations that run in the background";
  };

  # How long systemd gives a conversation's own process to stop, in seconds.
  # It parks one conversation, not many, and is told the same number, so its
  # own limit, three quarters of it, comes first.
  conversationStopSeconds = 60;

  # The command line a host and a conversation's own process share, up to
  # the harness's program: the realm's folders, the host's node with the
  # inspector shut, and its heap. `rest` comes after the program.
  startOf =
    host: heapMb: rest:
    lib.escapeShellArgs (
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
        # Any process of the account may send SIGUSR1, which would open
        # node's inspector and let it run code as the host. mycelium takes
        # the host's word for which conversation each pi-spawner serves
        # only where this is the first option on the host's command line
        # and no option there opens the inspector; once its lock here
        # carries its list, also only where the host's environment gives
        # none of loadsCode, which the unit clears below. It refuses the
        # word of a host started otherwise, and logs why.
        "--disable-sigusr1"
      ]
      # After the flag above, which must come first; asyncContextFrame's
      # description says why a realm sets it.
      ++ lib.optional host.asyncContextFrame "--experimental-async-context-frame"
      ++ [
        # The harness's package.json names no module type, on purpose.
        "--disable-warning=MODULE_TYPELESS_PACKAGE_JSON"
        "--max-old-space-size=${toString heapMb}"
        entry
      ]
      ++ rest
    );

  # The environment a host and a conversation's own process share, quoted,
  # as systemd splits an unquoted entry at its spaces; `own` comes last.
  # `many` is whether the process holds many conversations, as a host does,
  # and so wants the pools sized for them; a conversation's own process
  # keeps the defaults a plain pi has.
  environmentOf =
    realm: host: many: own:
    lib.mapAttrsToList (name: value: ''"${name}=${lib.escape [ "\\" "\"" ] value}"'') (
      lib.optionalAttrs many {
        # The host reads its conversations' mesh folders off its loop, on
        # libuv's pool, which is four threads by default. On 16 November a
        # stalled disk held those reads for up to 17 s, and four stalled
        # reads would leave every other conversation's reads, lookups and
        # hashing waiting behind them. libuv sizes the pool when it first
        # starts, before any of the host's code runs, so it is set here. A
        # machine's own value still wins.
        UV_THREADPOOL_SIZE = "16";
        # pi gives a process two codemode workers, which suits one
        # conversation but is shared here by every conversation the host
        # holds. After the reset of 17 November the background host's
        # forty conversations had one cell's six-minute script and one
        # other holding both, and every other script failed after waiting
        # a minute. A worker starts only for a script, so a bigger limit
        # costs nothing while idle. pi reads it at each script.
        PI_CODEMODE_MAX_WORKERS = "16";
      }
      // lib.optionalAttrs agentProxied {
        # A hosted conversation's commands run with the host's own
        # environment, so they reach the agents' SSH agent, as a fenced
        # command does, rather than whichever agent the user manager was
        # told of before the host started. After the reset of 17 November
        # the hosts started before the session told the manager of any, and
        # every ssh from a hosted conversation was refused.
        SSH_AUTH_SOCK = "%t/pi-agent.sock";
      }
      // host.environment
      // {
        MYCELIUM_REALM = realm;
        # pi reads its own files from here, and a subagent a hosted
        # conversation starts runs this pi too.
        PI_PACKAGE_DIR = host.pi;
        # The host checks it against its pi as it starts.
        PI_HOST_PROOF = "${host.proof}";
      }
      // lib.optionalAttrs (homeRealm != null) { PI_HOME_REALM = homeRealm; }
      // own
    );

  # The user manager names the TPM's agent, Joel's own keys, to everything
  # it starts, and a hosted conversation's commands inherit the host's
  # environment, so on 18 November the background host still named it after
  # SSH_AUTH_SOCK moved to the agents' proxy. Where the proxy runs, agents
  # sign and log in through it; where it doesn't, SSH_AUTH_SOCK is whatever
  # the manager holds. This hides the TPM agent's variable only: its socket
  # is still at a path everyone knows, and what keeps a host's commands from
  # it, once the agent has started with it, is the agent's key gate
  # (modules/ssh-tpm-agent/gate.conf), which refuses agents.slice. The gate
  # catches accidents, not a command set on escaping; the fence is that
  # boundary. Joel's own signing still finds his agent without the variable.
  #
  # loadsCode goes too, as systemd's last step, after every Environment= and
  # EnvironmentFile= a machine adds, so neither those nor the user manager's
  # environment run code as the host before pi does, or open the inspector
  # the flag shuts. A hosted conversation's commands, which inherit the
  # host's environment, start without them too.
  unsetEnvironment = [ "SSH_TPM_AUTH_SOCK" ] ++ loadsCode;

  # The template each realm-kind host starts a conversation's own process
  # from, `pi-conversation-KEY@SLOT.service`, so no conversation's work can
  # hold another's thread, a host's restart stops none of them, and systemd
  # weighs each one's memory alone. On 19 November one conversation held the
  # background host's one thread for 3 s while a list waited on it, and
  # Joel couldn't work. The host writes the slot's file, KEY.d/SLOT.json in
  # its runtime folder, before it starts the instance, and the process
  # listens on KEY.d/SLOT.sock beside it (the harness's
  # host/service/child-units.ts). Its environment is the host's, so nothing
  # secret passes through the host's code or a unit's record. It starts the
  # harness `current` names then, so after an install a host may hold
  # children of another version: the two read each other's slot and socket
  # leniently, passing over what they don't know, and nothing more pins
  # them. Each resume is a new slot, so systemd's start limit never trips;
  # the host retries a child that dies at once only at its conversation's
  # next wake, and bounds no more than that yet. One host's conversations
  # stop together with `systemctl --user stop 'pi-conversation-KEY@*'`,
  # KEY being its REALM-KIND, which parks each one; stop the host first, or
  # it takes them up again at their next wake, and run it before moving the
  # harness back to one without children, whose host would take up what
  # they hold. jobs-sorter, the shaper and cadence know only `pi-host-*`
  # units so far, so a conversation's busy commands stay in its own unit,
  # held back by its memory throttle below, and aren't ranked first while
  # Joel watches it; its uploads aren't yet raised while he types in it;
  # and cadence doesn't see one still running an old pi after a switch.
  conversationOf =
    realm: host: kind:
    let
      key = "${realm}-${kind}";
    in
    {
      name = "pi-conversation-${key}@";
      value = {
        Unit = {
          # No ConditionPathExists, unlike the host's: a running host is what
          # starts one, though nothing stops another process of the account
          # doing so, so a missing program leaves the instance failed, with
          # node's error in the journal, rather than skipped with nothing run
          # or said. The start itself still succeeds, as Type=exec's does
          # once node runs; the host sees the instance gone before it listens.
          Description = "Holds one of ${kinds.${kind}} in the ${realm} realm, in slot %i";
          # A restart would end the conversation's turn, so a switch leaves a
          # running one be; the next conversation it takes up runs the new pi.
          X-SwitchMethod = "keep-old";
          # Each open is a slot of its own, never started again, so an
          # instance that failed is forgotten rather than listed for good.
          # A [Unit] setting: systemd ignores it under [Service].
          CollectMode = "inactive-or-failed";
        };
        Service = {
          # It says nothing to systemd: its host follows it on its socket.
          Type = "exec";
          ExecStart = startOf host host.conversationHeapMb [ "--child" ];
          Environment = environmentOf realm host false {
            PI_HOST_CHILD_KEY = key;
            PI_HOST_CHILD_SOCKET = "%t/pi-host/${key}.d/%i.sock";
            # The stop limit below, which it parks within.
            PI_HOST_STOP_USEC = toString (conversationStopSeconds * 1000000);
          };
          UnsetEnvironment = unsetEnvironment;
          # Its host decides what comes after an ending: a crash's
          # conversation is resumed by the host in a new process, so systemd
          # never restarts one, and no watchdog kills one for a long turn.
          Restart = "no";
          # One of its commands killed for memory leaves the conversation be,
          # as a plain pi outlives its command.
          OOMPolicy = "continue";
          # Parking ends its turn and the tools it runs; what is left is
          # ended with it.
          KillMode = "mixed";
          TimeoutStopSec = "${toString conversationStopSeconds}s";
          # Past this the conversation and its commands are slowed, not
          # killed, so one conversation's build can't take the memory every
          # other conversation runs in. A default, so a machine may move it.
          MemoryHigh = lib.mkDefault "12G";
          # It keeps no core, as its host keeps none (LimitCORE on the host
          # says why).
          LimitCORE = "0";
          Slice = "agents.slice";
        };
      };
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
        # cadence's evening read keep those. A restart asked of systemd whose
        # stop exits failing does reach it, starting again at once, so the
        # notice stays quiet while the unit starts again.
        OnFailure = [ "pi-host-down@%N.service" ];
      };
      Service = {
        # The host says it's ready once its socket listens, which is when a
        # conversation can be sent to it; it says so from a child process.
        Type = "notify";
        NotifyAccess = "all";
        ExecStart = startOf host host.heapMb [ kind ];
        Environment = environmentOf realm host true {
          # The stop limit below, which the host parks within.
          PI_HOST_STOP_USEC = toString (stopSeconds * 1000000);
          # A host restarts itself onto a new pi or harness once it's
          # installed, or for its heap. At 16:38 on 19 November the attended
          # host did so mid-call as its install moved, and every window on it
          # went with it, so it waits until no call is on and nothing a
          # machine adds holds.
          PI_HOST_RESTART_WAITS_ON = restartWaitsOn;
        };
        UnsetEnvironment = unsetEnvironment;
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
        # times over.
        WatchdogSec = "2min";
        # The watchdog's abort is a SIGABRT, which a JavaScript listener can
        # catch: pi bundles signal-exit, which adds one, and node then only
        # queues the signal for a loop that may never turn. At 16:46 on 17
        # November the background host's main thread spun on a CPU for four
        # minutes; the watchdog aborted it at 120 s and systemd waited 150 s
        # more before the kill, since an abort's timeout is the stop's
        # (TimeoutStopSec below) unless set apart. A node without the listener
        # still aborts at once, and it dumps no core (LimitCORE below), so
        # the kill 10 s on loses nothing: the host's hold watcher has written
        # what held the loop by then.
        TimeoutAbortSec = "10s";
        # A host keeps no core. A core holds every hosted conversation's keys
        # and tokens, and writing one held a crashed host down for minutes:
        # on 16 November an out-of-memory abort streamed 12.5 GB for three
        # minutes before the watchdog's kill. The commands a host runs
        # inherit the limit. A native fault is hunted with perf and /proc
        # on the live process instead, which hold no secrets on disk.
        LimitCORE = "0";
        # The host exits 78 when what it was given is refused (its kind, its
        # realm, a pi missing a host patch), which another start would only
        # refuse again, so it waits for a fix rather than retrying forever.
        RestartPreventExitStatus = "78";
        # Parking ends each turn in flight, the tools it runs with it; what is
        # still running after that is ended with it. At 03:30 on 17 November
        # a host with a 2.5 GB heap was still parking 13 conversations when
        # a 60 s stop killed it, its loop busy 35 s of the first 50, so its own
        # 45 s limit on parking came late and never wrote down what it hadn't
        # parked. 150 s lets a busy host reach its own limit and record what
        # was left, and the host gives parking three quarters of it, while a
        # stuck stop still ends. It covers a restart or a switch: at logout,
        # reboot or poweroff the account's manager is given 5 s to stop
        # (modules/systemd), on purpose, so a shutdown stays quick.
        KillMode = "mixed";
        TimeoutStopSec = "${toString stopSeconds}s";
        # A start resumes each conversation in turn. Under load one takes 8 to
        # 9 minutes: at 09:12 on 18 November a 5 minute limit ended a start at
        # 76 of 81 conversations while it still moved. systemd weighs the
        # host's asks for two more minutes (EXTEND_TIMEOUT_USEC) only past this
        # limit, and then ends the start when one isn't renewed in time, which
        # that start's was not. Twenty minutes lets a slow start finish on the
        # limit alone; past it, the start lives only while the host renews its
        # ask every 30 s, so a stuck one still ends. A default, so a machine
        # may give its hosts longer.
        TimeoutStartSec = lib.mkDefault "20min";
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
              built for; the host runs on it, given --disable-sigusr1, so it
              must be a node that knows that flag: 22.14 or later in 22, or
              23.7 or later. An older one is refused only as its host starts,
              which then stops and is said to be down.
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
          heapMb = lib.mkOption {
            type = lib.types.ints.positive;
            default = 8192;
            description = ''
              The most JavaScript heap each of the realm's hosts may hold, in MB.
              node's own limit is about 4 GB, and on 18 November the background
              host, holding some sixty conversations, ran out of it and aborted,
              ending every one of their turns. A realm's two hosts share
              agents.slice, which slows past 42% of the machine's memory, so both
              together stay under it: 8 GB each fits a 64 GB machine, and a
              machine with less memory sets less. A host past its limit still
              aborts alone, rather than slowing every agent beside it.
            '';
          };
          conversationHeapMb = lib.mkOption {
            type = lib.types.ints.positive;
            default = 4096;
            description = ''
              The most JavaScript heap each conversation's own process may
              hold, in MB: about node's own default, which a plain pi in a
              terminal runs with. One past it aborts alone, and its host
              resumes the conversation in a new process. These heaps are each
              conversation's own, so unlike heapMb they don't add up to a
              share of agents.slice: the slice's ceiling slows them all
              together once they pass it.
            '';
          };
          asyncContextFrame = lib.mkOption {
            type = lib.types.bool;
            default = false;
            description = ''
              Whether the realm's hosts keep each conversation's async context
              in V8's own frame rather than in promise hooks, which run on
              every await of every conversation a host holds: on node 22.23 a
              million awaits under one AsyncLocalStorage took 420 to 740 ms
              through the hooks and 170 to 200 ms through the frame. It passes
              --experimental-async-context-frame, which only node 22 and 23
              know: node 24 and later keep the frame by default and refuse the
              flag as a bad option. A host on one exits as it starts, is
              started twice more in ten minutes, then left down and said to be.
              Evaluation can't see which node a realm names, so set it only
              for a node 22 or 23, and turn it off with any move to 24.
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
              conversation, not here. It may give none of NODE_OPTIONS,
              NODE_PATH, NODE_COMPILE_CACHE, LD_PRELOAD, LD_AUDIT,
              LD_LIBRARY_PATH, OPENSSL_CONF or OPENSSL_MODULES, which the
              host's unit clears since each could make its node run code:
              evaluation refuses a realm giving one, and a command that needs
              one sets it itself.
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

  options.jitsusama.pi.hostRestartWaitsWhile = lib.mkOption {
    type = lib.types.listOf (
      lib.types.submodule {
        options = {
          command = lib.mkOption {
            # A bare name would be looked up on the host's PATH, which a
            # realm's own folders lead.
            type = lib.types.addCheck (lib.types.nonEmptyListOf lib.types.str) (
              command: lib.hasPrefix "/" (lib.head command)
            );
            description = ''
              A command, its program named by an absolute path, that exits 0
              while a host mustn't restart itself and 1 while it may. Any
              other exit, or none within 15 s, holds the restart too, since it
              couldn't tell.
            '';
          };
          why = lib.mkOption {
            type = lib.types.str;
            description = "Why it holds, as the host's journal says it, such as \"a meeting is near\".";
          };
        };
      }
    );
    default = [ ];
    description = ''
      What else a pi host's own restart, onto a new pi or harness or for
      its heap, waits out, beside a call being on here or on another of the
      person's machines, which comes first. The host looks again each
      minute and says why it waits. Only a heap that has nearly run out
      restarts anyway, and a restart asked of systemd by hand never waits.
    '';
  };

  config.jitsusama.pi.hostRestartWaitsWhile = lib.mkBefore [
    {
      command = [ (lib.getExe (pkgs.callPackage ../calls/package.nix { })) ];
      why = "a call is on";
    }
  ];

  config.assertions =
    lib.mapAttrsToList (
      realm: host:
      let
        refused = proofRefusal realm host;
      in
      {
        assertion = refused == null;
        message = toString refused;
      }
    ) cfg
    # The unit clears loadsCode, so a realm's own would be dropped unseen;
    # it is refused instead, saying why.
    ++ lib.mapAttrsToList (
      realm: host:
      let
        given = lib.filter (name: host.environment ? ${name}) loadsCode;
        one = lib.length given == 1;
        # "A", "A and B", "A, B and C"; nothing where none is given.
        named =
          if lib.length given < 2 then
            lib.concatStrings given
          else
            "${lib.concatStringsSep ", " (lib.init given)} and ${lib.last given}";
      in
      {
        assertion = given == [ ];
        message = "jitsusama.pi.hosts.${realm}.environment gives ${named}, which a pi host's unit clears: ${if one then "it" else "each"} could make the host's node run code nobody gave it. NODE_OPTIONS does so through --require or --import, or by opening the host's inspector with --inspect; NODE_PATH through a module a require finds there; NODE_COMPILE_CACHE through compiled code node reads back from its folder; LD_PRELOAD, LD_AUDIT and LD_LIBRARY_PATH through a library the loader loads; and OPENSSL_CONF and OPENSSL_MODULES through a module OpenSSL loads. ${
          if one then "Drop it." else "Drop each one."
        } A host's node takes no option from a realm but its heap, set with heapMb, and V8's async context frame, set with asyncContextFrame. The conversations' commands share the host's environment, so they get none of these from it: a command that needs one sets it itself.";
      }
    ) cfg;

  config.systemd.user.services = lib.mkIf pkgs.stdenv.hostPlatform.isLinux (
    lib.listToAttrs (
      lib.concatLists (
        lib.mapAttrsToList (
          realm: host:
          map (hostOf realm host) (lib.attrNames kinds)
          ++ map (conversationOf realm host) (lib.attrNames kinds)
        ) cfg
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
