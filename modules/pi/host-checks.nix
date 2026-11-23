# The checks of pi's shared hosts, which pi's checks.nix gathers beside its
# own: the units host.nix renders for each realm and the proof each realm's pi
# needs, kept apart since they change with host.nix and nothing else.
{
  account,
  bareDarwin,
  bareNixos,
  holds,
  lib,
  macWorkstation,
  onLinux,
  pkgs,
  self,
  workstation,
  ...
}:
let
  # A runtime's proof as the harness's host-prove writes it, written without
  # building anything, so reading it back needs no build at evaluation.
  proofOf =
    fields:
    builtins.toFile "pi-host-proof.json" (
      builtins.toJSON (
        {
          kind = "pi-host-runtime-proof";
          version = 1;
          harness = "0000000000000000000000000000000000000000";
          files = 14;
          cases = 71;
          provenAt = "2026-11-10T11:00:00.000Z";
        }
        // fields
      )
    );
  # What would run code as a host as its node starts, which its unit clears
  # and a realm may not give: written out rather than read from
  # host-launch.nix's list, and the unit must clear just these beside the TPM agent's
  # variable, so a change to either list alone fails here.
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

  # A Mac naming one realm, whose environment holds a value the shell would
  # read as more than a word, with a home realm and a probe whose path holds
  # a %, which only systemd doubles, and the harness's entry on it, for the
  # two checks of a Mac's hosts.
  macRuntime = builtins.unsafeDiscardStringContext "${pkgs.emptyDirectory}";
  macProof = proofOf { runtime = macRuntime; };
  macNaming = bareDarwin [
    self.darwinModules.workstation
    {
      home-manager.users.someone.jitsusama = {
        mycelium.realm = "mine";
        pi.hostRestartWaitsWhile = [
          {
            command = [
              "/bin/a-meeting-is-near"
              "/Users/someone/100% kept"
            ];
            why = "a meeting is near";
          }
        ];
        pi.hosts.one = {
          pi = "${macRuntime}/pi";
          node = "${macRuntime}/node";
          proof = macProof;
          path = [ "/first/bin" ];
          environment = {
            PROXY_ORIGIN = "https://proxy.example.com";
            GREETING = "it's $HOME, `here`";
            # The realm's own, which replaces the account's.
            LANG = "C.UTF-8";
          };
        };
      };
    }
  ];
  macEntry =
    (import ./host-launch.nix { inherit lib; }).entry
      macNaming.config.home-manager.users.someone;
