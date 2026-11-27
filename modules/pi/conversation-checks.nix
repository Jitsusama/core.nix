# The check of the template each realm-kind host starts its conversations'
# own processes from, which host-checks.nix gathers on Linux: kept apart from
# the hosts' checks since it changes with the conversation unit alone.
{
  account,
  bareDarwin,
  bareNixos,
  holds,
  lib,
  loadsCode,
  pkgs,
  proofOf,
  self,
}:
let
  # Pi hosts for these realms, each given an empty pi and the node `nodeOf`
  # names for it, and its percent where it has one.
  growingHostsOf =
    nodeOf: hosts:
    let
      pi = "${pkgs.emptyDirectory}/pi";
      proof = proofOf { runtime = builtins.unsafeDiscardStringContext "${pkgs.emptyDirectory}"; };
    in
    lib.mapAttrs (
      realm: percent:
      {
        inherit pi proof;
        node = nodeOf realm;
      }
      // lib.optionalAttrs (percent != null) {
        conversationHeapGrowingPercent = percent;
      }
    ) hosts;
  # The account of a Linux workstation whose account names these pi hosts.
  # The traced realm's node is node 22, which its proof runs, so its
  # rendered units can be built; every other realm's is empty, so its proof
  # is only named, never built.
  growingAccountOf =
    hosts:
    account (bareNixos [
      self.nixosModules.workstation
      {
        home-manager.users.someone.jitsusama.pi.hosts = growingHostsOf (
          realm: if realm == "traced" then lib.getExe pkgs.nodejs_22 else "${pkgs.emptyDirectory}/node"
        ) hosts;
      }
    ]);
  growingServicesOf = hosts: (growingAccountOf hosts).systemd.user.services;
  # One workstation for every percent that is taken, so one evaluation serves
  # them all: a percent that is refused fails the whole evaluation, so only
  # those need one of their own.
  grown = growingAccountOf {
    set = 30;
    plain = null;
    least = 10;
    most = 100;
    traced = 70;
  };
  growing = grown.systemd.user.services;
  # The command a unit starts with, or nothing where there's no such unit.
  startOf = services: unit: lib.head (lib.toList (services.${unit}.Service.ExecStart or ""));
  growthTakenBy = import ./heap-growth-taken { inherit lib pkgs; };
