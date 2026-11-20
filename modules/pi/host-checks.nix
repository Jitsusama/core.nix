# The checks of pi's shared hosts, which pi's checks.nix gathers beside its
# own: the units host.nix renders for each realm and the proof each realm's pi
# needs, kept apart since they change with host.nix and nothing else.
{
  account,
  bareDarwin,
  bareNixos,
  holds,
  lib,
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
  # and a realm may not give: written out rather than read from host.nix's
  # list, and the unit must clear just these beside the TPM agent's
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
in
{
  # A host's own restart waits out a call, always, and what a machine adds,
  # such as a meeting being near, in the order given, each with its words:
  # read back from the unit's environment as the host reads it, unquoted
  # and with systemd's doubled % made single.
  pi-hosts-restart-waits-on-calls =
    let
      runtime = "/nix/store/00000000000000000000000000000000-pi-host-runtime-1.5.0";
      named = lib.removeSuffix "\"" (
        lib.removePrefix ''"PI_HOST_RESTART_WAITS_ON='' (
          lib.findFirst (lib.hasPrefix ''"PI_HOST_RESTART_WAITS_ON='') "none" (
            lib.toList
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
                          "/home/someone/100% Calls"
                        ];
                        why = "a meeting is near";
                      }
                    ];
                  };
                }
              ])).systemd.user.services.pi-host-one-background.Service.Environment
          )
        )
      );
      probes = builtins.fromJSON (
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
          (builtins.unsafeDiscardStringContext named)
      );
    in
    holds "pi-hosts-restart-waits-on-calls"
      "A pi host's own restart doesn't wait out a call, or what a machine adds, or names it in another order or in words the host can't read."
      (
        lib.length probes == 2
        && (lib.head probes)."while" == "a call is on"
        && lib.hasSuffix "/bin/a-call-is-on" (lib.head (lib.head probes).run)
        && lib.length (lib.head probes).run == 1
        &&
          lib.last probes == {
            "while" = "a meeting is near";
            run = [
              "/bin/a-meeting-is-near"
              "/home/someone/100% Calls"
            ];
          }
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

  # A realm a machine names gets two hosts, attended and background, started
  # with the account's session once the harness is installed, run by the
  # host's own pi and node, said ready by the host, among the agents'
  # sessions, left running through a switch, and said to be down when left
  # down, never at a restart; a machine that names none, and a Mac, get no host.
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
          { home-manager.users.someone.jitsusama.pi.hosts = hosts; }
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
        # Its core is kept until 20 November (host.nix says why).
        && unit.Service.LimitCORE or null == "infinity"
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
        && lib.hasSuffix "/scripts/pi-host.ts ${kind}" (first unit.Service.ExecStart)
        && lib.hasSuffix "/scripts/pi-host.ts" (first unit.Unit.ConditionPathExists)
        && lib.elem ''"MYCELIUM_REALM=one"'' environment
        && lib.elem ''"PI_PACKAGE_DIR=${pi}"'' environment
        && lib.elem ''"PI_HOST_PROOF=${proof}"'' environment
        # The host parks within the stop limit the unit names.
        && lib.elem ''"PI_HOST_STOP_USEC=150000000"'' environment
        # Every conversation it holds shares its codemode workers.
        && lib.elem ''"PI_CODEMODE_MAX_WORKERS=16"'' environment
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
      "A realm's pi host isn't started with the session, runs another pi or node, leaves its inspector open to SIGUSR1, opens it another way or leaves NODE_OPTIONS, NODE_PATH, NODE_COMPILE_CACHE, the loader's LD_PRELOAD, LD_AUDIT or LD_LIBRARY_PATH, or OpenSSL's OPENSSL_CONF or OPENSSL_MODULES to the user manager, or clears more than those and the TPM agent's variable, says ready before it listens, runs outside agents.slice, restarts after a refusal or at a switch, stops when the kernel kills one of its commands for memory, isn't watched for a wedged loop or isn't killed soon after the watchdog aborts it, isn't given 20 minutes to start, isn't given 150 s to park at a stop or isn't told so, shares pi's two codemode workers among all its conversations, has only node's default heap rather than its realm's, points its conversations at no SSH agent where pi's proxy runs, passes them the TPM agent's own variable, or leaves its commands' processes behind, can't keep its core when it dies, restarts without limit, isn't said to be down once left down or is said to be at each restart, misses its realm's environment or the folders it puts first on PATH, or a machine naming no realm, or a Mac, gets one."
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