in
{
  # A host's own restart waits out a call, always, and what a machine adds,
  # such as a meeting being near, in the order given, each with its words:
  # read back from each kind's unit environment as the host reads it,
  # unquoted and with systemd's doubled % made single. The path holds a %,
  # a quote and a backslash, so each layer of escaping is read through.
  pi-hosts-restart-waits-on-calls =
    let
      runtime = "/nix/store/00000000000000000000000000000000-pi-host-runtime-1.5.0";
      calls = ''/home/someone/100% "Calls" \ kept'';
      services =
        (account (bareNixos [
          self.nixosModules.workstation
          {
            home-manager.users.someone.jitsusama.pi = {
              hosts.one = {
                pi = "${runtime}/lib/node_modules/pi";
                node = "${pkgs.emptyDirectory}/node";
                proof = proofOf { inherit runtime; };
              };
              hostRestartWaitsWhile = [
                {
                  command = [
                    "/bin/a-meeting-is-near"
                    calls
                  ];
                  why = ''a "meeting" is near'';
                }
              ];
            };
          }
        ])).systemd.user.services;
      # The variable's quoted entry in a kind's unit, or null with none.
      named =
        kind:
        lib.findFirst (lib.hasPrefix ''"PI_HOST_RESTART_WAITS_ON='') null (
          lib.toList services."pi-host-one-${kind}".Service.Environment
        );
      probesOf =
        entry:
        builtins.fromJSON (
          lib.replaceStrings
            [
              ''\"''
              ''\\''
              "%%"
            ]
            [
              ''"''
              ''\''
              "%"
            ]
            (
              builtins.unsafeDiscardStringContext (
                lib.removeSuffix "\"" (lib.removePrefix ''"PI_HOST_RESTART_WAITS_ON='' entry)
              )
            )
        );
      waitsRight =
        kind:
        let
          entry = named kind;
          probes = probesOf entry;
        in
        entry != null
        # systemd would read a lone % as a specifier.
        && lib.hasInfix "100%% " entry
        && !(lib.hasInfix "100% " entry)
        && lib.length probes == 2
        && (lib.head probes)."while" == "a call is on"
        && lib.hasSuffix "/bin/a-call-is-on" (lib.head (lib.head probes).run)
        && lib.length (lib.head probes).run == 1
        &&
          lib.last probes == {
            "while" = ''a "meeting" is near'';
            run = [
              "/bin/a-meeting-is-near"
              calls
            ];
          };
    in
    holds "pi-hosts-restart-waits-on-calls"
      "A pi host's own restart doesn't wait out a call, or what a machine adds, or names it in another order, unescaped for systemd, or in words the host can't read."
      (
        lib.all waitsRight [
          "attended"
          "background"
        ]
      );

  # A realm's pi reaches its hosts only with a proof that pnpm test:host passed
  # on the runtime it is in: a proof of another runtime, of one whose name
  # merely starts the same, or not a proof at all fails evaluation, naming the
  # realm and how to prove it.
  pi-hosts-refuse-an-unproven-runtime =
    let
      runtime = "/nix/store/00000000000000000000000000000000-pi-host-runtime-1.5.0";
      pi = "${runtime}/lib/node_modules/pi";
      node = "${pkgs.emptyDirectory}/node";
      realm = proof: { inherit pi node proof; };
      refusals = lib.concatMapStrings (check: check.message + "\n") (
        lib.filter (check: !check.assertion)
          (account (bareNixos [
            self.nixosModules.workstation
            {
              home-manager.users.someone.jitsusama.pi.hosts = {
                proven = realm (proofOf {
                  inherit runtime;
                });
                elsewhere = realm (proofOf {
                  runtime = "/nix/store/11111111111111111111111111111111-pi-host-runtime-1.5.1";
                });
                prefixed = realm (proofOf {
                  runtime = lib.removeSuffix "-1.5.0" runtime;
                });
                stranger = realm (proofOf {
                  inherit runtime;
                  kind = "something-else";
                });
                later = realm (proofOf {
                  inherit runtime;
                  version = 2;
                });
              };
            }
          ])).assertions
      );
    in
    holds "pi-hosts-refuse-an-unproven-runtime"
      "A realm's pi was taken without a proof of its own runtime, or the refusal didn't name the realm and how to prove it."
      (
        !lib.hasInfix "hosts.proven." refusals
        && lib.hasInfix "hosts.elsewhere.proof proves /nix/store/1111" refusals
        && lib.hasInfix "node scripts/host-prove.ts" refusals
        && lib.hasInfix "hosts.prefixed.proof proves" refusals
        && lib.hasInfix "hosts.stranger.proof isn't a pi-host-runtime-proof" refusals
        && lib.hasInfix "hosts.later.proof is version 2, not 1" refusals
      );

  # A realm's own NODE_OPTIONS, or any other variable a host's unit clears
  # since it would run code as the host, fails evaluation, naming the realm
  # and each variable, rather than being dropped unseen.
  pi-hosts-refuse-code-loading-variables =
    let
      runtime = "/nix/store/00000000000000000000000000000000-pi-host-runtime-1.5.0";
      realm = environment: {
        pi = "${runtime}/lib/node_modules/pi";
        node = "${pkgs.emptyDirectory}/node";
        proof = proofOf { inherit runtime; };
        inherit environment;
      };
      # A realm giving each alone, named for it: node_options and so on.
      realmOf = name: lib.toLower name;
      checks =
        (account (bareNixos [
          self.nixosModules.workstation
          {
            home-manager.users.someone.jitsusama.pi.hosts =
              lib.listToAttrs (
                map (
                  name:
                  lib.nameValuePair (realmOf name) (realm {
                    ${name} = "/x";
                  })
                ) loadsCode
              )
              // {
                plain = realm { GREETING = "hi"; };
                # Two, beside one that loads nothing, which the
                # refusal leaves out.
                both = realm {
                  OPENSSL_CONF = "/x.cnf";
                  LD_PRELOAD = "/x.so";
                  GREETING = "hi";
                };
                three = realm {
                  OPENSSL_MODULES = "/x";
                  NODE_PATH = "/x";
                  LD_AUDIT = "/x.so";
                };
              };
          }
        ])).assertions;
      refusals = map (check: check.message) (lib.filter (check: !check.assertion) checks);
      # The one refusal naming a realm, or none.
      prefixOf = realm: "jitsusama.pi.hosts.${realm}.environment gives ";
      refusalOf = realm: lib.findFirst (lib.hasPrefix (prefixOf realm)) null refusals;
      # Whether a realm's refusal names just these variables, as written,
      # and speaks of one as it and of several as each; then names each of
      # loadsCode again where it says how they load code, and where a
      # command gets one instead. How each loads code is the message's to
      # word, so only its names are pinned.
      names =
        realm: named:
        let
          refused = refusalOf realm;
          one = !(lib.hasInfix " and " named);
          head = "${prefixOf realm}${named}, which a pi host's unit clears: ${if one then "it" else "each"} could make";
          explained = lib.removePrefix head refused;
        in
        refused != null
        && lib.hasPrefix head refused
        && lib.all (name: lib.hasInfix " ${name}" explained) loadsCode
        && lib.hasInfix (if one then " Drop it. " else " Drop each one. ") refused
        && lib.hasSuffix " a command that needs one sets it itself." refused;
    in
    holds "pi-hosts-refuse-code-loading-variables"
      "A realm giving NODE_OPTIONS, NODE_PATH, NODE_COMPILE_CACHE, LD_PRELOAD, LD_AUDIT, LD_LIBRARY_PATH, OPENSSL_CONF or OPENSSL_MODULES, which its pi host's unit clears, was taken unseen, or the refusal didn't name the realm and only the ones it gave, speaking of one as it and of several as each, then name each of the eight again where it says how they load code, and say that a command needing one sets it itself."
      (
        refusalOf "plain" == null
        && lib.all (name: names (realmOf name) name) loadsCode
        && names "both" "LD_PRELOAD and OPENSSL_CONF"
        && names "three" "NODE_PATH, LD_AUDIT and OPENSSL_MODULES"
      );

  # A Mac's hosts are launchd daemons run as its primary account, not agents,
  # since launchd ends an account's agents with its login session and every
  # conversation a host holds with them. Each starts at boot, again after it
  # fails and not after a clean stop, once the store is mounted, has the
  # 150 s it parks within and the files it holds open, and names no store
  # path, so a switch that brings a new pi doesn't restart it. Its start is
  # a file a switch changes, which the-mac-pi-host-starts-as-its-account
  # runs. The machine evaluates whole,
  # so a plist or file nix-darwin can't render fails here.
  the-mac-pi-host-outlives-a-log-out =
    let
      mac = macNaming;
      hostsIn =
        machine: lib.filter (lib.hasPrefix "pi-host-") (lib.attrNames machine.config.launchd.daemons);
      # nix-darwin gives a key left unset as null, a set's keys too.
      set = attrs: lib.filterAttrs (_: value: value != null) (if attrs == null then { } else attrs);
      outlives =
        kind:
        let
          daemon = mac.config.launchd.daemons."pi-host-one-${kind}".serviceConfig or { };
          start = mac.config.environment.etc."pi-host/one-${kind}".text or "";
          # The program the start becomes, last on its last line, and its
          # words, ending with the kind.
          last = lib.findFirst (lib.hasPrefix "  /nix/store/") "" (lib.splitString "\n" start);
          program = lib.head (lib.splitString " " (lib.removePrefix "  " last));
        in
        lib.hasSuffix "/scripts/pi-host.ts ${kind}" last
        # Each kind writes its own log, and its own record of starts.
        && lib.hasInfix "/Users/someone/Library/Logs/pi-host-one-${kind}.log" start
        && daemon.Label or null == "org.jitsusama.pi-host-one-${kind}"
        && daemon.UserName or null == "someone"
        && daemon.RunAtLoad or false
        # nix-darwin gives every key of KeepAlive, those unset as null.
        &&
          set (daemon.KeepAlive or { }) == {
            SuccessfulExit = false;
          }
        && daemon.ExitTimeOut or null == 150
        &&
          set (daemon.SoftResourceLimits or { }) == {
            NumberOfFiles = 8192;
          }
        &&
          daemon.ProgramArguments or [ ] == [
            "/bin/sh"
            "-c"
            "/bin/wait4path /nix/store && exec /bin/sh /etc/pi-host/one-${kind}"
          ]
        # nix-darwin restarts a daemon whose plist changed.
        && !lib.hasInfix "/nix/store/" (builtins.toJSON (set daemon))
        # launchd opens these as root, who would follow a link left there;
        # the start opens its log as the account.
        && !(set daemon ? StandardOutPath)
        && !(set daemon ? StandardErrorPath)
        # The realm's node, as a file named pi-host, since a Mac's ps names a
        # process by its program's file, and mycelium tells a forebear so
        # named as an agent's: launchd is the host's parent, and with no
        # pi above them its conversations' commands would be told as the
        # person's.
        && lib.hasSuffix "-pi-host/bin/pi-host" program;
    in
    holds "the-mac-pi-host-outlives-a-log-out"
      "A Mac's pi host isn't a launchd daemon run as its primary account, so a log-out ends it, or isn't started as its own kind with a log of its own, or it isn't started at boot once the store is mounted, is restarted after a clean stop or not after a failure, has less than the 150 s it parks within or launchd's 256 open files, names a store path a switch would restart it for, has launchd open its log as root, runs node under a name mycelium doesn't tell as an agent's, the machine doesn't evaluate whole, or a Mac naming no realm gets one."
      (
        builtins.seq mac.config.system.build.toplevel.drvPath true
        && outlives "attended"
        && outlives "background"
        &&
          hostsIn mac == [
            "pi-host-one-attended"
            "pi-host-one-background"
          ]
        && hostsIn macWorkstation == [ ]
        && lib.all (check: check.assertion) mac.config.assertions
      );

  # A Mac's host's start, run on Linux with the Mac's programs stood in for:
  # it passes shellcheck, moves only a log past 10 MB aside, waits before a
  # fourth start in ten minutes, fails on a missing harness or a temporary
  # or cache folder that isn't a whole path, the account's alone, naming it,
  # and otherwise becomes the realm's node in its own place, with its words,
  # its environment's values whole, TMPDIR the temporary folder,
  # AGENTS_RUNTIME_DIR the cache folder and no XDG_RUNTIME_DIR given, the call
  # probe its own restart waits on, and none of loadsCode though each set.
  the-mac-pi-host-starts-as-its-account =
    let
      start = builtins.unsafeDiscardStringContext (
        macNaming.config.environment.etc."pi-host/one-attended".text or ""
      );
    in
    pkgs.runCommandLocal "the-mac-pi-host-starts-as-its-account"
      {
        nativeBuildInputs = [ pkgs.shellcheck ];
        inherit start;
        passAsFile = [ "start" ];
        entry = macEntry;
      }
      ''
        ${lib.getExe pkgs.bash} ${./tests/the-mac-pi-host-starts-as-its-account.sh} \
          "$startPath" "$entry" ${lib.getExe pkgs.dash} ${lib.getExe pkgs.bash}
        touch $out
      '';

  # A Mac's hosts are launchd daemons run as its primary account, so another
  # account naming a realm is refused by name rather than given hosts
  # nothing starts, and a Mac naming no primary account refuses them all
  # with a message rather than failing to evaluate. A realm giving a DYLD_
  # variable, which a Mac's loader would read in the host's node, is
  # refused by name too.
  a-mac-refuses-another-accounts-pi-hosts =
    let
      runtime = builtins.unsafeDiscardStringContext "${pkgs.emptyDirectory}";
      realm = {
        pi = "${runtime}/pi";
        node = "${runtime}/node";
        proof = proofOf { inherit runtime; };
      };
      mac = bareDarwin [
        self.darwinModules.workstation
        {
          users.users.other.home = "/Users/other";
          home-manager.users.other = {
            home.stateVersion = "26.05";
            jitsusama.pi.hosts.one = realm;
          };
        }
      ];
      unowned = bareDarwin [
        self.darwinModules.workstation
        {
          system.primaryUser = lib.mkForce null;
          home-manager.users.someone.jitsusama.pi.hosts.one = realm;
        }
      ];
      loading = bareDarwin [
        self.darwinModules.workstation
        {
          home-manager.users.someone.jitsusama.pi.hosts.one = realm // {
            environment = {
              DYLD_INSERT_LIBRARIES = "/x.dylib";
              GREETING = "hi";
            };
          };
        }
      ];
      refusalsOf =
        machine:
        map (check: check.message) (lib.filter (check: !check.assertion) machine.config.assertions);
      daemonsOf =
        machine: lib.filter (lib.hasPrefix "pi-host-") (lib.attrNames machine.config.launchd.daemons);
    in
    holds "a-mac-refuses-another-accounts-pi-hosts"
      "A Mac gives an account other than its primary one pi hosts, gives them on a Mac naming no primary account, names no account when it refuses them, or takes a realm's DYLD_ variable, or refuses it without naming the realm and only that variable."
      (
        lib.any (lib.hasPrefix "home-manager.users.other.jitsusama.pi.hosts names a realm") (refusalsOf mac)
        && daemonsOf mac == [ ]
        && lib.any (lib.hasInfix "system.primaryUser names none") (refusalsOf unowned)
        && daemonsOf unowned == [ ]
        && lib.any (lib.hasPrefix "jitsusama.pi.hosts.one.environment gives DYLD_INSERT_LIBRARIES, which") (
          refusalsOf loading
        )
        && !(lib.any (lib.hasInfix "DYLD_") (refusalsOf macNaming))
      );
}
# The hosts run as Linux's systemd user units, in slices a Mac has none of.
// onLinux {
  # A node given the flag a host's unit gives it outlives SIGUSR1 with its
  # inspector shut, where one without opens it; on node 22, which pi is
  # built with, and on this tree's own node. A realm names its host's node,
  # which this can't reach, so the flag's version floor is said on the option.
  pi-hosts-ignore-sigusr1 = pkgs.runCommandLocal "pi-hosts-ignore-sigusr1" { } ''
    for node in ${lib.getExe pkgs.nodejs_22} ${lib.getExe pkgs.nodejs}; do
      ${lib.getExe pkgs.nodejs_22} ${./tests/pi-hosts-ignore-sigusr1.mjs} "$node"
    done
    touch $out
  '';

  # A realm's asyncContextFrame on the node pi is built with, as a host's
  # unit starts it, and the tree's own node, which refuses the flag as
  # node 24 does, so the option's version floor is shown, not only said.
  pi-hosts-keep-async-context = pkgs.runCommandLocal "pi-hosts-keep-async-context" { } ''
    ${lib.getExe pkgs.nodejs_22} ${./tests/pi-hosts-keep-async-context.mjs} ${lib.getExe pkgs.nodejs_22} keeps
    ${lib.getExe pkgs.nodejs_22} ${./tests/pi-hosts-keep-async-context.mjs} ${lib.getExe pkgs.nodejs} refuses
    touch $out
  '';

  # A realm a machine names gets two hosts, attended and background, started
  # with the account's session once the harness is installed, run by the
  # host's own pi and node, said ready by the host, among the agents'
  # sessions, left running through a switch, and said to be down when left
  # down, never at a restart; a machine that names none gets no host, and a
  # Mac no systemd unit, since its hosts are launchd's.
  pi-hosts-start-with-the-session =
    let
      pi = "${pkgs.emptyDirectory}/pi";
      node = "${pkgs.emptyDirectory}/node";
      proof = proofOf { runtime = builtins.unsafeDiscardStringContext "${pkgs.emptyDirectory}"; };
      hosts = {
        one = {
          inherit pi node proof;
          environment = {
            PROXY_ORIGIN = "https://proxy.example.com";
            GREETING = ''say "hi" there'';
          };
        };
        two = {
          inherit pi node proof;
          heapMb = 3072;
          asyncContextFrame = true;
          path = [
            "/first/bin"
            "/second/bin"
            "/100%/$HOME"
          ];
        };
      };
      naming =
        bare: role:
        account (bare [
          role
          {
            home-manager.users.someone.jitsusama = {
              pi.hosts = hosts;
              mycelium.realm = "mine";
            };
          }
        ]);
      services = (naming bareNixos self.nixosModules.workstation).systemd.user.services;
      # The same hosts where pi's SSH agent proxy runs.
      proxied =
        (account (bareNixos [
          self.nixosModules.workstation
          {
            home-manager.users.someone.jitsusama.pi = {
              inherit hosts;
              agentProxy.builderKey = "/builders/key";
            };
          }
        ])).systemd.user.services;
      agentOf =
        services: kind:
        lib.findFirst (lib.hasPrefix ''"SSH_AUTH_SOCK='') null (
          lib.toList services."pi-host-one-${kind}".Service.Environment
        );
      mac = naming bareDarwin self.darwinModules.workstation;
      # home-manager keeps a unit's settings as lists.
      first = value: lib.head (lib.toList value);
      # Whether words name a node option that opens its inspector, or undoes
      # the flag that shuts it, as node reads them, with underscores for
      # dashes.
      opensInspector =
        words:
        let
          read = lib.replaceStrings [ "_" ] [ "-" ] words;
        in
        lib.hasInfix "--inspect" read || lib.hasInfix "--no-disable-sigusr1" read;
      # It refuses what it should, so the check can't pass for a typo.
      readsOptions =
        opensInspector "--inspect_brk=0"
        && opensInspector "--no_disable_sigusr1"
        && !(opensInspector "--disable-sigusr1 --max-old-space-size=8192");
      # Any process of Joel's may send SIGUSR1, which would open node's
      # inspector and let it run code as the host, whose word mycelium takes
      # for which conversation a spawner serves; so node is told to ignore
      # it, nothing opens the inspector another way, and the unit clears
      # NODE_OPTIONS, so neither the user manager's environment nor a
      # machine's hands node an option, nor NODE_PATH a module, nor
      # NODE_COMPILE_CACHE compiled code, nor the loader's or OpenSSL's
      # variables a library's or module's code. A host started through env
      # is held to it as one started plainly is.
      shutsItsInspector =
        unit: start:
        let
          execStart = first unit.Service.ExecStart;
        in
        lib.hasPrefix "${start}${node} --disable-sigusr1 " execStart
        && !(opensInspector execStart)
        &&
          lib.sort lib.lessThan (lib.toList (unit.Service.UnsetEnvironment or [ ]))
          == lib.sort lib.lessThan ([ "SSH_TPM_AUTH_SOCK" ] ++ loadsCode);
      starts =
        kind:
        let
          unit = services."pi-host-one-${kind}";
          environment = lib.toList unit.Service.Environment;
        in
        unit.Service.Type == "notify"
        && unit.Service.NotifyAccess == "all"
        && unit.Service.Slice == "agents.slice"
        && first unit.Service.RestartPreventExitStatus == "78"
        && unit.Service.WatchdogSec or null == "2min"
        # A watchdog's abort is followed by a kill 10 s on.
        && unit.Service.TimeoutAbortSec or null == "10s"
        && unit.Service.KillMode or null == "mixed"
        && unit.Service.TimeoutStopSec or null == "150s"
        # Twenty minutes to start, unless the machine gives longer.
        && unit.Service.TimeoutStartSec or null == "20min"
        # It keeps no core (host.nix says why).
        && unit.Service.LimitCORE or null == "0"
        && unit.Unit.StartLimitIntervalSec or null == 600
        && unit.Unit.StartLimitBurst or null == 3
        && unit.Unit.OnFailure or null == [ "pi-host-down@%N.service" ]
        && unit.Service.RestartMode or null == "direct"
        && unit.Service.OOMPolicy or null == "continue"
        && unit.Unit.X-SwitchMethod == "keep-old"
        && unit.Install.WantedBy == [ "default.target" ]
        && shutsItsInspector unit ""
        # Room for the heap of every conversation it holds, past node's 4 GB,
        # given before the entry, as node reads only flags there; quoted for
        # its =, which systemd unquotes.
        && lib.hasInfix " '--max-old-space-size=8192' /" (first unit.Service.ExecStart)
        # No async context frame flag unless its realm asks.
        && !(lib.hasInfix "async-context-frame" (first unit.Service.ExecStart))
        && lib.hasSuffix "/scripts/pi-host.ts ${kind}" (first unit.Service.ExecStart)
        && lib.hasSuffix "/scripts/pi-host.ts" (first unit.Unit.ConditionPathExists)
        && lib.elem ''"MYCELIUM_REALM=one"'' environment
        && lib.elem ''"PI_PACKAGE_DIR=${pi}"'' environment
        && lib.elem ''"PI_HOST_PROOF=${proof}"'' environment
        # The host parks within the stop limit the unit names.
        && lib.elem ''"PI_HOST_STOP_USEC=150000000"'' environment
        # Only a conversation's own process tunes its allocator.
        && !(lib.any (lib.hasPrefix ''"GLIBC_TUNABLES='') environment)
        # Every conversation it holds shares its codemode workers, and
        # libuv's pool, which its reads wait on.
        && lib.elem ''"PI_CODEMODE_MAX_WORKERS=16"'' environment
        && lib.elem ''"UV_THREADPOOL_SIZE=16"'' environment
        # Its browser is Chromium, which no Chrome policy enrols.
        && lib.elem ''"CHROME_PATH=${lib.getExe pkgs.chromium}"'' environment
        # The account's own mycelium realm, from the shared environment.
        && lib.elem ''"PI_HOME_REALM=mine"'' environment
        && lib.elem ''"PROXY_ORIGIN=https://proxy.example.com"'' environment
        # systemd keeps a quoted entry whole, spaces and escaped quotes in it.
        && lib.elem ''"GREETING=say \"hi\" there"'' environment;
      # A realm naming folders runs node through env, which puts them before
      # the PATH systemd expands, quoted so it stays one word, with a folder's
      # % and $ doubled so systemd takes them as they are.
      findsItsPath =
        kind:
        shutsItsInspector services."pi-host-two-${kind}"
          "${pkgs.coreutils}/bin/env 'PATH=/first/bin:/second/bin:/100%%/$$HOME:\${PATH}' "
        # A realm's own heap reaches both its hosts.
        && lib.hasInfix " '--max-old-space-size=3072' /" (
          first services."pi-host-two-${kind}".Service.ExecStart
        )
        # A realm asking for V8's frame gets it, after the flag that comes first.
        && lib.hasInfix " --disable-sigusr1 --experimental-async-context-frame " (
          first services."pi-host-two-${kind}".Service.ExecStart
        );
      # The notice a host left down starts is a template, not a host.
      hostsIn =
        services:
        lib.filter (name: lib.hasPrefix "pi-host-" name && name != "pi-host-down@") (
          lib.attrNames services
        );
      # It runs once, apart from the agents, on the host's name as %i gives it.
      saysDown =
        let
          unit = services."pi-host-down@" or { };
        in
        unit.Service.Type or null == "oneshot"
        && unit.Service.Slice or null == "background.slice"
        && lib.hasSuffix "/bin/pi-host-down %i" (first (unit.Service.ExecStart or ""));
    in
    holds "pi-hosts-start-with-the-session"
      "A realm's pi host isn't started with the session, runs another pi or node, leaves its inspector open to SIGUSR1, opens it another way or leaves NODE_OPTIONS, NODE_PATH, NODE_COMPILE_CACHE, the loader's LD_PRELOAD, LD_AUDIT or LD_LIBRARY_PATH, or OpenSSL's OPENSSL_CONF or OPENSSL_MODULES to the user manager, or clears more than those and the TPM agent's variable, says ready before it listens, runs outside agents.slice, restarts after a refusal or at a switch, stops when the kernel kills one of its commands for memory, isn't watched for a wedged loop or isn't killed soon after the watchdog aborts it, isn't given 20 minutes to start, isn't given 150 s to park at a stop or isn't told so, shares pi's two codemode workers or libuv's four threads among all its conversations, browses in Google Chrome rather than Chromium, isn't told the account's home realm, has only node's default heap rather than its realm's, misses node's async context frame flag after --disable-sigusr1 where its realm asks for V8's frame, or has it where its realm doesn't, points its conversations at no SSH agent where pi's proxy runs, passes them the TPM agent's own variable, or leaves its commands' processes behind, can dump a core when it dies, is given a conversation's allocator tuning, restarts without limit, isn't said to be down once left down or is said to be at each restart, misses its realm's environment or the folders it puts first on PATH, or a machine naming no realm gets one, or a Mac gets a systemd unit."
      (
        readsOptions
        && starts "attended"
        && starts "background"
        && findsItsPath "attended"
        && findsItsPath "background"
        && saysDown
        # Its conversations reach the agents' SSH agent where the proxy runs,
        # and no agent is named for them where it doesn't.
        && agentOf services "attended" == null
        && agentOf proxied "attended" == ''"SSH_AUTH_SOCK=%t/pi-agent.sock"''
        && agentOf proxied "background" == ''"SSH_AUTH_SOCK=%t/pi-agent.sock"''
        &&
          lib.all
            (
              kind:
              lib.elem "SSH_TPM_AUTH_SOCK" (
                lib.toList (proxied."pi-host-one-${kind}".Service.UnsetEnvironment or [ ])
              )
            )
            [
              "attended"
              "background"
            ]
        && !((account workstation).systemd.user.services ? "pi-host-down@")
        &&
          hostsIn services == [
            "pi-host-one-attended"
            "pi-host-one-background"
            "pi-host-two-attended"
            "pi-host-two-background"
          ]
        && hostsIn (account workstation).systemd.user.services == [ ]
        && hostsIn mac.systemd.user.services == [ ]
        && lib.all (check: check.assertion) (naming bareNixos self.nixosModules.workstation).assertions
      );
}
# The conversations' own template changes with its unit alone, so its check
# sits beside these.
// onLinux (
  import ./conversation-checks.nix {
    inherit
      account
      bareDarwin
      bareNixos
      holds
      lib
      loadsCode
      pkgs
      proofOf
      self
      ;
  }
)