in
{
  # Each realm-kind host gets a template its conversations' own processes
  # start from, which its host's code names: run by the host's program as a
  # child of the host's key, listening in the slot systemd names, with the
  # host's environment and its inspector shut, among the agents, never
  # restarted by systemd or watched, forgotten once ended, and left be by a
  # switch, a host's restart or a command the kernel kills for memory.
  pi-conversations-run-apart =
    let
      pi = "${pkgs.emptyDirectory}/pi";
      node = "${pkgs.emptyDirectory}/node";
      proof = proofOf { runtime = builtins.unsafeDiscardStringContext "${pkgs.emptyDirectory}"; };
      services =
        (account (bareNixos [
          self.nixosModules.workstation
          {
            home-manager.users.someone.jitsusama.pi = {
              hosts.one = {
                inherit pi node proof;
                conversationHeapMb = 3072;
                asyncContextFrame = true;
                # A % reaches the conversation as written.
                environment.PROXY_ORIGIN = "https://proxy.example.com/a%40b";
              };
              # A realm that tunes its own allocator, one that names its
              # own trim threshold beside other tuning, one that names it in
              # glibc's older variable, and one that names it there beside
              # other tuning.
              hosts.tuned = {
                inherit pi node proof;
                environment.GLIBC_TUNABLES = "glibc.malloc.arena_max=2";
              };
              hosts.trimmed = {
                inherit pi node proof;
                environment.GLIBC_TUNABLES = "glibc.malloc.arena_max=2:glibc.malloc.trim_threshold=65536";
              };
              hosts.aliased = {
                inherit pi node proof;
                environment.MALLOC_TRIM_THRESHOLD_ = "65536";
              };
              hosts.both = {
                inherit pi node proof;
                environment.GLIBC_TUNABLES = "glibc.malloc.arena_max=2";
                environment.MALLOC_TRIM_THRESHOLD_ = "65536";
              };
              # Where pi's SSH agent proxy runs.
              agentProxy.builderKey = "/builders/key";
            };
            # A machine names its own browser once, for the account, and its
            # hosts follow it rather than Chromium.
            home-manager.users.someone.home.sessionVariables.CHROME_PATH = "/own/100%/chrome";
          }
        ])).systemd.user.services;
      mac =
        (account (bareDarwin [
          self.darwinModules.workstation
          {
            home-manager.users.someone.jitsusama.pi.hosts.one = { inherit pi node proof; };
          }
        ])).systemd.user.services;
      first = value: lib.head (lib.toList value);
      tunablesOf =
        unit:
        lib.filter (lib.hasPrefix ''"GLIBC_TUNABLES='') (
          lib.toList (services.${unit}.Service.Environment or [ ])
        );
      # A realm's own tuning reaches its hosts, and its conversations with
      # the trim threshold beside it unless it names its own.
      keepsTuning =
        realm: own: conversation: kind:
        tunablesOf "pi-host-${realm}-${kind}" == [ ''"GLIBC_TUNABLES=${own}"'' ]
        && tunablesOf "pi-conversation-${realm}-${kind}@" == [ ''"GLIBC_TUNABLES=${conversation}"'' ]
        && loginAllocatorOf "pi-conversation-${realm}-${kind}@" == [ "MALLOC_TRIM_THRESHOLD_" ];
      # Which of the allocator variables a unit keeps from the login.
      loginAllocatorOf =
        unit:
        lib.filter (
          name:
          lib.elem name [
            "GLIBC_TUNABLES"
            "MALLOC_TRIM_THRESHOLD_"
          ]
        ) (lib.toList (services.${unit}.Service.UnsetEnvironment or [ ]));
      # A realm naming its threshold only in glibc's older variable keeps it
      # alone, given no GLIBC_TUNABLES and kept from the login's.
      keepsAlias =
        kind:
        let
          unit = "pi-conversation-aliased-${kind}@";
        in
        services ? ${unit}
        && lib.elem ''"MALLOC_TRIM_THRESHOLD_=65536"'' (
          lib.toList (services.${unit}.Service.Environment or [ ])
        )
        && tunablesOf unit == [ ]
        && loginAllocatorOf unit == [ "GLIBC_TUNABLES" ];
      # One naming it there beside other tuning keeps both as given, with no
      # threshold added and nothing kept from the login, since it sets both.
      keepsBoth =
        kind:
        let
          unit = "pi-conversation-both-${kind}@";
        in
        services ? ${unit}
        && lib.elem ''"MALLOC_TRIM_THRESHOLD_=65536"'' (
          lib.toList (services.${unit}.Service.Environment or [ ])
        )
        && tunablesOf unit == [ ''"GLIBC_TUNABLES=glibc.malloc.arena_max=2"'' ]
        && loginAllocatorOf unit == [ ];
      runsApart =
        kind:
        let
          unit = services."pi-conversation-one-${kind}@" or { Service = { }; };
          service = unit.Service;
          execStart = first (service.ExecStart or "");
          environment = lib.toList (service.Environment or [ ]);
        in
        service.Type or null == "exec"
        && service.Slice or null == "agents.slice"
        && service.Restart or null == "no"
        && !(service ? WatchdogSec)
        # Read only from [Unit], where systemd looks for it.
        && unit.Unit.CollectMode or null == "inactive-or-failed"
        && !(service ? CollectMode)
        && service.OOMPolicy or null == "continue"
        && service.KillMode or null == "mixed"
        && service.TimeoutStopSec or null == "60s"
        && service.MemoryHigh or null == "12G"
        && service.LimitCORE or null == "0"
        && unit.Unit.X-SwitchMethod or null == "keep-old"
        # A missing program fails the instance rather than skipping it.
        && !(unit.Unit ? ConditionPathExists)
        # Started by its host alone, and stopped with nothing else.
        && !(unit ? Install)
        && !(unit.Unit ? PartOf)
        && !(unit.Unit ? BindsTo)
        && !(unit.Unit ? Requires)
        && !(unit.Unit ? Requisite)
        && lib.hasPrefix "${node} --disable-sigusr1 --experimental-async-context-frame " execStart
        && !(lib.hasInfix "--inspect" (lib.replaceStrings [ "_" ] [ "-" ] execStart))
        && lib.hasInfix " '--max-old-space-size=3072' /" execStart
        && lib.hasSuffix "/scripts/pi-host.ts --child" execStart
        &&
          lib.sort lib.lessThan (lib.toList (service.UnsetEnvironment or [ ])) == lib.sort lib.lessThan (
            [
              "SSH_TPM_AUTH_SOCK"
              # The login's older trim variable, which its realm doesn't name.
              "MALLOC_TRIM_THRESHOLD_"
            ]
            ++ loadsCode
          )
        # The names host/service/child.ts reads, and the slot
        # child-units.ts writes beside the socket.
        && lib.elem ''"PI_HOST_CHILD_KEY=one-${kind}"'' environment
        && lib.elem ''"PI_HOST_CHILD_SOCKET=%t/pi-host/one-${kind}.d/%i.sock"'' environment
        && lib.elem ''"PI_HOST_STOP_USEC=60000000"'' environment
        # It hands freed heap back as it goes.
        && lib.elem ''"GLIBC_TUNABLES=glibc.malloc.trim_threshold=131072"'' environment
        && lib.elem ''"MYCELIUM_REALM=one"'' environment
        && lib.elem ''"PI_PACKAGE_DIR=${pi}"'' environment
        && lib.elem ''"PI_HOST_PROOF=${proof}"'' environment
        && lib.elem ''"SSH_AUTH_SOCK=%t/pi-agent.sock"'' environment
        # systemd takes a doubled % as one.
        && lib.elem ''"PROXY_ORIGIN=https://proxy.example.com/a%%40b"'' environment
        && lib.elem ''"CHROME_PATH=/own/100%%/chrome"'' environment
        # Its host's unit too, so a split of the two can't leave one undoubled.
        && lib.elem ''"CHROME_PATH=/own/100%%/chrome"'' (
          lib.toList (services."pi-host-one-${kind}".Service.Environment or [ ])
        )
        # One conversation keeps a plain pi's pools, not a host's.
        && !(lib.any (lib.hasPrefix ''"PI_CODEMODE_MAX_WORKERS='') environment)
        && !(lib.any (lib.hasPrefix ''"UV_THREADPOOL_SIZE='') environment);
    in
    holds "pi-conversations-run-apart"
      "A realm-kind pi host has no template for its conversations' own processes named as the harness names it, or one runs other than the host's program as a child of its key, listens outside its slot, misses the host's environment or proof, browses in Chromium where its machine names its own browser, leaves its inspector open or the user manager's code-loading variables in place, runs outside agents.slice, is restarted or watched by systemd, is left listed once failed, is stopped by a switch, its host's restart or one of its commands killed for memory, isn't given 60 s to park or told so, has no memory throttle, holds on to freed heap, loses its realm's own allocator tuning or reads the login's GLIBC_TUNABLES or MALLOC_TRIM_THRESHOLD_ where neither the realm nor the trim sets it, unsets more than those, the TPM agent's variable and the code-loading variables, misses its realm's environment or its machine's browser as written, can dump a core, is skipped silently with no program, misses its realm's heap, frame flag or the agents' SSH agent, takes a host's pools, or a Mac gets one."
      (
        runsApart "attended"
        && runsApart "background"
        # A realm's own allocator tuning is kept, said once.
        &&
          lib.all
            (
              kind:
              keepsTuning "tuned" "glibc.malloc.arena_max=2"
                "glibc.malloc.arena_max=2:glibc.malloc.trim_threshold=131072"
                kind
              &&
                keepsTuning "trimmed" "glibc.malloc.arena_max=2:glibc.malloc.trim_threshold=65536"
                  "glibc.malloc.arena_max=2:glibc.malloc.trim_threshold=65536"
                  kind
              && keepsAlias kind
              && keepsBoth kind
            )
            [
              "attended"
              "background"
            ]
        && lib.filter (lib.hasPrefix "pi-conversation-") (lib.attrNames mac) == [ ]
      );

  # A realm may set how far each of its conversations' heaps grows past what
  # it holds live before V8 collects it in full. Its hosts, and every process
  # of a realm that names no percent, keep V8's own growth, and a percent
  # outside 10 to 100 is refused, while both ends are taken. Only a
  # conversation given a percent names the build proving its own realm's node
  # takes that percent, and the name carries the build into the generation.
  pi-conversations-set-heap-growth =
    let
      # The build that proves the set realm's node, which growingServicesOf
      # gives every host, takes its 30 percent.
      proof = "${growthTakenBy "set" "${pkgs.emptyDirectory}/node" 30}";
      set =
        kind:
        let
          start = startOf growing "pi-conversation-set-${kind}@";
          proven = growing."pi-conversation-set-${kind}@".Unit.X-HeapGrowthTakenBy or "";
        in
        lib.hasInfix " '--max-old-space-size=4096' '--heap-growing-percent=30' /" start
        && lib.hasSuffix "/scripts/pi-host.ts --child" start
        && proven == proof
        && builtins.getContext proven == builtins.getContext proof;
      # Whether a unit is there, its command leaves V8 its own growth and it
      # names no build proving a percent.
      keepsOwnGrowth =
        unit:
        growing ? ${unit}
        && !lib.hasInfix "--heap-growing-percent" (startOf growing unit)
        && !(growing.${unit}.Unit ? X-HeapGrowthTakenBy);
      # Taken when the realm's conversation holds the percent's word.
      taken =
        realm: percent:
        lib.hasInfix " '--heap-growing-percent=${toString percent}' /" (
          startOf growing "pi-conversation-${realm}-attended@"
        );
      # Refused when a realm naming it doesn't evaluate at all: each is a
      # whole workstation's evaluation of its own, and one in range on the
      # same path evaluating shows a refusal comes from the range.
      refused =
        percent:
        let
          start = startOf (growingServicesOf { one = percent; }) "pi-conversation-one-attended@";
        in
        !(builtins.tryEval (builtins.deepSeq start start)).success;
    in
    holds "pi-conversations-set-heap-growth"
      "A realm's percent of heap growth for its conversations doesn't reach their own processes as V8's growing percent, before the harness's program, with the build proving its own node takes that percent and its context, or it reaches its hosts or a realm that names none, or 10 or 100 percent isn't taken, or 9 or 101 isn't refused where 50 in its place is taken."
      (
        lib.all
          (
            kind:
            set kind
            && keepsOwnGrowth "pi-host-set-${kind}"
            && keepsOwnGrowth "pi-conversation-plain-${kind}@"
            && keepsOwnGrowth "pi-host-plain-${kind}"
          )
          [
            "attended"
            "background"
          ]
        && taken "least" 10
        && taken "most" 100
        && refused 9
        && refused 101
        && !(refused 50)
      );

  # A realm whose node refuses the growing percent fails the generation's
  # build, naming the realm, the percent and the node, rather than ending
  # each conversation as it starts, as node 22 and this tree's own node
  # refuse an option they don't know; one whose node fails another way,
  # exits 124 itself, never ends, ignores being told to end, is killed
  # before its limit, or can't be run, a package's folder among them, fails
  # saying so and what to do; node 22, which pi is built with, takes it, and
  # the unit file home-manager renders names that proof; and a machine
  # cross-built for another platform has the proof stand unproven and warns
  # of it unless its realm vouches for that very node, where a machine built
  # on its own doesn't.
  pi-conversations-refuse-a-node-without-growth =
    let
      given = "jitsusama.pi.hosts.one.conversationHeapGrowingPercent is 70, ";
      # Refuses the word as node refuses an option it doesn't know, and runs
      # anything else, so only a build that gives it the word fails.
      refusing = pkgs.writeShellScript "node" ''
        if [ "$1" = --heap-growing-percent=70 ]; then
          echo "$0: bad option: $1" >&2
          exit 9
        fi
      '';
      refused = pkgs.testers.testBuildFailure (growthTakenBy "one" "${refusing}" 70);
      refusedSaid = lib.concatStrings [
        given
        "but ${refusing} refuses --heap-growing-percent=70, so each of the realm's conversations "
        "would end as it starts: unset the percent, or give the realm a node that knows it."
      ];
      # Fails with the word, as a node that can't start does, but not as
      # node refuses an option.
      failing = pkgs.writeShellScript "node" ''
        if [ "$1" = --heap-growing-percent=70 ]; then
          echo "$0: can't start" >&2
          exit 3
        fi
      '';
      failed = pkgs.testers.testBuildFailure (growthTakenBy "one" "${failing}" 70);
      failedSaid = lib.concatStrings [
        given
        "but ${failing} failed with --heap-growing-percent=70 other than by refusing it, so the "
        "build can't prove it takes the word: give the realm a node its build can run, or unset "
        "the percent."
      ];
      # Exits 124 itself once given the word, as timeout does when it ends a
      # node, and says what timeout says as it does, but without timeout
      # sending a signal, so it fails another way.
      quitting = pkgs.writeShellScript "node" ''
        if [ "$1" = --heap-growing-percent=70 ]; then
          echo "$0: quitting" >&2
          echo "timeout: sending signal TERM to command '$0'" >&2
          exit 124
        fi
      '';
      quit = pkgs.testers.testBuildFailure (growthTakenBy "one" "${quitting}" 70);
      quitSaid = lib.concatStrings [
        given
        "but ${quitting} failed with --heap-growing-percent=70 other than by refusing it, so the "
        "build can't prove it takes the word"
      ];
      oneSecond = import ./heap-growth-taken {
        inherit lib pkgs;
        seconds = 1;
      } "one";
      # Never ends once given the word, as a node that hangs as it starts
      # doesn't, given a second to.
      hanging = pkgs.writeShellScript "node" ''
        if [ "$1" = --heap-growing-percent=70 ]; then
          echo "$0: still starting" >&2
          exec sleep 600
        fi
      '';
      slow = pkgs.testers.testBuildFailure (oneSecond "${hanging}" 70);
      slowSaid =
        node:
        lib.concatStrings [
          given
          "but ${node} didn't end within its 1-second limit once run with "
          "--heap-growing-percent=70, so the build can't prove it takes the word: give the realm "
          "a node that starts and ends, or unset the percent."
        ];
      # Ignores being told to end once given the word, so timeout kills it
      # five seconds past its limit and exits 137. It stops listening
      # before it speaks, so a builder slow to reach the echo can't have it
      # ended by the TERM, exiting 124 as a hanging node does.
      deaf = pkgs.writeShellScript "node" ''
        if [ "$1" = --heap-growing-percent=70 ]; then
          trap "" TERM
          echo "$0: not listening" >&2
          exec sleep 600
        fi
      '';
      outlasted = pkgs.testers.testBuildFailure (oneSecond "${deaf}" 70);
      # Killed at once once given the word, as the kernel kills a process
      # for memory, which exits 137 as timeout's own kill does, but with no
      # signal from timeout. Given the minute a realm's node is, so a loaded
      # builder can't make it outlive its limit.
      dying = pkgs.writeShellScript "node" ''
        if [ "$1" = --heap-growing-percent=70 ]; then
          echo "$0: out of memory" >&2
          kill -KILL $$
        fi
      '';
      killed = pkgs.testers.testBuildFailure (growthTakenBy "one" "${dying}" 70);
      killedSaid = lib.concatStrings [
        given
        "but ${dying} was killed before its 60-second limit once run with "
        "--heap-growing-percent=70, as the kernel kills a process for memory or another process "
        "can, so the build can't prove it takes the word: build again once memory is free."
      ];
      # Each a node the build can't run: one named outside the store, as no
      # build's sandbox holds, and a package's folder, named with its
      # context, in place of its program.
      unrunSaid =
        node:
        lib.concatStrings [
          given
          "but the build proving the realm's node takes it can't run ${node}: give the realm a "
          "node its build can run, a program from the store named with its context, as "
          "lib.getExe gives one, or unset the percent."
        ];
      unrun = pkgs.testers.testBuildFailure (growthTakenBy "one" "/nowhere/node" 70);
      folder = pkgs.testers.testBuildFailure (growthTakenBy "one" "${pkgs.nodejs_22}" 70);
      taken = growthTakenBy "one" (lib.getExe pkgs.nodejs_22) 70;
      # The proof the traced realm's units name, whose node is node 22, and
      # the unit files home-manager renders for them.
      tracedProof = growthTakenBy "traced" (lib.getExe pkgs.nodejs_22) 70;
      renderedOf =
        kind: grown.xdg.configFile."systemd/user/pi-conversation-traced-${kind}@.service".source;
      # A workstation built here for an aarch64 machine, whose node its
      # builder can't run: realm one vouches for its own node, realm two for
      # another, as a realm does whose node changed after it vouched, and
      # realm three for none. Only evaluated, never built.
      crossMachine = bareNixos [
        self.nixosModules.workstation
        (
          { pkgs, ... }:
          {
            nixpkgs.buildPlatform = "x86_64-linux";
            nixpkgs.hostPlatform = "aarch64-linux";
            home-manager.users.someone.jitsusama.pi.hosts =
              lib.recursiveUpdate
                (growingHostsOf (_: lib.getExe pkgs.nodejs_22) {
                  one = 70;
                  two = 70;
                  three = 70;
                })
                {
                  one.conversationHeapGrowthVouchedNode = lib.getExe pkgs.nodejs_22;
                  two.conversationHeapGrowthVouchedNode = "/nowhere/node";
                };
          }
        )
      ];
      crossNode = lib.getExe crossMachine.pkgs.nodejs_22;
      unprovenSaidOf =
        realm:
        lib.concatStrings [
          "jitsusama.pi.hosts.${realm}.conversationHeapGrowingPercent is 70, "
          "but this machine is built on another platform, whose builder can't run ${crossNode}, so "
          "its switch doesn't prove the node takes --heap-growing-percent=70"
        ];
      # The proof the cross-built machine's units name stands unproven,
      # running no node, and says which option vouches for a node. Its
      # account warns of the realms vouching for another node or for none,
      # which home-manager carries into the machine's warnings, and not of
      # the one vouching for its own; the workstation built on its own
      # platform warns of no such proof.
      elsewhere = import ./heap-growth-taken {
        inherit lib;
        inherit (crossMachine) pkgs;
      } "one" crossNode 70;
      crossWarnings = (account crossMachine).warnings;
      warnsOf = home: lib.any (lib.hasInfix "built on another platform, whose builder") home.warnings;
      unproven =
        elsewhere ? unproven
        && lib.hasPrefix (unprovenSaidOf "one") elsewhere.unproven
        && lib.hasInfix "name its node in the realm's conversationHeapGrowthVouchedNode" (
          elsewhere.unproven
        )
        && !lib.hasInfix "taken.sh" elsewhere.buildCommand
        && lib.any (lib.hasPrefix (unprovenSaidOf "two")) crossWarnings
        && lib.any (lib.hasPrefix (unprovenSaidOf "three")) crossWarnings
        && !lib.any (lib.hasPrefix (unprovenSaidOf "one")) crossWarnings
        && !warnsOf grown;
      # A realm still giving the bare yes the vouched node replaced is
      # refused, saying what to set instead.
      stillYes = account (bareNixos [
        self.nixosModules.workstation
        {
          home-manager.users.someone.jitsusama.pi.hosts = lib.recursiveUpdate (growingHostsOf (
            _: "${pkgs.emptyDirectory}/node"
          ) { one = 70; }) { one.conversationHeapGrowthMayGoUnproven = true; };
        }
      ]);
      yesSaid = lib.concatStrings [
        "jitsusama.pi.hosts.one.conversationHeapGrowthMayGoUnproven is gone: name the node known "
        "to take conversationHeapGrowingPercent in conversationHeapGrowthVouchedNode instead"
      ];
      yesRefused = lib.any (
        check: !check.assertion && lib.hasPrefix yesSaid check.message
      ) stillYes.assertions;
    in
    # Kept in Nix because each line reads a build Nix gives it for words
    # Nix makes from that build's own inputs, which a script of its own
    # would only be handed back.
    pkgs.runCommandLocal "pi-conversations-refuse-a-node-without-growth" { } ''
      says() {
        if ! grep -qF "$2" "$1/testBuildFailure.log"; then
          echo "a node $3 failed without saying so; its log:"
          cat "$1/testBuildFailure.log"
          exit 1
        fi
      }
      says ${refused} ${lib.escapeShellArg refusedSaid} "refusing the percent"
      says ${refused} ${lib.escapeShellArg "${refusing}: bad option: --heap-growing-percent=70"} \
        "refusing the percent"
      says ${failed} ${lib.escapeShellArg failedSaid} "failing another way"
      says ${failed} ${lib.escapeShellArg "${failing}: can't start"} "failing another way"
      says ${quit} ${lib.escapeShellArg quitSaid} "exiting 124 itself"
      says ${quit} ${lib.escapeShellArg "${quitting}: quitting"} "exiting 124 itself"
      says ${slow} ${lib.escapeShellArg (slowSaid "${hanging}")} "that never ends"
      says ${slow} ${lib.escapeShellArg "${hanging}: still starting"} "that never ends"
      says ${outlasted} ${lib.escapeShellArg (slowSaid "${deaf}")} "ignoring being told to end"
      says ${outlasted} ${lib.escapeShellArg "${deaf}: not listening"} "ignoring being told to end"
      says ${killed} ${lib.escapeShellArg killedSaid} "killed before its limit"
      says ${killed} ${lib.escapeShellArg "${dying}: out of memory"} "killed before its limit"
      says ${unrun} ${lib.escapeShellArg (unrunSaid "/nowhere/node")} "the build can't run"
      says ${folder} ${lib.escapeShellArg (unrunSaid "${pkgs.nodejs_22}")} "named by its folder"
      # taken.sh reads exit 9 as a refusal, which a real node gives an
      # option it doesn't know, saying so.
      for node in ${lib.getExe pkgs.nodejs_22} ${lib.getExe pkgs.nodejs}; do
        if "$node" --heap-growing-percents=70 -e 0 2>said; then
          echo "$node took an option it doesn't know"
          exit 1
        else
          code=$?
        fi
        if [ "$code" != 9 ] || ! grep -qF 'bad option: --heap-growing-percents=70' said; then
          echo "$node refused an option it doesn't know with exit $code," \
            "not as taken.sh reads a refusal:"
          cat said
          exit 1
        fi
      done
      test -e ${taken}
      for unit in ${renderedOf "attended"} ${renderedOf "background"}; do
        if ! grep -qxF ${lib.escapeShellArg "X-HeapGrowthTakenBy=${tracedProof}"} "$unit"; then
          echo "$unit doesn't name the build proving its node takes its percent:"
          cat "$unit"
          exit 1
        fi
      done
      test -e ${tracedProof}
      ${lib.optionalString (!unproven) ''
        echo "a machine cross-built for another platform didn't have its proof stand" \
          "unproven, warning of a realm vouching for another node and not of one vouching for" \
          "its own, or one built on its own platform warned"
        exit 1
      ''}
      ${lib.optionalString (!yesRefused) ''
        echo "a realm still giving conversationHeapGrowthMayGoUnproven wasn't refused" \
          "with what to set instead"
        exit 1
      ''}
      touch $out
    '';

  # The word a realm's conversation is given for a growth of 70 percent, run
  # on node 22, which pi is built with, and on this tree's own node, which
  # stands in for the next pi may be built with: V8 grows each heap limit by
  # a factor of 1.7, in place of one of its own. Not 30, whose 1.3 is the
  # most V8 grows by in its own slow modes, so V8 could give it unasked.
  pi-conversations-take-heap-growth =
    let
      start = startOf growing "pi-conversation-traced-attended@";
      # The one word between the heap and the program.
      words = builtins.match ".* '--max-old-space-size=4096' '([^']*)' /.*" (
        builtins.unsafeDiscardStringContext start
      );
      # Said rather than given to node as an empty word, which it would fail
      # on as a missing script.
      missing = "pi-conversations-take-heap-growth: no one word after the heap in";
      word = if words == null then throw "${missing} ${start}" else lib.head words;
      node22 = lib.getExe pkgs.nodejs_22;
      test = ./tests/pi-conversations-take-heap-growth.mjs;
    in
    pkgs.runCommandLocal "pi-conversations-take-heap-growth" { } ''
      for node in ${node22} ${lib.getExe pkgs.nodejs}; do
        ${node22} ${test} "$node" ${lib.escapeShellArg word} 1.7
      done
      touch $out
    '';
}
