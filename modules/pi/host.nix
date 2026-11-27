# Shared pi hosts: for each realm a machine names in jitsusama.pi.hosts, two
# long-lived processes, one for the conversations Joel watches and one for
# those that run in the background, each holding many pi conversations
# behind a socket of its own. The harness's host program is the process,
# run by the node the host's pi was built with; the pi it hosts carries the
# host's patches (checks.nix, pi-hosted-keeps-turns-its-own), and every pi
# an account starts by hand stays unpatched. Each host may run a
# conversation in a process of its own, from a template unit beside it
# (conversationOf). A machine that names no realm gets no host. These are
# Linux's; a Mac's are launchd daemons (darwin.nix), started the same way
# (host-launch.nix).
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

  launch = import ./host-launch.nix { inherit lib; };
  inherit (launch) kinds loadsCode stopSeconds;
  entry = launch.entry config;
  growthTakenBy = import ./heap-growth-taken { inherit lib pkgs; };
  # The build proving a realm's node takes its conversations' growing
  # percent, which each of its conversations' units names.
  growthTakenOf = realm: host: growthTakenBy realm host.node host.conversationHeapGrowingPercent;

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

  # What a host's own restart waits out, as the JSON its unit names, with
  # each % doubled so systemd reads none in a probe's path as a specifier.
  restartWaitsOn = lib.replaceStrings [ "%" ] [ "%%" ] (launch.restartWaitsOn config);

  # How long systemd gives a conversation's own process to stop, in seconds.
  # It parks one conversation, not many, and is told the same number, so its
  # own limit, three quarters of it, comes first.
  conversationStopSeconds = 60;

  # The command line a host and a conversation's own process share, up to
  # the harness's program: the realm's folders, the host's node with the
  # inspector shut, and its heap, then node's own `flags`. `rest` comes
  # after the program.
  startOf =
    host: heapMb: flags: rest:
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
      ++ [ host.node ]
      ++ launch.words config host heapMb flags rest
    );

  # A conversation's allocator tuning, for one in a unit of its own (so on
  # Linux only): its realm's GLIBC_TUNABLES with glibc's trim threshold
  # fixed at 128 KiB. A realm that names its own threshold, in
  # GLIBC_TUNABLES or as MALLOC_TRIM_THRESHOLD_, gets nothing added, since
  # which of two glibc reads last is not the realm's to choose.
  #
  # 128 KiB is where glibc starts, but it raises the threshold, and the
  # size it maps apart, each time a big block is freed, up to 64 MB and 32
  # MB, so a long conversation keeps what it once grew to. Fixing it gives
  # freed heap back. In four runs of 150 prompts each way on 22 November, a
  # conversation held 71 MB less resident and swapped on average (493 to
  # 422 MB) and 113 MB less at the end (605 to 492 MB), for 0.2% more
  # instructions.
  #
  # Who gets what, where the login is the systemd user manager's
  # environment, which every user unit starts from:
  #
  # - A conversation in its own unit gets the above, and never the login's
  #   GLIBC_TUNABLES or MALLOC_TRIM_THRESHOLD_: the realm's value or the
  #   trim replaces each, and loginAllocator drops whichever neither sets.
  #   glibc's other MALLOC_ variables, such as MALLOC_ARENA_MAX,
  #   still come from the realm or the login.
  # - Its commands and subagents inherit that environment. From the
  #   harness's pull request 2015 on, they get it without GLIBC_TUNABLES,
  #   as a fixed trim would pin their mmap threshold too: the harness
  #   lists it in HOST_ONLY (host/service/program.ts), which no check here
  #   reads, and an older harness passes it on. So a trim threshold meant
  #   for them goes in the realm as MALLOC_TRIM_THRESHOLD_, which also
  #   replaces the trim added above, or in the command, and any other
  #   tunable in the command.
  # - The hosts get no trim: they keep the login's allocator variables,
  #   each one the realm names replaced by the realm's. A conversation a
  #   host holds in its own process, and that conversation's commands,
  #   have the host's environment. So the conversation keeps glibc's
  #   growing thresholds unless the realm or the login fixes one of the
  #   four that stop the growth (the trim threshold, top pad, mmap
  #   threshold or mmap count), and its commands get the login's
  #   MALLOC_TRIM_THRESHOLD_ where the realm names none, and before pull
  #   request 2015 the host's GLIBC_TUNABLES too.
  trimmedTunables =
    host:
    let
      trim = "glibc.malloc.trim_threshold=131072";
      own = host.environment.GLIBC_TUNABLES or "";
      named =
        host.environment ? MALLOC_TRIM_THRESHOLD_ || lib.hasInfix "glibc.malloc.trim_threshold=" own;
    in
    lib.optionalAttrs (!named) {
      GLIBC_TUNABLES = if own == "" then trim else "${own}:${trim}";
    };

  # The login's allocator variables a conversation's own unit drops: its
  # GLIBC_TUNABLES and MALLOC_TRIM_THRESHOLD_, each unless the realm or the
  # trim above sets it. So the threshold comes only from the realm and the
  # trim, and no login value sits beside the added one with glibc's read
  # order deciding between them. The conversation's commands inherit the
  # drop. systemd applies UnsetEnvironment after every Environment=, so a
  # name the realm or the trim sets stays off the list, or its own value
  # would go too, and a name on it is dropped even when another module sets
  # it on the unit.
  #
  # The cost: replacing or dropping the login's GLIBC_TUNABLES loses every
  # tunable it names, not only the allocator's, so a conversation loses any
  # hardening tunable the login sets. To keep one, name it in the realm's
  # GLIBC_TUNABLES. That reaches the conversation's own process, and from the
  # harness's pull request 2015 on not its commands, which an older harness
  # passes it on to. It also replaces the login's whole GLIBC_TUNABLES in both
  # of the realm's hosts, so a login tunable the hosts need is named there too.
  loginAllocator =
    host:
    let
      set = host.environment // trimmedTunables host;
    in
    lib.filter (name: !(set ? ${name})) [
      "GLIBC_TUNABLES"
      "MALLOC_TRIM_THRESHOLD_"
    ];

  # A value with each % doubled, as systemd reads a % in an Environment=
  # entry as a specifier.
  doubledPercent = lib.replaceStrings [ "%" ] [ "%%" ];

  # A realm's environment taken as written: a proxy's URL may hold %40,
  # which systemd would otherwise refuse or expand rather than pass on.
  # What this module adds itself, such as SSH_AUTH_SOCK's %t, is a
  # specifier on purpose and is left as it is. The account's CHROME_PATH,
  # a path set for its shell rather than for systemd, is doubled where it
  # is added.
  realmTakenAsWritten =
    host: host // { environment = lib.mapAttrs (_: doubledPercent) host.environment; };

  # The environment a host and a conversation's own process share, quoted,
  # as systemd splits an unquoted entry at its spaces; host-launch.nix says
  # what `many` and `own` are.
  environmentOf =
    realm: host: many: own:
    lib.mapAttrsToList (name: value: ''"${name}=${lib.escape [ "\\" "\"" ] value}"'') (
      launch.environment config realm (realmTakenAsWritten host) many (
        lib.optionalAttrs agentProxied {
          # A hosted conversation's commands run with the host's own
          # environment, so they reach the agents' SSH agent, as a fenced
          # command does, rather than whichever agent the user manager was
          # told of before the host started. After the reset of 17 November
          # the hosts started before the session told the manager of any,
          # and every ssh from a hosted conversation was refused.
          SSH_AUTH_SOCK = "%t/pi-agent.sock";
        }
        // {
          # The harness's browser tool starts each browser on a profile of its
          # own, made fresh. On Linux a fresh Google Chrome profile enrols
          # itself as a new browser with whatever cloud management a policy
          # in /etc/opt/chrome names, and takes that policy's forced
          # extensions, so every agent's look at a page added one more
          # browser to an organisation's books. Chromium reads only
          # /etc/chromium's policies. A machine names its own browser once,
          # in the account's CHROME_PATH (home.nix sets Chromium as its
          # default), and its hosts follow it; a realm's environment still
          # wins over both.
          CHROME_PATH = doubledPercent (
            config.home.sessionVariables.CHROME_PATH or (lib.getExe pkgs.chromium)
          );
        }
      ) own
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
  # they hold. jobs-sorter ranks each conversation's unit as a host's, by
  # the windows showing it (jobs-sorter/src/hosts.rs), but moves busy
  # commands out of `pi-host-*` units only, and the shaper and cadence
  # know only those. So a conversation's busy commands stay in its own
  # unit, held back by its memory throttle below; its uploads aren't yet
  # raised while he types in it; and cadence doesn't see one still
  # running an old pi after a switch.
  conversationOf =
    realm: host: kind:
    let
      key = "${realm}-${kind}";
      growthTaken = growthTakenOf realm host;
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
        }
        // lib.optionalAttrs (host.conversationHeapGrowingPercent != null) {
          # Unread by systemd: it ties the generation to a build that runs the
          # realm's node with the percent, so a node that refuses it fails the
          # switch by name rather than ending each conversation as it starts.
          X-HeapGrowthTakenBy = "${growthTaken}";
        };
        Service = {
          # It says nothing to systemd: its host follows it on its socket.
          Type = "exec";
          ExecStart = startOf host host.conversationHeapMb (lib.optional (
            host.conversationHeapGrowingPercent != null
          ) "--heap-growing-percent=${toString host.conversationHeapGrowingPercent}") [ "--child" ];
          Environment = environmentOf realm host false (
            {
              PI_HOST_CHILD_KEY = key;
              PI_HOST_CHILD_SOCKET = "%t/pi-host/${key}.d/%i.sock";
              # The stop limit below, which it parks within.
              PI_HOST_STOP_USEC = toString (conversationStopSeconds * 1000000);
            }
            // trimmedTunables (realmTakenAsWritten host)
          );
          UnsetEnvironment = unsetEnvironment ++ loginAllocator host;
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
        ExecStart = startOf host host.heapMb [ ] [ kind ];
        # The stop limit below, which the host parks within, and what its
        # restart waits on (host-launch.nix's hostOwn).
        Environment = environmentOf realm host true (launch.hostOwn restartWaitsOn);
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
        # still aborts at once, and systemd-coredump keeps no core of it
        # (LimitCORE below), so the kill 10 s on loses nothing: the host's
        # hold watcher has written what held the loop by then.
        TimeoutAbortSec = "10s";
        # A host keeps no core. A core holds every hosted conversation's keys
        # and tokens, and writing one held a crashed host down for minutes:
        # on 16 November an out-of-memory abort streamed 12.5 GB for three
        # minutes before the watchdog's kill. The commands a host runs
        # inherit the limit, and its hard limit is 0 too, so none can raise
        # it. A native fault then leaves no stack trace in the journal either:
        # it is hunted on the live process with /proc and perf, whose call
        # graphs copy raw stack into perf.data, so that file holds the host's
        # secrets too: nothing removes it, so whoever records one deletes it
        # once read. A host started before a switch keeps its old limit until
        # its next start.
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
              which then stops and is said to be down. It must be node's own
              program, not a script that runs it: a Mac runs a copy of it
              named pi-host, and refuses a script, which would run node
              under node's name.
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
          conversationHeapGrowingPercent = lib.mkOption {
            type = lib.types.nullOr (lib.types.ints.between 10 100);
            default = null;
            description = ''
              How far each conversation's own process lets its heap grow past
              what it held live after a full collection before it collects in
              full again, in percent, as node's --heap-growing-percent: V8 then
              grows each limit by one plus the percent over a hundred, in place
              of the factor it would pick. It still grows a limit by at least a
              few megabytes past what is live, adds room for the young
              generation and stops halfway from what is live to the heap
              limit, so a small heap grows by more than the percent. Unset,
              with a heap limit of 2048 MB or more, as conversationHeapMb's
              default is, V8 lets the heap reach up to four times what is live
              while collections keep up, and carries that growth as resident
              memory between collections, but grows by a tenth after a
              collection that gives memory back. Set, the percent stands in for
              both. A lower percent collects more often, keeping less resident
              for more collection time; it starts at 10, the tenth V8 itself
              grows by at its lowest, and stops at 100, since more would grow
              past V8's own choice more often. It reaches only conversations in
              a unit of their own, on Linux: hosts, and a Mac's pi, keep V8's
              own growth. A node that doesn't know the flag refuses it as a bad
              option, which would end each conversation as it starts, so the
              generation builds the realm's node running with it first, and a
              node that refuses it, fails another way or doesn't end within a
              minute fails the switch by name. So the realm's node must be a
              program its build can run, from the store and with its context,
              as lib.getExe gives one. A machine built on another platform,
              whose builder can't run its programs, goes without that proof,
              and warns of it unless conversationHeapGrowthVouchedNode names
              that node.
            '';
          };
          conversationHeapGrowthVouchedNode = lib.mkOption {
            type = lib.types.nullOr lib.types.path;
            default = null;
            description = ''
              The node known to take conversationHeapGrowingPercent, which a
              machine built on another platform may run unproven, saying
              nothing, while it is the realm's node. Its builder can't run the
              node, so the machine warns, which a check that a machine
              evaluates without warnings refuses. Name the node once one built
              the same way on the machine's own platform shows it takes the
              percent. Given another node, the realm warns again, so a node
              changed later doesn't go unproven unsaid. A machine built on its
              own platform is proven either way.
            '';
          };
          conversationHeapGrowthMayGoUnproven = lib.mkOption {
            type = lib.types.bool;
            default = false;
            visible = false;
            description = "Gone, but kept so a machine setting it is told to name the node in conversationHeapGrowthVouchedNode.";
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
              one sets it itself. Each value is taken as written: a % in it
              stays a %, never read as one of systemd's specifiers, so a
              value meaning a folder such as %h or %t spells out its path.

              On Linux, each conversation in a unit of its own is given
              GLIBC_TUNABLES with glibc.malloc.trim_threshold=131072 added
              to the realm's own value, so it hands freed heap back. A realm
              that names its own threshold, in GLIBC_TUNABLES or
              MALLOC_TRIM_THRESHOLD_, keeps it alone; a large one, such as
              4294967295, turns the trim off. Any fixed trim threshold also
              fixes the size glibc maps apart, at 128 KiB unless
              glibc.malloc.mmap_threshold or MALLOC_MMAP_THRESHOLD_ names
              another.

              Such a conversation never reads the login's GLIBC_TUNABLES or
              MALLOC_TRIM_THRESHOLD_, where the login is the systemd user
              manager's environment: each is replaced by the realm's value or
              the trim, or dropped. A dropped one stays dropped even when
              another module sets it on the unit; a replaced one holds
              only where no other module sets it there too. This loses every
              tunable the login's GLIBC_TUNABLES names, a hardening one
              included. To keep one, name it in the realm's GLIBC_TUNABLES.
              That reaches the conversation's own process, and from the
              harness's pull request 2015 on not its commands, which an older
              harness passes it on to. It also replaces the login's whole
              GLIBC_TUNABLES in both of the realm's hosts, so a login tunable
              the hosts need is named there too. glibc's other MALLOC_
              variables, such as MALLOC_ARENA_MAX, still come from the realm
              or the login.

              From the harness's pull request 2015 on, no hosted command gets
              GLIBC_TUNABLES; one that needs a tunable sets it itself. The
              commands of a conversation in its own unit get the rest of its
              environment, the realm's MALLOC_TRIM_THRESHOLD_ among it.

              The hosts get no trim: they keep the login's allocator
              variables, each one the realm names replaced by the realm's. A
              conversation a host holds in its own process, and that
              conversation's commands, have the host's environment, so they
              get the login's MALLOC_TRIM_THRESHOLD_ where the realm names
              none. Before pull request 2015, those commands get the host's
              GLIBC_TUNABLES too.
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
        } A host's node takes no option from a realm but its heap, set with heapMb, and V8's async context frame, set with asyncContextFrame; a conversation's own node takes its heap and its growth too, set with conversationHeapMb and conversationHeapGrowingPercent. The conversations' commands share the host's environment, so they get none of these from it: a command that needs one sets it itself.";
      }
    ) cfg
    ++ lib.mapAttrsToList (realm: host: {
      assertion = !host.conversationHeapGrowthMayGoUnproven;
      message = "jitsusama.pi.hosts.${realm}.conversationHeapGrowthMayGoUnproven is gone: name the node known to take conversationHeapGrowingPercent in conversationHeapGrowthVouchedNode instead, so a node changed later warns again.";
    }) cfg;

  # A realm whose proof stands unproven, as on a machine built on another
  # platform, says so here, since its switch can't.
  config.warnings = lib.mkIf pkgs.stdenv.hostPlatform.isLinux (
    lib.concatLists (
      lib.mapAttrsToList (
        realm: host:
        let
          growthTaken = growthTakenOf realm host;
        in
        lib.optional (
          host.conversationHeapGrowingPercent != null
          && growthTaken ? unproven
          && toString host.conversationHeapGrowthVouchedNode != toString host.node
        ) growthTaken.unproven
      ) cfg
    )
  );

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
