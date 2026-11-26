# The checks of mycelium, which checks.nix gathers with every other folder's;
# its opening comment says what each kind of check proves. Long because each
# check reads the same few machines built below, which a split would have to
# pass from file to file.
{
  account,
  bareDarwin,
  bareNixos,
  holds,
  lib,
  pkgs,
  self,
  ...
}:
let
  # A workstation of either system whose account sets mycelium's options.
  linking =
    bare: role: settings:
    account (bare [
      role
      { home-manager.users.someone.jitsusama.mycelium = settings; }
    ]);
  linux = linking bareNixos self.nixosModules.workstation;
  mac = linking bareDarwin self.darwinModules.workstation;

  links = {
    realm = "personal";
    links = [
      "one"
      "two"
    ];
  };
  ssh = [
    "--ssh"
    "one"
    "--ssh"
    "two"
  ];
  # home-manager keeps ExecStart as a list of its lines.
  linuxNode = toString (linux links).systemd.user.services.mycelium.Service.ExecStart;
  linuxAgent = (linux links).systemd.user.services.mycelium.Service.Environment;
  # What sends a Linux node's ssh to its realm's key's agent, rather than
  # one the account's SSH config names.
  ownAgent = lib.escapeShellArgs [
    "--ssh-option"
    "IdentityAgent=SSH_AUTH_SOCK"
    "--ssh-option"
    "AddKeysToAgent=no"
    "--ssh-option"
    "IdentitiesOnly=no"
    "--ssh-option"
    "ForwardAgent=no"
  ];
  # The home realm's node of a machine naming sources in its table.
  stimulatingNode =
    toString
      (linux (links // { stimuli = [ "github" ]; })).systemd.user.services.mycelium.Service.ExecStart;
  # A key program in the store, named with a dot as a versioned package is,
  # and never built, since only its path is read.
  modelsKey = pkgs.writeShellScript "models-key-1.0" "echo 'Bearer stand-in'";
  # What a machine lends, in the names of the file mycelium reads.
  lending = {
    vms = [
      {
        prefix = "agents-";
        budget = "models";
        also = [ ];
        requests_a_day = 500;
        usd_a_day = 25;
      }
    ];
    budgets = {
      models = {
        upstream = "model-relay/agents.sock";
        host = "models.example";
        paths = [ "/v1/messages" ];
        auth_header = "authorization";
        key = [ "${modelsKey}" ];
        kind = "models";
        # Every field set, so the file the module writes is this one.
        prices.a-model = {
          input = 3;
          output = 15;
          cache_write = 3.75;
          cache_read = 0.3;
          most_output = 64000;
        };
        spend_watch = "models";
      };
      keyless = {
        upstream = "other-relay/agents.sock";
        host = "other.example";
        paths = [ "/v1/messages" ];
        auth_header = "authorization";
        kind = "models";
      };
    };
  };
  # The home realm's node of a machine that lends.
  lendingNode =
    toString
      (linux (both // { lend = lending; })).systemd.user.services.mycelium.Service.ExecStart;
  # The home realm's node of a machine that asks the harness to park.
  sleepingNode =
    toString
      (linux (links // { easesSleep = true; })).systemd.user.services.mycelium.Service.ExecStart;
  # What a machine that makes the carries it chooses and parks the sessions
  # it would put to sleep sets, as a busy machine would, its account on
  # Linux, its home realm's node there, and a Mac's.
  easing = links // {
    easesSleep = true;
    easesCarry = true;
  };
  easingLinux = linux easing;
  carryingNode = toString easingLinux.systemd.user.services.mycelium.Service.ExecStart;
  macEasing = (mac easing).launchd.agents.mycelium.config.ProgramArguments;
  # A machine whose agents run as an account apart from the node's, as a
  # cloud VM's do, in two realms so only the home one's answers it.
  apartAgents = both // {
    agentsUid = 1001;
  };
  apartAgentsLinux = (linux apartAgents).systemd.user.services;
  apartAgentsNode = toString apartAgentsLinux.mycelium.Service.ExecStart;
  # Whether a node built from `mycelium` may make carries, by
  # carry-guards.nix's own rule, what the switch warns of an account
  # setting it anyway, and whether its node is told to carry.
  inherit (import ./carry-guards.nix { inherit lib; }) guards keepsTheLed missing;
  warnedOf = account: lib.filter (lib.hasInfix "easesCarry") account.warnings;
  carriesIn =
    account:
    lib.hasInfix "--eases-carry" (toString account.systemd.user.services.mycelium.Service.ExecStart);
  # A mycelium source as a machine overriding core's input might have it,
  # from tests/stale-mycelium, whose README says what each holds, under a
  # revision naming it. Its package is never built, since the node's
  # command line is only read as text.
  standIn =
    name:
    self.inputs.mycelium
    // {
      outPath = ./tests/stale-mycelium + "/${name}";
      rev = "stand-in-${name}";
    };
  # The stand-ins lacking a guard, by what the switch must say of each
  # file a guard is looked for in: that it has no line matching the
  # guard's pattern, named so the person can tell what form it takes, that
  # it is missing, or nothing (`kept`) where it keeps its guard. All but
  # `without` and `older` lack one in one file alone, so loosening either
  # pattern lets its own case through.
  refusedSays =
    let
      lacks = file: "${file} has no line of code matching ${guards.${file}}";
      gone = file: "${file} is missing";
      kept = null;
      each = says: lib.mapAttrs (file: say: if say == kept then kept else say file) says;
      easeFile = "src/carry/ease.rs";
      easingFile = "src/carry/easing.rs";
    in
    lib.mapAttrs (_: each) {
      without = {
        ${easeFile} = lacks;
        ${easingFile} = lacks;
      };
      older = {
        ${easeFile} = gone;
        ${easingFile} = gone;
      };
      half = {
        ${easeFile} = kept;
        ${easingFile} = lacks;
      };
      other-half = {
        ${easeFile} = lacks;
        ${easingFile} = kept;
      };
      commented = {
        ${easeFile} = lacks;
        ${easingFile} = kept;
      };
      commented-leads = {
        ${easeFile} = kept;
        ${easingFile} = lacks;
      };
    };
  refused = lib.attrNames refusedSays;
  # A workstation of either system whose account's node is built from
  # `input` in place of core's own.
  machineFrom =
    bare: role: input: settings:
    bare [
      role
      {
        home-manager.users.someone = {
          disabledModules = [ (toString ./home.nix) ];
          imports = [ (lib.modules.importApply ./home.nix { mycelium = input; }) ];
          jitsusama.mycelium = settings;
        };
      }
    ];
  # A Linux workstation with an easing account built from each stand-in,
  # evaluated once each, since every clause below reads the same few.
  staleMachines = lib.genAttrs (refused ++ [ "reshaped" ]) (
    name: machineFrom bareNixos self.nixosModules.workstation (standIn name) easing
  );
  staleAccounts = lib.mapAttrs (_: account) staleMachines;
  stale = staleAccounts.without;
  # A Mac whose account's node is built from `without`, and its account.
  staleMac = machineFrom bareDarwin self.darwinModules.workstation (standIn "without") easing;
  staleMacAccount = account staleMac;
  staleMacArguments = staleMacAccount.launchd.agents.mycelium.config.ProgramArguments;
  # How many of a system's own warnings name the stand-in `without`.
  warnsOfWithout =
    machine: lib.length (lib.filter (lib.hasInfix "(stand-in-without)") machine.config.warnings);
  # The Linux account from `without` with `settings` in place of easing's.
  withoutAs =
    settings:
    account (machineFrom bareNixos self.nixosModules.workstation (standIn "without") settings);
  # What the switch warns of an account whose node is built from the
  # stand-in `name`.
  staleWarning = name: lib.concatStrings (warnedOf staleAccounts.${name});
  # The home realm's node of a machine that makes its carries but leaves
  # parking to the harness's own rule.
  carryingAloneNode =
    toString
      (linux (links // { easesCarry = true; })).systemd.user.services.mycelium.Service.ExecStart;
  # The home realm's node for a machine naming one via, forced whole so a
  # value the option's type refuses fails here.
  viaNode =
    via:
    let
      words =
        toString
          (linux (links // { vias = [ via ]; })).systemd.user.services.mycelium.Service.ExecStart;
    in
    builtins.deepSeq words words;
  # The same for a machine naming one provider's program.
  providerNode =
    program:
    let
      words =
        toString
          (linux (links // { providers = [ program ]; })).systemd.user.services.mycelium.Service.ExecStart;
    in
    builtins.deepSeq words words;
  macNode = (mac links).launchd.agents.mycelium.config.ProgramArguments;
  # A node's words before its realm's kit, which every node is given last.
  unkitted =
    words:
    let
      at = lib.lists.findFirstIndex (word: word == "--kit") null words;
    in
    if at == null then words else lib.take at words;
  # A Mac workstation signs its commits from the Secure Enclave, so its node
  # signs its links in with the same key, each option after its own flag.
  enclaveOptions = lib.concatMap (option: [
    "--ssh-option"
    option
  ]) (mac links).jitsusama.signing.signer.agentSshOptions;
  answering = linux { realm = "personal"; };
  apart = linux { };

  both = {
    realm = "home";
    links = [ "one" ];
    alsoJoins.other.links = [ "two" ];
  };
  bothLinux = (linux both).systemd.user.services;
  otherNode = toString bothLinux.mycelium-other.Service.ExecStart;
  otherApart = [
    "--state"
    "/home/someone/.local/state/mycelium/other"
    "--mesh"
    "/home/someone/.local/state/pi/agentic-harness.pi/realms/other/mesh-workflow"
    "--steer"
    "/home/someone/.local/state/pi/agentic-harness.pi/realms/other/steer"
  ];

  declaring = {
    reach = [ "internet" ];
    probes.home = "192.0.2.1:53";
    probes.office = [
      "test"
      "-e"
      "/run/office"
    ];
    taints = [
      {
        key = "person-works";
        effect = "prefer_not";
        why = "its person works here";
      }
    ];
  };
  # What a declaration says, read as mycelium reads it. A fence's launcher
  # names its program in the store, which fromJSON refuses to read unless
  # the text forgets it was built from that path; the words are the same.
  read = text: builtins.fromJSON (builtins.unsafeDiscardStringContext text);
  declared = bare: read (bare { declares = declaring; }).xdg.configFile."mycelium/declared.json".text;
  # A machine within another, and the host it runs in, each naming the other.
  declaredAs =
    extra: read (linux { declares = declaring // extra; }).xdg.configFile."mycelium/declared.json".text;
  inVm = declaredAs {
    within = {
      machine = "host-machine";
      file = "/run/host/said.json";
    };
  };
  # A machine whose strictly fenced sessions build remotely, and no other.
  strictly = declaredAs {
    fence.strict = {
      builders = [ "remote" ];
      forges = [ ];
      secrets = [ ];
    };
  };
  # An unfenced machine whose sessions carried to another may gain a secret.
  accepting = declaredAs {
    fence.none = {
      builders = [ ];
      forges = [ ];
      secrets = [ ];
      accepts.other-machine.secrets = [ "tokens" ];
    };
  };
  # The same allowance for a machine named as no realm names one.
  misnamed = builtins.tryEval (
    builtins.deepSeq (declaredAs {
      fence.none = {
        builders = [ ];
        forges = [ ];
        secrets = [ ];
        accepts."Other Machine".secrets = [ "tokens" ];
      };
    }) true
  );
  # The same name under `mkIf`: dropped before it is read when false, so
  # it isn't refused, and refused when true, so the false one isn't let
  # through only because `mkIf` drops everything.
  misnamedIf =
    shown:
    builtins.tryEval (
      builtins.deepSeq (declaredAs {
        fence.none = {
          builders = [ ];
          forges = [ ];
          secrets = [ ];
          accepts."Other Machine" = lib.mkIf shown { secrets = [ "tokens" ]; };
        };
      }) true
    );
  # A machine fenced in the kinds it names, each opening no doors.
  fencedIn =
    bare: kinds: settings:
    bare (
      settings
      // {
        declares = declaring // {
          fence = lib.genAttrs kinds (_: {
            builders = [ ];
            forges = [ ];
            secrets = [ ];
          });
        };
      }
    );
  # A machine fenced every way whose pi fences each command itself.
  selfFenced =
    bare:
    bare {
      declares = declaring // {
        piFencesItself = true;
        fence = lib.genAttrs everyKind (_: {
          builders = [ ];
          forges = [ ];
          secrets = [ ];
        });
      };
    };
  saidBy = home: read home.xdg.configFile."mycelium/declared.json".text;
  refusesSelfFencing =
    home:
    lib.any (
      check: !check.assertion && lib.hasPrefix "jitsusama.mycelium.declares.piFencesItself" check.message
    ) home.assertions;
  # A machine fenced every way, and one whose agents' runs are turned off.
  launching = bare: kinds: launchingWith bare kinds { };
  launchingWith =
    bare: kinds: settings:
    read (fencedIn bare kinds settings).xdg.configFile."mycelium/declared.json".text;
  everyKind = [
    "none"
    "fence"
    "strict"
  ];
  # Whether every assertion a machine's account makes holds.
  evaluates = home: lib.all (check: check.assertion) home.assertions;
  # pi's own fence, strict, which an agent's run starts through, each
  # system's with its own build of pi-fence, whichever system checks it.
  # A Mac's strict, its sandbox-exec profile, reads the home folder but what
  # it hides, where Linux's reads only what it lists, so a secret it doesn't
  # name lies open. What it hides was probed by hand on a Mac; nothing here
  # runs it, since core's checks lay no fence on darwin.
  launcherOn = bare: [
    (lib.getExe ((bare [ ]).pkgs.callPackage ../pi/fence-run/package.nix { }))
    "--kind"
    "strict"
    "--"
  ];
  launcher = launcherOn bareNixos;
  macLauncher = launcherOn bareDarwin;
  hosting = declaredAs {
    hosts = [
      {
        machine = "guest-machine";
        file = "/var/host/said.json";
      }
    ];
  };
  # A machine at home naming the command its sessions there start with, or
  # one in a realm it doesn't join.
  saying =
    agents:
    linux {
      realm = "home";
      declares = declaring;
      inherit agents;
    };
  refusesAgents =
    home:
    lib.any (
      check: !check.assertion && lib.hasInfix "jitsusama.mycelium.agents" check.message
    ) home.assertions;
  # A machine in both realms, and one in the sending realm alone, keeping a
  # flow from other into home; and flows naming no realm a machine joins.
  flowing = {
    from = "other";
    to = "home";
  };
  bothFlowing = (linux (both // { flows = [ flowing ]; })).systemd.user.services;
  sendingOnly =
    toString
      (linux {
        realm = "other";
        flows = [ flowing ];
      }).systemd.user.services.mycelium.Service.ExecStart;
  refusesFlows =
    flows:
    lib.any (check: !check.assertion && lib.hasInfix "jitsusama.mycelium.flows" check.message)
      (linux (both // { inherit flows; })).assertions;
in
{
  # A flow is kept by both realms' nodes, one way: the sending realm's node
  # hands text into the other, and the receiving realm's takes it from the
  # first, neither naming the reverse. A machine in the sending realm alone
  # keeps it too, so what it sends waits for a link to carry it, and one
  # naming no flow keeps every realm apart. A flow into its own realm, or
  # between realms the machine joins neither of, is refused.
  mycelium-flows-one-way =
    holds "mycelium-flows-one-way"
      "A realm flow isn't kept one way by both realms' nodes, a machine in the sending realm alone doesn't keep it, a node names a flow nobody declared, or a flow into its own realm or between two realms the machine doesn't join is taken."
      (
        lib.hasInfix "node ${lib.escapeShellArgs otherApart} --flow-to home --ssh two" (
          toString bothFlowing.mycelium-other.Service.ExecStart
        )
        && lib.hasInfix "--realm home node --flow-from other --ssh one" (
          toString bothFlowing.mycelium.Service.ExecStart
        )
        && !lib.hasInfix "--flow-from" (toString bothFlowing.mycelium-other.Service.ExecStart)
        && !lib.hasInfix "--flow-to" (toString bothFlowing.mycelium.Service.ExecStart)
        && lib.hasInfix "--realm other node --flow-to home --waiting" sendingOnly
        && !lib.hasInfix "--flow" (toString bothLinux.mycelium.Service.ExecStart)
        && !lib.hasInfix "--flow" (toString bothLinux.mycelium-other.Service.ExecStart)
        && !refusesFlows [ flowing ]
        && refusesFlows [
          {
            from = "home";
            to = "home";
          }
        ]
        && refusesFlows [
          {
            from = "third";
            to = "fourth";
          }
        ]
      );

  # A workstation in a realm runs the node in it, dialling each machine it
  # names, over SSH or through a provider's program named by its path, in
  # its home realm and in each further one, and asks each provider's
  # program it names what it could make: a user service on Linux, an
  # agent on a Mac. A program named any other way, or a place it could take
  # for a flag or systemd would expand, doesn't evaluate. One that names
  # none runs it dialling nothing, since its node still holds its sessions
  # and starts what others ask of it, and one in no realm has neither a
  # realm nor a node. On Linux the node signs in with the agents' key,
  # which asks nothing, unless its realm names a key of its own, and asks
  # that key's agent, rather than one the account's SSH config names for
  # the machine it dials, never lending or adding to it; one with no key,
  # in the home realm or a further one, is given none of those; on a Mac
  # what it starts signs commits with that key, since nobody at another
  # machine can touch this one's sensor. A Mac's node finds the system's
  # ssh first and the account's programs after it, among them the
  # harness's pi-carry, which a carry runs by name.
  mycelium-links-what-a-machine-names =
    holds "mycelium-links-what-a-machine-names"
      "A machine's links, over SSH or through a provider's program, don't each get dialled in its realm, its providers' programs aren't each asked, a via or a provider naming its program by no path or a place it could misread evaluates, Linux's node signs in with a key that may ask for a PIN or not the one its realm names, or through an agent the account's SSH config names, a Mac's node starts sessions whose commits wait on its sensor or finds the system's ssh after the account's programs or none of them, Linux's node isn't given a soft 65536 and a hard 524288 open files, or a Mac's a soft 10240 and no hard limit of its own, the home realm's node doesn't write how it would ease the machine or a further realm's does, it asks the harness to park sessions or makes its carries though its machine doesn't say to or doesn't where it does, on Linux or a Mac, a machine in a realm naming none runs no node, or one in none runs one."
      (
        lib.hasInfix "--realm personal node --ssh one --ssh two" linuxNode
        && lib.hasInfix "--health-socket %t/daemons/system.sock --eases" linuxNode
        && !(lib.hasInfix "--eases" otherNode)
        && !(lib.hasInfix "--eases" (
          toString
            (linux (
              both
              // {
                easesSleep = true;
                easesCarry = true;
              }
            )).systemd.user.services.mycelium-other.Service.ExecStart
        ))
        && !(lib.hasInfix "--eases-sleep" linuxNode)
        && !(lib.hasInfix "--eases-carry" linuxNode)
        && !(lib.hasInfix "--eases-carry" sleepingNode)
        && lib.hasInfix "--eases --eases-carry --eases-sleep" carryingNode
        && lib.hasInfix "--eases --eases-carry" carryingAloneNode
        && !(lib.hasInfix "--eases-sleep" carryingAloneNode)
        && lib.elem "--eases-carry" macEasing
        && lib.hasInfix "--eases-sleep /Users/someone/.local/state/pi/agentic-harness.pi/park-workflow/ease" (
          toString macEasing
        )
        && lib.hasInfix "--eases --eases-sleep /home/someone/.local/state/pi/agentic-harness.pi/park-workflow/ease" sleepingNode
        && lib.hasInfix "--ssh one --ssh two --via /bin/cell:cell-1 --waiting" (viaNode "/bin/cell:cell-1")
        && lib.all (wrong: !(builtins.tryEval (viaNode wrong)).success) [
          "cell:cell-1"
          "/bin/cell:"
          "/bin/cell:-c"
          "/bin/my cell:cell-1"
          "/bin/cell:cell 1"
          "/bin/cell:cell-%h"
          "/bin/cell:cell-$HOME"
        ]
        && lib.hasInfix "--ssh one --ssh two --provider /bin/cell --waiting" (providerNode "/bin/cell")
        && lib.all (wrong: !(builtins.tryEval (providerNode wrong)).success) [
          "cell"
          "/bin/my cell"
          "/bin/cell:make"
          "/bin/cell-%h"
        ]
        && lib.hasInfix "--ssh two --via /bin/cell:cell-2 --provider /bin/cell --waiting" (
          toString
            (linux (
              both
              // {
                alsoJoins.other = {
                  links = [ "two" ];
                  vias = [ "/bin/cell:cell-2" ];
                  providers = [ "/bin/cell" ];
                };
              }
            )).systemd.user.services.mycelium-other.Service.ExecStart
        )
        && linuxAgent == [ "SSH_AUTH_SOCK=%t/agent-signing.sock" ]
        &&
          (linux (links // { signsInWith = "personal-link"; }))
          .systemd.user.services.mycelium.Service.Environment == [ "SSH_AUTH_SOCK=%t/personal-link.sock" ]
        && lib.hasInfix "${ownAgent} --kit " linuxNode
        && lib.hasInfix "${ownAgent} --kit " otherNode
        && !(lib.hasInfix "--ssh-option" (
          toString (linux (links // { signsInWith = null; })).systemd.user.services.mycelium.Service.ExecStart
        ))
        && !(lib.hasInfix "--ssh-option" (
          toString
            (linux (both // { alsoJoins.other.signsInWith = null; }))
            .systemd.user.services.mycelium-other.Service.ExecStart
        ))
        &&
          lib.drop 1 (unkitted macNode) == [
            "--realm"
            "personal"
            "node"
          ]
          ++ enclaveOptions
          ++ ssh
          ++ [
            "--waiting"
            "${(mac links).xdg.stateHome}/mission/waiting"
            "--serves"
            "${(mac links).xdg.configHome}/mycelium/serves-personal.json"
            "--health-socket"
            "${(mac links).xdg.stateHome}/daemons/system.sock"
            "--eases"
          ]
        && (mac links).launchd.agents.mycelium.enable
        && (mac links).launchd.agents.mycelium.config.EnvironmentVariables.SIGN_AS == "agent"
        && (mac links).launchd.agents.mycelium.config.SoftResourceLimits.NumberOfFiles == 10240
        && (linux links).systemd.user.services.mycelium.Service.LimitNOFILE == "65536:524288"
        && (mac links).launchd.agents.mycelium.config.HardResourceLimits or null == null
        &&
          (mac links).launchd.agents.mycelium.config.EnvironmentVariables.PATH
          == "/usr/bin:/bin:${(mac links).home.profileDirectory}/bin"
        && lib.hasInfix "--realm personal node" (
          toString answering.systemd.user.services.mycelium.Service.ExecStart
        )
        && !(lib.elem "--ssh" (
          lib.splitString " " (toString answering.systemd.user.services.mycelium.Service.ExecStart)
        ))
        && (mac { realm = "personal"; }).launchd.agents.mycelium.enable
        && !(apart.systemd.user.services ? mycelium)
        && answering.home.sessionVariables.MYCELIUM_REALM == "personal"
        && !(apart.home.sessionVariables ? MYCELIUM_REALM)
        && lib.any (package: (package.pname or "") == "mycelium") apart.home.packages
      );

  # Core's own lock keeps carry-guards.nix's guards, so a lock moving to
  # a mycelium that wrote either line in a form the patterns don't accept
  # fails here, naming what it lacks, rather than every machine on core's
  # lock quietly ceasing to carry.
  mycelium-carry-guards-match-its-lock =
    holds "mycelium-carry-guards-match-its-lock"
      "Core's locked mycelium doesn't keep carry-guards.nix's guards, so every machine on core's lock would stop carrying: change the patterns there to the lines it has, or lock a mycelium with them. What the switch would say: ${missing self.inputs.mycelium}"
      (keepsTheLed self.inputs.mycelium);

  # A machine's node carries only from a mycelium that keeps the person's
  # and the leading sessions in place. Each stand-in lacking a guard says
  # which of its files lack one, and all but two lack one in one file
  # alone, so neither pattern can be loosened unseen; the one keeping both
  # in other forms carries, so neither can be made too strict. The switch
  # warns, in the system's own warnings on Linux and a Mac alike, naming
  # that mycelium and only the files lacking a guard, and never where the
  # machine doesn't carry or runs no node.
  mycelium-carries-only-with-its-guards =
    holds "mycelium-carries-only-with-its-guards"
      "A machine's node is told to carry from a mycelium whose code lacks either guard that keeps the person's and the leading sessions, names one only in a line comment, or has no file to hold it, or the switch doesn't warn of it once in the system's warnings on Linux or a Mac, naming that mycelium and only the files lacking one and how, or a node built from a mycelium keeping both in forms the patterns accept doesn't carry, or a machine that doesn't carry, or runs no node, is warned, or the stand-ins' table doesn't name exactly the files carry-guards.nix's guards read."
      (
        keepsTheLed (standIn "reshaped")
        && carriesIn staleAccounts.reshaped
        && warnedOf staleAccounts.reshaped == [ ]
        && !(keepsTheLed { })
        && warnedOf easingLinux == [ ]
        && lib.hasInfix "--eases --eases-sleep " (
          toString stale.systemd.user.services.mycelium.Service.ExecStart
        )
        && lib.all (
          name:
          let
            warning = staleWarning name;
            saysOf =
              file: said: if said == null then !(lib.hasInfix file warning) else lib.hasInfix said warning;
          in
          !(keepsTheLed (standIn name))
          && !(carriesIn staleAccounts.${name})
          && lib.length (warnedOf staleAccounts.${name}) == 1
          && lib.hasInfix "(stand-in-${name})" warning
          # The table must name the guards' files before its rows are read, so
          # a guard it lacks fails with this check's message, not a missing key.
          && lib.attrNames refusedSays.${name} == lib.attrNames guards
          && lib.all (file: saysOf file refusedSays.${name}.${file}) (lib.attrNames guards)
        ) refused
        && warnsOfWithout staleMachines.without == 1
        && warnsOfWithout staleMac == 1
        && lib.length (warnedOf staleMacAccount) == 1
        && lib.all (lib.flip lib.elem staleMacArguments) [
          "--eases"
          "--eases-sleep"
        ]
        && !(lib.elem "--eases-carry" staleMacArguments)
        && warnedOf (withoutAs (easing // { easesCarry = false; })) == [ ]
        &&
          warnedOf (withoutAs {
            easesCarry = true;
          }) == [ ]
      );

  # A machine in a realm reads each minute how evenly the home realm's
  # machines carry its work, on Linux in the agents' slice, on a Mac as an
  # agent; only the home realm is read, and a machine in none reads nothing.
  mycelium-watches-its-balance =
    holds "mycelium-watches-its-balance"
      "A machine in a realm doesn't read its home realm's balance each minute, reads it outside the agents' slice on Linux, reads a further realm's, or one in no realm reads one."
      (
        let
          linuxBalance = bothLinux.mycelium-balance.Service;
          macBalance = (mac both).launchd.agents.mycelium-balance;
        in
        lib.hasSuffix "/bin/mycelium --realm home balance --follow" (toString linuxBalance.ExecStart)
        && linuxBalance.Slice == "agents.slice"
        && macBalance.enable
        &&
          lib.drop 1 macBalance.config.ProgramArguments == [
            "--realm"
            "home"
            "balance"
            "--follow"
          ]
        && !(bothLinux ? mycelium-other-balance)
        && !(apart.systemd.user.services ? mycelium-balance)
      );

  # The home realm's node on Linux shows the Nix builders nix-offload's
  # lists name beside the machines, the lent one among them, by the folder
  # its lender keeps its health in; a further realm's node shows none, as
  # they are the machine's, and a Mac's, with no lists, is given none.
  mycelium-shows-the-builders =
    holds "mycelium-shows-the-builders"
      "Linux's home realm node isn't given nix-offload's standing, beside and lent lists and the lent builder's health, a further realm's node is, or a Mac's node is."
      (
        lib.hasInfix (lib.escapeShellArgs [
          "--builders"
          "standing=/run/nix-offload-lines/machines-with-standing"
          "--builders"
          "beside=/run/nix-offload-lines/machines-beside"
          "--lent-builders"
          "lent=/run/nix-offload-lines/machines-for-anything"
          "--lent-health"
          "/run/{host}/health.json"
        ]) linuxNode
        && !(lib.hasInfix "--builders" otherNode)
        && !(lib.elem "--builders" macNode)
        && !(lib.elem "--lent-builders" macNode)
      );

  # A Mac's node signs its links in with the agents' key from the Secure
  # Enclave, through that key's own library and no agent, since the login
  # session's one may ask for an approval nobody is there to give; one
  # without that key's module passes no such options, and Linux's node is
  # given none of them, even from a signer that names them.
  mycelium-signs-a-macs-links-with-the-enclave =
    holds "mycelium-signs-a-macs-links-with-the-enclave"
      "A Mac's node signs its links in through whatever agent the login holds, or not with the agents' key from the Secure Enclave, a Mac without that key's module is given options for it, or Linux's node is given a Mac's options."
      (
        let
          enclave = {
            home-manager.sharedModules = [ self.homeModules.secure-enclave-signing ];
          };
          withEnclave =
            (bareDarwin [
              self.darwinModules.workstation
              enclave
              { home-manager.users.someone.jitsusama.mycelium = links; }
            ]).config.home-manager.users.someone;
          words = withEnclave.launchd.agents.mycelium.config.ProgramArguments;
          # A Linux account whose signer names options as a Mac's does, so
          # only the node's own gate keeps them out.
          linuxSigned =
            toString
              (account (bareNixos [
                self.nixosModules.workstation
                {
                  home-manager.users.someone = {
                    jitsusama.mycelium = links;
                    jitsusama.signing.signer.agentSshOptions = [
                      "IdentityAgent=none"
                      "IdentitiesOnly=yes"
                    ];
                  };
                }
              ])).systemd.user.services.mycelium.Service.ExecStart;
        in
        lib.elem "IdentityFile=/Users/someone/.ssh/agent-signing" words
        && lib.elem "SecurityKeyProvider=/usr/lib/ssh-keychain.dylib" words
        && lib.elem "IdentityAgent=none" words
        && lib.elem "IdentitiesOnly=yes" words
        && lib.elem "--ssh-option" words
        && !(lib.hasInfix "IdentityAgent=none" linuxSigned)
        && !(lib.hasInfix "IdentitiesOnly=yes" linuxSigned)
      );

  # A machine joining a further realm runs a node of its own for it, apart
  # as another machine's would be: its own state, and the mesh and steer
  # folders the harness keeps for a realm that isn't home. Its links sign in
  # with that realm's own key, never the agents' key every session reaches,
  # and the home realm's node is as it was. Its pis know which realm is home.
  mycelium-keeps-each-realm-apart =
    holds "mycelium-keeps-each-realm-apart"
      "A further realm's node shares the home realm's state, mesh, steer folders or key, the home realm's node changes, or a pi isn't told its home realm."
      (
        lib.hasInfix (lib.escapeShellArgs (
          [
            "--realm"
            "other"
            "node"
          ]
          ++ otherApart
          ++ [
            "--ssh"
            "two"
          ]
        )) otherNode
        && bothLinux.mycelium-other.Service.Environment == [ "SSH_AUTH_SOCK=%t/other-link.sock" ]
        && bothLinux.mycelium-other.Unit.Wants == [ "other-link.socket" ]
        && lib.hasInfix "--realm home node --ssh one --waiting /home/someone/.local/state/mission/waiting --serves /home/someone/.config/mycelium/serves-home.json --health-socket %t/daemons/system.sock" (
          toString bothLinux.mycelium.Service.ExecStart
        )
        && bothLinux.mycelium.Service.Environment == [ "SSH_AUTH_SOCK=%t/agent-signing.sock" ]
        && lib.hasInfix "/.local/state/pi/agentic-harness.pi/realms/other/steer --ssh-option " (
          toString (mac both).launchd.agents.mycelium-other.config.ProgramArguments
        )
        && lib.hasInfix " --ssh two --waiting " (
          toString (mac both).launchd.agents.mycelium-other.config.ProgramArguments
        )
        && (linux both).home.sessionVariables.PI_HOME_REALM == "home"
        && (linux both).home.sessionVariables.MYCELIUM_REALM == "home"
      );

  # Each realm's node serves the harness's files of its realm, as the
  # harness keeps them: the home realm's in the harness's usual folders, a
  # further realm's in its own, and the machine's waiting and verdicts on
  # the home realm's node alone, so nothing is heard twice.
  # The node wakes with every session it carries, so it runs in their
  # slice, every realm's alike, rather than among Joel's programs.
  # The locked node takes each flag its service is given, so a flag set
  # ahead of the mycelium that reads it fails here, not as the node starts.
  # It reads only the command's help, so it builds the node without its
  # tests: mycelium's own gate runs those, and their timing bars fail on a
  # busy Mac or its Linux builder, which would fail this check for nothing
  # it proves. The flags are words alone, so the nodes' lines give them
  # without their store paths, which would bring the Linux node's whole build
  # into a Mac's check. Each line also says which of its flags take a value,
  # those a word other than a flag follows, and the help must say the same,
  # since a switch the node reads as taking one would swallow the next flag
  # and stop the node starting. It proves names and values, not that the
  # node honours them, nor that two flags may go together.
  mycelium-node-takes-its-flags =
    let
      lines = map (line: lib.filter (word: word != "") (lib.splitString " " line)) (
        map builtins.unsafeDiscardStringContext [
          linuxNode
          otherNode
          stimulatingNode
          lendingNode
          sleepingNode
          carryingNode
          carryingAloneNode
          apartAgentsNode
          (viaNode "/bin/cell:cell-1")
          (providerNode "/bin/cell")
          (toString bothFlowing.mycelium.Service.ExecStart)
          (toString bothFlowing.mycelium-other.Service.ExecStart)
          (toString
            (linux (links // { declares = declaring; })).systemd.user.services.mycelium.Service.ExecStart
          )
        ]
      );
      # Each flag of a line, with whether a value follows it there.
      said =
        words:
        lib.concatLists (
          lib.imap0 (
            at: word:
            lib.optional (lib.hasPrefix "--" word) {
              flag = word;
              valued = at + 1 < lib.length words && !(lib.hasPrefix "--" (lib.elemAt words (at + 1)));
            }
          ) words
        );
      flagged = lib.unique (lib.concatMap said lines);
      flags = lib.unique (map (one: one.flag) flagged);
      valued = map (one: one.flag) (lib.filter (one: one.valued) flagged);
      bare = map (one: one.flag) (lib.filter (one: !one.valued) flagged);
      locked = self.inputs.mycelium.packages.${pkgs.stdenv.hostPlatform.system}.default;
      untested = locked.overrideAttrs { doCheck = false; };
      mycelium = lib.getExe untested;
    in
    # Kept in Nix because its flags and node come from this evaluation,
    # and a script of its own would only be handed them back as words.
    pkgs.runCommandLocal "mycelium-node-takes-its-flags" { } ''
      ${mycelium} node --help > help
      for flag in ${lib.escapeShellArgs flags}; do
        # The whole flag, so one dropped beside a longer one, as --ssh
        # beside --ssh-option, isn't taken as there.
        grep -q -E -e "(^|[[:space:]])$flag([[:space:]=,]|\$)" help \
          || { echo "mycelium node doesn't take $flag"; exit 1; }
      done
      # clap's help names a flag's value after it, as --flag <VALUE>, and one
      # it takes only after an =, as --flag=<VALUE>, which a line can't give.
      for flag in ${lib.escapeShellArgs valued}; do
        grep -q -E -e "(^|[[:space:]])$flag \[?<" help \
          || { echo "mycelium node takes no value after $flag, which is given one"; exit 1; }
      done
      for flag in ${lib.escapeShellArgs bare}; do
        ! grep -q -E -e "(^|[[:space:]])$flag[ =]<" help \
          || { echo "mycelium node wants a value after $flag, which is given none"; exit 1; }
      done
      touch $out
    '';

  # Every act Joel takes on his cells is on all four of his surfaces, the
  # mycelium command, a pi session, the map and the launcher, or owed by a
  # cell by a time, or waived for a reason (surfaces/acts.toml): Joel,
  # 2026-11-24, "any interface i'm in should be directly engagable to a
  # meaningful level". Where the table says a surface has an act, the file
  # it names holds its words, or the locked mycelium's help for the command
  # says them; a pi tool is taken by its name, since the harness isn't an
  # input. Where a surface lists its acts in a file, an act there the table
  # doesn't name fails, so one a surface gains is seen; the launcher's
  # session row and the map's cell drawer are read. So it can't pass by reading nothing,
  # each way of straying is shown to fail first, beside a clean act that
  # passes, so each fails on its own fault alone. It proves
  # each act's words are where the table says, not that the code offers or
  # works it, nor that an act is worded alike on each surface.
  mycelium-surfaces-stay-even =
    let
      table = ./surfaces/acts.toml;
      read = builtins.fromTOML (builtins.readFile table);
      acts = read.act;
      named = lib.unique (
        lib.concatMap (
          act:
          lib.concatMap (
            surface:
            let
              has = act.${surface}.has or { };
            in
            lib.optional (has ? file) has.file
          ) (lib.attrNames act)
        ) acts
        ++ map (offered: offered.file) (read.offered or [ ])
        # The strays' own files, named here so they don't rest on the table;
        # nowhere.ts is never there, so the gone stray goes through kept.
        ++ [
          "modules/mycelium/site/view.js"
          "modules/mycelium/site/nowhere.ts"
          "modules/quickshell/launcher/words/agents.json"
        ]
      );
      # Only the files the table names, so an edit elsewhere rebuilds nothing.
      # One renamed away is left out rather than stopping evaluation, so the
      # check says which is gone.
      kept = file: lib.fileset.maybeMissing (../.. + "/${file}");
      root = lib.fileset.toSource {
        root = ../..;
        fileset = lib.fileset.unions (map kept named);
      };
      locked = self.inputs.mycelium.packages.${pkgs.stdenv.hostPlatform.system}.default;
      untested = locked.overrideAttrs { doCheck = false; };
      check = "${lib.getExe pkgs.python3} ${./surfaces/stays-even.py}";
      # An act on each surface, with what one of them says in place of the
      # rest, as a table of one act gives it. Only the wayless act leaves
      # out the way to it every surface takes. The clean act, which strays
      # nowhere, must pass, so each stray fails on its own fault alone.
      clean = act "clean" ''
        [act.cli]
        waived = "no window to raise"
        [act.launcher]
        waived = "none"
      '';
      act =
        name: says:
        pkgs.writeText "${name}.toml" ''
          [[act]]
          name = "go"
          says = "go to a cell"
          ${lib.optionalString (name != "wayless") ''through = "the launcher's go"''}
          [act.session.has]
          tool = "mesh_read"
          [act.map.has]
          file = "modules/mycelium/site/view.js"
          holds = "It Asks You"
          ${says}
        '';
      # Each way of straying the check has to catch, with what it says.
      strays = [
        {
          table = act "gone" ''
            [act.cli]
            waived = "no window to raise"
            [act.launcher.has]
            file = "modules/mycelium/site/nowhere.ts"
            holds = "Go"
          '';
          says = "go on the launcher: modules/mycelium/site/nowhere.ts isn't there";
        }
        {
          table = act "ownerless" ''
            [act.cli]
            waived = "no window to raise"
            [act.launcher.owed]
            by = "soon"
            due = "2026-11-24 17:00"
          '';
          says = "go on the launcher is owed by no cell's address";
        }
        {
          table = act "helpless" ''
            [act.cli.has]
            command = "flies"
            holds = "to the moon"
            [act.launcher]
            waived = "none"
          '';
          says = "go on the mycelium command: mycelium flies --help failed, exited";
        }
        {
          table = act "whyless" ''
            [act.cli]
            waived = "no window to raise"
            [act.launcher.owed]
            by = "KP0L6A"
            due = "2026-11-24 17:00"
          '';
          says = "go on the launcher is owed with no why";
        }
        {
          table = act "unpadded" ''
            [act.cli]
            waived = "no window to raise"
            [act.launcher.owed]
            by = "KP0L6A"
            due = "2026-11-24 9:00"
            why = "soon"
          '';
          says = "go on the launcher is owed with no due time";
        }
        {
          table = act "unoffered" ''
            [act.cli]
            waived = "no window to raise"
            [act.launcher]
            waived = "none"
            [[offered]]
            surface = "launcher"
            file = "modules/quickshell/launcher/words/agents.json"
            pattern = '"id": "verb:agent:([a-z-]+)"'
            [offered.acts]
            switch = "go"
            take-back = "go"
            pause = "go"
            resume = "go"
            wake = "go"
            stop = "go"
            close = "go"
          '';
          says = "what the launcher lists: talk is no act of the table, nor set apart";
        }
        {
          table = act "unlisted" ''
            [act.cli]
            waived = "no window to raise"
            [act.launcher]
            waived = "none"
            [[offered]]
            surface = "launcher"
            file = "modules/quickshell/launcher/words/agents.json"
            pattern = '"id": "verb:agent:([a-z-]+)"'
            [offered.acts]
            switch = "go"
            talk = "go"
            take-back = "go"
            pause = "go"
            resume = "go"
            wake = "go"
            stop = "go"
            close = "go"
            gone = "go"
          '';
          says = "launcher/words/agents.json no longer lists gone";
        }
        {
          table = act "misaimed" ''
            [act.cli]
            waived = "no window to raise"
            [act.launcher]
            waived = "none"
            [[offered]]
            surface = "launcher"
            file = "modules/quickshell/launcher/words/agents.json"
            pattern = '"id": "verb:agent:([a-z-]+)"'
            [offered.acts]
            switch = "went"
            talk = "go"
            take-back = "go"
            pause = "go"
            resume = "go"
            wake = "go"
            stop = "go"
            close = "go"
          '';
          says = "switch is said to be went, no act of the table";
        }
        {
          table = act "unmatched" ''
            [act.cli]
            waived = "no window to raise"
            [act.launcher]
            waived = "none"
            [[offered]]
            surface = "launcher"
            file = "modules/quickshell/launcher/words/agents.json"
            pattern = '"id": "verb:nobody:([a-z-]+)"'
          '';
          says = "agents.json lists no act, as";
        }
        {
          table = act "left-out" ''
            [act.cli]
            waived = "no window to raise"
          '';
          says = "go on the launcher: the table leaves it out";
        }
        {
          table = act "not-held" ''
            [act.cli]
            waived = "no window to raise"
            [act.launcher.has]
            file = "modules/mycelium/site/view.js"
            holds = "Never Said Anywhere"
          '';
          says = ''go on the launcher: modules/mycelium/site/view.js doesn't hold "Never'';
        }
        {
          table = act "not-helped" ''
            [act.cli.has]
            command = "steer"
            holds = "flies to the moon"
            [act.launcher]
            waived = "none"
          '';
          says = ''go on the mycelium command: mycelium steer --help doesn't say "flies'';
        }
        {
          table = act "undue" ''
            [act.cli]
            waived = "no window to raise"
            [act.launcher.owed]
            by = "KP0L6A"
            why = "soon"
          '';
          says = "go on the launcher is owed with no due time";
        }
        {
          table = act "twice" ''
            [act.cli]
            waived = "no window to raise"
            owed = { by = "KP0L6A", due = "2026-11-24 17:00", why = "soon" }
            [act.launcher]
            waived = "none"
          '';
          says = "go on the mycelium command says owed and waived, not one";
        }
        {
          table = act "reasonless" ''
            [act.cli]
            waived = ""
            [act.launcher]
            waived = "none"
          '';
          says = "go on the mycelium command: its waived is malformed or gives no reason";
        }
        {
          table = act "elsewhere" ''
            [act.cli]
            waived = "no window to raise"
            [act.launcher]
            waived = "none"
            [act.phone]
            waived = "none"
          '';
          says = "go names phone, which isn't a surface";
        }
        {
          table = act "wayless" ''
            [act.cli]
            waived = "no window to raise"
            [act.launcher]
            waived = "none"
          '';
          says = "go doesn't say the way to it every surface takes";
        }
      ];
    in
    # Kept in Nix because each line runs the check on a table Nix wrote, or
    # loops over the strays Nix lists, so the script is only their glue.
    pkgs.runCommandLocal "mycelium-surfaces-stay-even" { } ''
      strays() {
        if ${check} "$1" ${root} ${lib.getExe untested} > said; then
          echo "The surfaces check passed on $1, which strays."
          exit 1
        fi
        if ! grep -qF -- "$2" said; then
          echo "On $1 the surfaces check didn't say: $2"
          cat said
          exit 1
        fi
      }
      if ! ${check} ${clean} ${root} ${lib.getExe untested} > said; then
        echo "The surfaces check refused the clean act, so no stray shows its own fault."
        cat said
        exit 1
      fi
      ${lib.concatMapStrings (stray: ''
        strays ${stray.table} ${lib.escapeShellArg stray.says}
      '') strays}
      ${check} ${table} ${root} ${lib.getExe untested}
      touch $out
    '';

  # The swarm's teams complete in a real zsh, asked of a stand-in node
  # (tests/completion.zsh).
  mycelium-completes-its-teams-in-zsh =
    pkgs.runCommandLocal "mycelium-completes-its-teams-in-zsh" { nativeBuildInputs = [ pkgs.zsh ]; }
      ''
        export HOME=$PWD
        zsh -f ${./tests/completion.zsh} ${./completion.zsh}
        touch $out
      '';

  mycelium-runs-among-the-sessions =
    holds "mycelium-runs-among-the-sessions"
      "A mycelium node runs outside agents.slice, where its wakeups preempt Joel's programs."
      (
        lib.all (node: (node.Service.Slice or null) == "agents.slice") [
          bothLinux.mycelium
          bothLinux.mycelium-other
        ]
      );

  mycelium-serves-the-harness-files =
    holds "mycelium-serves-the-harness-files"
      "A realm's node doesn't serve its realm's open and ended quests, restore list, runs, landings and talks from that realm's folders, the machine's waiting, verdicts, queue, the void's answers and the balance's latest reading aren't served by the home realm's node alone, a node reads its cells' waits from any folder but the waiting one the home realm's node serves, or a node, on Linux or a Mac, doesn't restart when what it serves changes."
      (
        let
          state = "/home/someone/.local/state";
          serves = realm: builtins.fromJSON (linux both).xdg.configFile."mycelium/serves-${realm}.json".text;
        in
        serves "home" == {
          quest = {
            file = "${state}/pi/agentic-harness.pi/quest-workflow/quests.json";
            records = "quests";
            key = "id";
          };
          ended-quest = {
            file = "${state}/pi/agentic-harness.pi/quest-workflow/ended.json";
            records = "quests";
            key = "id";
          };
          restore = {
            file = "${state}/pi/restore.json";
            records = "sessions";
            key = "id";
          };
          run = {
            folder = "${state}/pi/agentic-harness.pi/mesh-workflow/runs";
            records = "runs";
            key = "id";
          };
          landing = {
            file = "${state}/pi/agentic-harness.pi/mesh-workflow/landings.json";
            records = "landings";
            key = "key";
          };
          talk.talks = "${state}/pi/agentic-harness.pi/steer";
          waiting = {
            folder = "${state}/mission/waiting";
            records = "items";
            key = "id";
            or = "title";
            by = "quest";
          };
          verdict = {
            file = "${state}/mission/verdicts.json";
            records = "items";
          };
          job = {
            file = "jobs/queue.json";
            within = "runtime";
            records = "jobs";
            key = "id";
          };
          queue = {
            file = "jobs/queue.json";
            within = "runtime";
            whole = true;
          };
          void-answers = {
            file = "${state}/void/answers.jsonl";
            whole = true;
          };
          balance = {
            file = "${state}/mycelium/balance/now.json";
            whole = true;
          };
        }
        &&
          serves "other" == {
            quest = {
              file = "${state}/pi/agentic-harness.pi/realms/other/quest-workflow/quests.json";
              records = "quests";
              key = "id";
            };
            ended-quest = {
              file = "${state}/pi/agentic-harness.pi/realms/other/quest-workflow/ended.json";
              records = "quests";
              key = "id";
            };
            restore = {
              file = "${state}/pi/agentic-harness.pi/realms/other/restore.json";
              records = "sessions";
              key = "id";
            };
            run = {
              folder = "${state}/pi/agentic-harness.pi/realms/other/mesh-workflow/runs";
              records = "runs";
              key = "id";
            };
            landing = {
              file = "${state}/pi/agentic-harness.pi/realms/other/mesh-workflow/landings.json";
              records = "landings";
              key = "key";
            };
            talk.talks = "${state}/pi/agentic-harness.pi/realms/other/steer";
          }
        # Each realm's node reads its cells' waits, for their forages, from
        # the one folder the launcher writes and the home node serves.
        && lib.hasInfix "--waiting ${state}/mission/waiting --serves /home/someone/.config/mycelium/serves-other.json --health-socket %t/daemons/system.sock" otherNode
        && !(apart.xdg.configFile ? "mycelium/serves-home.json")
        # The node reads its serves file only as it starts, so a switch
        # changing what it serves restarts it.
        &&
          lib.all
            (
              one:
              one.node.Unit.X-Restart-Triggers == [
                (builtins.hashString "sha256" (linux both).xdg.configFile."mycelium/serves-${one.realm}.json".text)
              ]
            )
            [
              {
                node = bothLinux.mycelium;
                realm = "home";
              }
              {
                node = bothLinux.mycelium-other;
                realm = "other";
              }
            ]
        # On a Mac, launchd reloads an agent only when its own file
        # changes, so what the node serves is written into it.
        &&
          lib.all
            (
              realm:
              (mac both).launchd.agents.${
                if realm == "home" then "mycelium" else "mycelium-${realm}"
              }.config.EnvironmentVariables.MYCELIUM_SERVES or null
              == builtins.hashString "sha256" (mac both).xdg.configFile."mycelium/serves-${realm}.json".text
            )
            [
              "home"
              "other"
            ]
      );

  # Each of a machine's nodes carries in its machine record what the system
  # daemon says of the machine, so every machine of the realm sees how it
  # stands: on Linux from the runtime folder, and on a Mac, which has none,
  # from the account's state folder. A machine naming none carries nothing.
  mycelium-says-how-its-machine-stands =
    holds "mycelium-says-how-its-machine-stands"
      "A machine's node, in its home realm or a further one, isn't given the system daemon's socket where that machine's daemon serves, or a machine naming none is given one anyway."
      (
        let
          health = [
            "--health-socket"
            "%t/daemons/system.sock"
          ];
          linuxWords = service: lib.splitString " " (toString service.Service.ExecStart);
          macSocket = "${(mac links).xdg.stateHome}/daemons/system.sock";
        in
        lib.hasInfix (lib.concatStringsSep " " health) linuxNode
        && lib.hasInfix (lib.concatStringsSep " " health) otherNode
        && lib.hasInfix "--health-socket ${macSocket}" (toString macNode)
        && lib.hasInfix "--health-socket ${macSocket}" (
          toString (mac both).launchd.agents.mycelium-other.config.ProgramArguments
        )
        && !(lib.elem "--health-socket" (
          linuxWords (linux (links // { healthSocket = null; })).systemd.user.services.mycelium
        ))
        && lib.hasInfix "--health-socket /run/elsewhere.sock" (
          toString
            (linux (links // { healthSocket = "/run/elsewhere.sock"; }))
            .systemd.user.services.mycelium.Service.ExecStart
        )
      );

  # Each of a machine's nodes is given its realm's kit, the tools every
  # session and run it starts finds first on its path: the shared ones, and
  # whatever the machine adds, as one folder in the store, since its tools go
  # first for agents too and nothing an agent writes may change them.
  mycelium-carries-the-realms-kit =
    holds "mycelium-carries-the-realms-kit"
      "A machine's node, in its home realm or a further one, isn't given its realm's kit in the store last among its words, the kit lacks the shared tools (a curl speaking wss:// among them) or what the machine adds, it takes git or gh from an account that may wrap its own, or a machine emptying its kit gives one anyway."
      (
        let
          kitted =
            realm: words:
            builtins.match ".* --kit ${builtins.storeDir}/[a-z0-9]{32}-mycelium-kit-${realm}" words != null;
          # Tools by name, since the machines here are Linux ones whatever
          # system runs the checks, so a Mac's own packages never match.
          names = map lib.getName;
          # mycelium itself by what it is, since nixpkgs holds another program
          # of that name.
          shares =
            system: kit:
            lib.elem self.inputs.mycelium.packages.${system}.default kit
            && lib.all (tool: lib.elem tool (names kit)) [
              "jq"
              "ripgrep"
              "websocat"
            ]
            # A curl speaking wss://, which a Mac's websocat can't over TLS 1.3.
            && lib.any (
              tool: lib.getName tool == "curl" && lib.elem "--enable-websockets" (tool.configureFlags or [ ])
            ) kit
            && !(lib.any (tool: lib.elem tool (names kit)) [
              "gh"
              "git"
              "git-lfs"
            ]);
          adding = linux (
            both
            // {
              kit = [ pkgs.hello ];
              alsoJoins.other = {
                links = [ "two" ];
                kit = [ pkgs.cowsay ];
              };
            }
          );
        in
        kitted "personal" linuxNode
        && kitted "other" otherNode
        && kitted "personal" (toString macNode)
        && kitted "other" (toString (mac both).launchd.agents.mycelium-other.config.ProgramArguments)
        && shares "x86_64-linux" (linux links).jitsusama.mycelium.kit
        && shares "aarch64-darwin" (mac links).jitsusama.mycelium.kit
        && shares "x86_64-linux" (linux both).jitsusama.mycelium.alsoJoins.other.kit
        && shares "x86_64-linux" adding.jitsusama.mycelium.kit
        && lib.elem "hello" (names adding.jitsusama.mycelium.kit)
        && lib.elem "cowsay" (names adding.jitsusama.mycelium.alsoJoins.other.kit)
        && !(lib.elem "cowsay" (names adding.jitsusama.mycelium.kit))
        && !(lib.hasInfix "--kit" (
          toString
            (linux (links // { kit = lib.mkForce [ ]; })).systemd.user.services.mycelium.Service.ExecStart
        ))
      );

  # The tools of the kit a machine of this system is given build here and
  # hold what its sessions reach for, so a tool one system lacks fails on
  # that system's checks, the Mac's included, before a machine of it
  # switches. Core's checks run on these two systems alone. mycelium is
  # built without its tests, as for its flags above: its own gate runs them,
  # and their timing bars fail on a busy builder for nothing this proves.
  mycelium-kit-builds-here =
    let
      machine = if pkgs.stdenv.hostPlatform.isDarwin then mac else linux;
      untested =
        tool: if lib.getName tool == "mycelium" then tool.overrideAttrs { doCheck = false; } else tool;
      kit = pkgs.buildEnv {
        name = "mycelium-kit-here";
        paths = map untested (machine links).jitsusama.mycelium.kit;
        pathsToLink = [ "/bin" ];
      };
    in
    pkgs.runCommandLocal "mycelium-kit-builds-here" { } ''
      bash ${./tests/kit-builds-here.sh} ${kit}
      touch "$out"
    '';

  # A machine that declares what its node can't measure has it written where
  # mycelium reads it, in mycelium's words; Linux's held sessions outlive
  # their window, the node and a suspend, and a Mac claims none yet. A fenced
  # or strict fence names its system's launcher for an agent's run, which
  # runs pi's strict profile; whether every start goes through it is
  # mycelium's to keep, not this check's. An unfenced one says none, so mycelium
  # refuses it.
  # One that declares nothing has no file, which mycelium reads as unknown.
  mycelium-declares-what-a-machine-says =
    holds "mycelium-declares-what-a-machine-says"
      "What a machine declares, its probes of either kind included, isn't written in mycelium's shape, its node doesn't say whether a call is on there with a-call-is-on asked about that machine alone, a held session's outlives isn't each system's, a fence it doesn't say is written, whether pi fences itself isn't written only when a Mac says so beside its fences or isn't refused on Linux, doors it accepts for another machine aren't or take a machine no realm could name, an agent's run in a fenced or strict fence isn't said to start through its system's launcher, an unfenced one's is, one is when runs are refused, a machine of either system can't declare every fence, a machine kept lively doesn't say its ceiling, or the memory it keeps available only where it keeps some, or one not kept lively says one, or a machine declaring nothing gets a file anyway."
      (
        declared linux == {
          reach = [ "internet" ];
          probes.home.connect = "192.0.2.1:53";
          probes.office.run = declaring.probes.office;
          probes.call.run = (declared linux).probes.call.run;
          outlives = [
            "window_close"
            "node_restart"
            "sleep"
          ];
          costs_money = false;
          inherit (declaring) taints;
        }
        &&
          map baseNameOf (declared linux).probes.call.run == [
            "a-call-is-on"
            "--here"
          ]
        &&
          map baseNameOf (declared mac).probes.call.run == [
            "a-call-is-on"
            "--here"
          ]
        && (declared mac).outlives == [ ]
        && !((declared linux) ? within)
        && !((declared linux) ? hosts)
        && !((declared linux) ? fence)
        && !((declared linux) ? pi_fences_itself)
        && !((declared linux) ? kept_lively)
        && (declaredAs { keptLively.cpu = 28; }).kept_lively or null == { cpu = 28; }
        && (
          (declaredAs {
            keptLively = {
              cpu = 28;
              memoryAvailable = 1024;
            };
          }).kept_lively or null == {
            cpu = 28;
            memory_available = 1024;
          }
        )
        && !((launching mac everyKind) ? pi_fences_itself)
        && (saidBy (selfFenced mac)).pi_fences_itself or false
        && (saidBy (selfFenced mac)).fence.strict.runs or null == macLauncher
        && evaluates (selfFenced mac)
        && refusesSelfFencing (selfFenced linux)
        && !refusesSelfFencing (fencedIn linux everyKind { })
        && (
          strictly.fence or null == {
            strict = {
              builders = [ "remote" ];
              forges = [ ];
              secrets = [ ];
              runs = launcher;
            };
          }
        )
        && (
          accepting.fence.none.accepts or null == {
            other-machine = {
              builders = [ ];
              forges = [ ];
              secrets = [ "tokens" ];
            };
          }
        )
        && !misnamed.success
        && (misnamedIf false).success
        && !(misnamedIf true).success
        && (launching linux everyKind).fence.fence.runs or null == launcher
        && (launching linux everyKind).fence.strict.runs or null == launcher
        && !((launching linux everyKind).fence.none ? runs)
        && (launching mac everyKind).fence.fence.runs or null == macLauncher
        && (launching mac everyKind).fence.strict.runs or null == macLauncher
        && !((launching mac everyKind).fence.none ? runs)
        && evaluates (fencedIn mac everyKind { })
        && evaluates (fencedIn linux everyKind { })
        && lib.all (kind: !((launchingWith linux everyKind { fenceRuns = null; }).fence.${kind} ? runs)) [
          "fence"
          "strict"
        ]
        && (
          inVm.within or null == {
            machine = "host-machine";
            file = "/run/host/said.json";
          }
        )
        && (
          hosting.hosts or null == [
            {
              machine = "guest-machine";
              file = "/var/host/said.json";
            }
          ]
        )
        && !(apart.xdg.configFile ? "mycelium/declared.json")
        &&
          (read (saying { home = "an-agent"; }).xdg.configFile."mycelium/declared.json".text).agents == {
            home = "an-agent";
          }
        && !refusesAgents (saying {
          home = "an-agent";
        })
        && refusesAgents (saying {
          elsewhere = "an-agent";
        })
      );

  # Each realm's node on either system reads what its machine declares from
  # the store, where only root writes, never from the account's config
  # folder, whose link anything running as the account could repoint; it
  # holds the very words the account's commands read there. A machine
  # declaring nothing names no file.
  mycelium-node-reads-its-declaration-from-the-store =
    holds "mycelium-node-reads-its-declaration-from-the-store"
      "A node of a machine that declares itself, on NixOS or nix-darwin and in each realm it joins, isn't given the store file holding what its config folder's declared.json says, or one declaring nothing is given a file."
      (
        let
          declaringLinks = both // {
            declares = declaring;
          };
          # Each machine whole, for its own pkgs, and its account.
          machine = {
            linux = declaringOn bareNixos self.nixosModules.workstation;
            mac = declaringOn bareDarwin self.darwinModules.workstation;
          };
          declaringOn =
            bare: role:
            bare [
              role
              { home-manager.users.someone.jitsusama.mycelium = declaringLinks; }
            ];
          home = lib.mapAttrs (_: account) machine;
          onLinux = home.linux.systemd.user.services;
          onMac = home.mac.launchd.agents;
          # The word after --declared, as the unit has it.
          given =
            words:
            let
              at = lib.lists.findFirstIndex (word: word == "--declared") null words;
            in
            if at == null then null else builtins.elemAt words (at + 1);
          linuxWords = service: lib.splitString " " (toString service.Service.ExecStart);
          # The store file, built as the machine's own, holding what its
          # account's config folder says.
          holding =
            system:
            let
              words = home.${system}.xdg.configFile."mycelium/declared.json".text;
            in
            "${machine.${system}.pkgs.writeText "mycelium-declared.json" words}";
        in
        given (linuxWords onLinux.mycelium) == holding "linux"
        && given (linuxWords onLinux.mycelium-other) == holding "linux"
        && given onMac.mycelium.config.ProgramArguments == holding "mac"
        && given onMac.mycelium-other.config.ProgramArguments == holding "mac"
        && given (linuxWords bothLinux.mycelium) == null
        && given (linuxWords bothLinux.mycelium-other) == null
        && given macNode == null
        && given (mac both).launchd.agents.mycelium-other.config.ProgramArguments == null
      );

  # The person's table of which receptors speak reaches every realm's node
  # from the store, as the declaration does, since the node refuses a table
  # any account but root could write. A machine naming no source names no
  # file, so every source stays silent, and a source mycelium has no
  # receptor for is refused.
  mycelium-node-reads-its-stimuli-from-the-store =
    holds "mycelium-node-reads-its-stimuli-from-the-store"
      "A node of a machine naming sources in its table, on NixOS or nix-darwin and in each realm it joins, isn't given the store file saying those sources, one naming none is given a file, or a source mycelium has no receptor for is taken."
      (
        let
          said = [
            "github"
            "slack"
          ];
          stimulating = both // {
            stimuli = said;
          };
          # Each machine whole, for its own pkgs, and its account.
          machine = {
            linux = stimulatingOn bareNixos self.nixosModules.workstation;
            mac = stimulatingOn bareDarwin self.darwinModules.workstation;
          };
          stimulatingOn =
            bare: role:
            bare [
              role
              { home-manager.users.someone.jitsusama.mycelium = stimulating; }
            ];
          home = lib.mapAttrs (_: account) machine;
          onLinux = home.linux.systemd.user.services;
          onMac = home.mac.launchd.agents;
          # The word after --stimuli, as the unit has it.
          given =
            words:
            let
              at = lib.lists.findFirstIndex (word: word == "--stimuli") null words;
            in
            if at == null then null else builtins.elemAt words (at + 1);
          linuxWords = service: lib.splitString " " (toString service.Service.ExecStart);
          # The store file, built as the machine's own, saying the sources.
          holding =
            system:
            "${machine.${system}.pkgs.writeText "mycelium-stimuli.json" (builtins.toJSON { say = said; })}";
          # A node whose table names a source mycelium doesn't know, forced
          # whole so the option's type is what refuses it.
          unknown =
            let
              words =
                toString
                  (linux (both // { stimuli = [ "pager" ]; })).systemd.user.services.mycelium.Service.ExecStart;
            in
            builtins.tryEval (builtins.deepSeq words words);
        in
        given (linuxWords onLinux.mycelium) == holding "linux"
        && given (linuxWords onLinux.mycelium-other) == holding "linux"
        && given onMac.mycelium.config.ProgramArguments == holding "mac"
        && given onMac.mycelium-other.config.ProgramArguments == holding "mac"
        && given (linuxWords bothLinux.mycelium) == null
        && given (linuxWords bothLinux.mycelium-other) == null
        && given macNode == null
        && given (mac both).launchd.agents.mycelium-other.config.ProgramArguments == null
        && !unknown.success
      );

  # What a machine lends reaches its home realm's node from the store, as the
  # declaration does: an agent runs as the account, so a file in its config
  # folder could raise a machine's cap or send its requests elsewhere. Its
  # names are the locked mycelium's own, read from its source, since the
  # node refuses a file with a name it doesn't know, and its budget kinds
  # are the lender's words, so a kind core writes is one it takes. A lend
  # leaving its defaults unsaid is written with them. A budget with no key
  # says none, a further realm's node lends nothing, and what mycelium
  # would refuse or read wider than meant is refused as the machine builds.
  mycelium-lends-from-the-store =
    holds "mycelium-lends-from-the-store"
      "A lending NixOS machine doesn't evaluate, its home realm node isn't given its lend as a store file holding it, the lend's fields aren't the locked mycelium's own by name and type, from structs serde reads plainly and the lender parses the file into, the lender doesn't join a relative upstream onto its runtime folder, the struct scan takes a struct serde reads otherwise, the kind scan takes an enum serde reads otherwise, the budget kinds lend.nix takes or its default aren't the words serde takes for the lender's Kind, a lend leaving a machine's also and a budget's kind unsaid, beside a forge budget a machine names, isn't written with their defaults, a lend capping no spend and pricing no model isn't written without them and warned of, its prices aren't read as defaulted, a budget naming no spend-watch budget or pricing none isn't warned of, a priced one or a machine lending only a forge is warned of, a model priced at nothing is taken, a budget with no key says one, a further realm's node or a machine lending nothing is given a lend, a Mac's lend isn't refused and kept from its node, a lend with plain prefixes, dotted paths or a key package is refused, or a lend with no realm, naming a budget it lacks, with overlapping or empty prefixes, a key outside the store, climbing out of it, named as text or empty, a whole upstream or one with a dot part, a host with a scheme, a header with a space, a budget of a kind the lender has none of, or a path lending everything or nothing is taken."
      (
        let
          # The machine whole, for its own pkgs, and its account.
          machine = bareNixos [
            self.nixosModules.workstation
            {
              home-manager.users.someone.jitsusama.mycelium = both // {
                lend = lending;
              };
            }
          ];
          home = account machine;
          # A Mac lending, which only Linux may.
          onMac = mac (both // { lend = lending; });
          macRefusal = "jitsusama.mycelium.lend is set on a Mac";
          refusesOnMac = check: !check.assertion && lib.hasPrefix macRefusal check.message;
          # The word after --lend, of the unit's words.
          after =
            words:
            let
              at = lib.lists.findFirstIndex (word: word == "--lend") null words;
            in
            if at == null then null else builtins.elemAt words (at + 1);
          given = node: after (lib.splitString " " (toString node.Service.ExecStart));
          # The store file, built as the machine's own: the budget with no
          # key is written with none.
          holding = "${machine.pkgs.writeText "mycelium-lend.json" (builtins.toJSON lending)}";
          source = builtins.readFile "${self.inputs.mycelium}/src/lend/lender/mod.rs";
          lines = lib.splitString "\n" source;
          # Whether the lender's lines hold the first of WANTED, and the rest
          # in order within SIZE lines after it.
          reads =
            size: wanted:
            let
              at = lib.lists.findFirstIndex (line: line == lib.head wanted) null lines;
              follows =
                rest: from:
                let
                  next = lib.lists.findFirstIndex (line: line == lib.head rest) null from;
                in
                rest == [ ] || next != null && follows (lib.tail rest) (lib.drop (next + 1) from);
            in
            at != null && follows (lib.tail wanted) (lib.take size (lib.drop (at + 1) lines));
          # The lender joins a relative upstream onto the runtime folder
          # place::login_runtime finds, refusing the request where there is
          # none, which is all that keeps the type's relative upstream in
          # the account's own folder; and it parses the file as Written,
          # the struct the scan reads.
          joins =
            reads 14 [
              "    let upstream = if budget.upstream.is_absolute() {"
              "    } else {"
              "        lender"
              "            .runtime"
              "            .join(&budget.upstream)"
            ]
            && lib.hasInfix "crate::place::login_runtime())" source;
          parsing = ".map_err(|error| unparsed(&error)))";
          parses = reads 3 [
            "    fn written(&self) -> Result<Written, Refusal> {"
            ("            .and_then(|text| serde_json::from_slice(&text)" + parsing)
          ];
          # The fields of struct NAME in LINES, each by its name to its
          # type. serde reads them so only while the struct derives
          # Deserialize with nothing but deny_unknown_fields over it, and no
          # field says more than default. So the scan reads every attribute
          # and doc line above the struct, back to a blank line or the end
          # of another item, and fails on any other line there or in the
          # body, on anything but that end past the blank lines, and on a
          # struct no longer found, rather than passing what it didn't read.
          # Past any blank lines over an item's attributes stands the end of
          # the item before, a line ending ';' or the file's start: never a
          # comment or an attribute left unread.
          ended =
            past:
            let
              gap = lib.lists.findFirstIndex (line: line != "") (lib.length past) past;
              line = lib.elemAt past gap;
            in
            gap == lib.length past
            || line == "}"
            || lib.hasSuffix ";" line && builtins.match "[[:space:]]*[/#].*" line == null;
          # Each scan answers what it read as read, or why it can't read it
          # as failed, so a case it refuses shows which refusal it met.
          reading =
            scanned: scanned.read or (throw "mycelium's ${scanned.failed}, which this check can't read");
          scan = lines: struct: reading (structScan lines struct);
          refusedFor =
            scanner:
            { lines, why }:
            lib.hasInfix why ((scanner lines).failed or "");
          structScan =
            lines: struct:
            let
              from = lib.lists.findFirstIndex (line: line == "struct ${struct} {") null lines;
              rest = lib.drop (from + 1) lines;
              to = lib.lists.findFirstIndex (line: line == "}") null rest;
              body = lib.take to rest;
              marked = line: lib.hasPrefix "#[" line || lib.hasPrefix "///" line;
              before = lib.reverseList (lib.take from lines);
              reach = lib.lists.findFirstIndex (line: !marked line) (lib.length before) before;
              above = lib.take reach before;
              # A derive naming Deserialize itself, not one named like it.
              derive = "#\\[derive\\(([A-Za-z]+, )*Deserialize(, [A-Za-z]+)*\\)]";
              deriving = line: builtins.match derive line != null;
              told = line: lib.hasPrefix "///" line || deriving line || line == denying;
              # Each body line a doc line, the one field attribute serde
              # may carry, or a field of one line.
              field = builtins.match "    ([a-z_][a-z0-9_]*): (.*),";
              attribute = "    #[serde(default)]";
              plain = line: lib.hasPrefix "    ///" line || line == attribute || field line != null;
              fail = why: { failed = "struct ${struct} ${why}"; };
            in
            if from == null || to == null then
              fail "has no lines 'struct ${struct} {' to '}'"
            else if !ended (lib.drop reach before) then
              fail "has over its attributes, past blank lines, other than an item's end"
            else if
              !lib.all told above
              || !lib.any deriving above
              || lib.length (lib.filter (line: line == denying) above) != 1
            then
              fail "has over it other than doc lines, a Deserialize derive and deny_unknown_fields"
            else if !lib.all plain body then
              fail "has a line in its body other than a doc line, a field or #[serde(default)]"
            else
              {
                # Each field serde reads as its type's default where the
                # file leaves it out: one whose nearest line above it, past
                # its doc lines, is the one attribute a field may carry.
                defaulted = lib.concatLists (
                  lib.imap0 (
                    at: line:
                    let
                      matched = field line;
                      over = lib.filter (above: !lib.hasPrefix "    ///" above) (lib.take at body);
                    in
                    lib.optional (matched != null && over != [ ] && lib.last over == attribute) (lib.head matched)
                  ) body
                );
                read = lib.listToAttrs (
                  lib.concatMap (
                    line:
                    let
                      matched = field line;
                    in
                    if matched == null then
                      [ ]
                    else
                      [
                        {
                          name = lib.head matched;
                          value = lib.last matched;
                        }
                      ]
                  ) body
                );
              };
          fields = scan lines;
          named = struct: lib.attrNames (fields struct);
          defaults = struct: (structScan lines struct).defaulted or [ ];
          # A struct laid out as the lender's are, which the scan reads,
          # and with each change that has serde read it otherwise, or that
          # the scan can't read, in place of its line AT.
          plainVm = [
            "/// A machine lent to."
            "#[derive(Debug, Deserialize)]"
            denying
            "struct Vm {"
            "    /// Its name's start."
            "    prefix: String,"
            "    #[serde(default)]"
            "    requests_a_day: u64,"
            "}"
          ];
          plainFields = {
            prefix = "String";
            requests_a_day = "u64";
          };
          denying = "#[serde(deny_unknown_fields)]";
          changedVm = at: lines: lib.take at plainVm ++ lines ++ lib.drop (at + 1) plainVm;
          # The refusals, each by words of its own reason.
          pastItem = "past blank lines, other than an item's end";
          overStruct = "other than doc lines, a Deserialize derive and deny_unknown_fields";
          inBody = "has a line in its body other than a doc line";
          noLines = "has no lines";
          # Ends of the item before that the scan reads past: one ending
          # ';' and one ending '}', with more than one blank line after.
          endedVms = [
            (
              [
                "const MOST: u64 = 1;"
                ""
              ]
              ++ plainVm
            )
            (
              [
                "}"
                ""
                ""
              ]
              ++ plainVm
            )
          ];
          unread = [
            {
              lines = changedVm 0 [
                "#[serde(rename_all = \"kebab-case\")]"
                ""
              ];
              why = pastItem;
            }
            {
              lines = changedVm 0 [
                "#[serde(rename_all = \"kebab-case\")]"
                "// A machine lent to."
                ""
              ];
              why = pastItem;
            }
            {
              lines = changedVm 0 [
                "#[serde(rename_all = \"kebab-case\")]"
                "  // Its name; its cap;"
                ""
              ];
              why = pastItem;
            }
            {
              lines = changedVm 0 [
                "#[serde(rename_all = \"kebab-case\")]"
                ""
                ""
              ];
              why = pastItem;
            }
            # An item's end opening with an attribute, read as one left
            # unread, so the scan refuses rather than guess.
            {
              lines = [
                "#[cfg(test)] mod tests;"
                ""
              ]
              ++ plainVm;
              why = pastItem;
            }
            {
              lines = changedVm 1 [ "#[derive(Debug, DeserializeFromStr)]" ];
              why = overStruct;
            }
            # Each line over these the scan tells, so only the missing
            # derive, or the count of deny_unknown_fields, refuses it.
            {
              lines = changedVm 1 [ ];
              why = overStruct;
            }
            {
              lines = changedVm 2 [ ];
              why = overStruct;
            }
            {
              lines = changedVm 2 [
                denying
                denying
              ];
              why = overStruct;
            }
            {
              lines = changedVm 2 [ "#[serde(deny_unknown_fields, rename_all = \"kebab-case\")]" ];
              why = overStruct;
            }
            {
              lines = changedVm 2 [
                denying
                "#[cfg_attr(test, serde(rename_all = \"camelCase\"))]"
              ];
              why = overStruct;
            }
            {
              lines = changedVm 3 [ "pub struct Vm {" ];
              why = noLines;
            }
            {
              lines = changedVm 5 [ "    pub prefix: String," ];
              why = inBody;
            }
            {
              lines = changedVm 5 [
                "    prefix:"
                "        String,"
              ];
              why = inBody;
            }
            {
              lines = changedVm 6 [ "    #[serde(rename = \"name\")]" ];
              why = inBody;
            }
          ];
          # Each field's JSON shape as the module writes it: a keyless
          # budget with no key, read as none only while the key is
          # optional, a key and paths as lists of words, a daily cap any
          # positive Nix integer fits, and prices and spend any Nix number
          # fits, each optional field read as none where it is left out.
          shapes = {
            Written = {
              vms = "Vec<Vm>";
              budgets = "BTreeMap<String, Budget>";
            };
            Vm = {
              prefix = "String";
              budget = "String";
              also = "Vec<String>";
              requests_a_day = "u64";
              usd_a_day = "Option<f64>";
            };
            Budget = {
              upstream = "PathBuf";
              host = "String";
              paths = "Vec<String>";
              auth_header = "String";
              key = "Option<Vec<String>>";
              kind = "Kind";
              prices = "BTreeMap<String, Price>";
              spend_watch = "Option<String>";
            };
            # Prices any Nix number fits, and a most any positive integer.
            Price = {
              input = "f64";
              output = "f64";
              cache_write = "Option<f64>";
              cache_read = "Option<f64>";
              most_output = "u64";
            };
          };
          # Refused by the assertion whose message starts so, and by it.
          assertsAgainst =
            settings: start:
            lib.any (check: !check.assertion && lib.hasPrefix start check.message) (linux settings).assertions;
          refuses = lend: start: assertsAgainst (both // { inherit lend; }) start;
          # Refused by the option's type, which fails the node's words.
          typeRefuses =
            lend:
            let
              words =
                toString
                  (linux (both // { inherit lend; })).systemd.user.services.mycelium.Service.ExecStart;
            in
            !(builtins.tryEval (builtins.deepSeq words words)).success;
          withModels =
            change:
            lending
            // {
              budgets = lending.budgets // {
                models = lending.budgets.models // change;
              };
            };
          # Taken by the option's type and every assertion.
          takes =
            lend:
            !typeRefuses lend
            && lib.all (check: check.assertion) (linux (both // { inherit lend; })).assertions;
          vm = lib.head lending.vms;
          # The words serde takes for enum Kind in LINES and the one it
          # takes where none is written: each variant lowercased and the
          # variant marked default. serde reads them so only while the enum
          # derives Deserialize with nothing but one lowercase rename over
          # it, and its body holds only plain variants. So, as the struct
          # scan does, it reads every attribute and doc line above the enum,
          # back to a blank line or another item's end, and fails on any
          # other line there or in the body, rather than passing what it
          # didn't read.
          kindScan =
            lines:
            let
              from = lib.lists.findFirstIndex (line: line == "enum Kind {") null lines;
              rest = lib.drop (from + 1) lines;
              to = lib.lists.findFirstIndex (line: line == "}") null rest;
              body = lib.filter (line: !lib.hasPrefix "    ///" line) (lib.take to rest);
              marked = line: lib.hasPrefix "#[" line || lib.hasPrefix "///" line;
              before = lib.reverseList (lib.take from lines);
              reach = lib.lists.findFirstIndex (line: !marked line) (lib.length before) before;
              above = lib.take reach before;
              derive = "#\\[derive\\(([A-Za-z]+, )*Deserialize(, [A-Za-z]+)*\\)]";
              deriving = line: builtins.match derive line != null;
              renaming = "#[serde(rename_all = \"lowercase\")]";
              told = line: lib.hasPrefix "///" line || deriving line || line == renaming;
              count = wanted: lib.length (lib.filter wanted above);
              variant = builtins.match "    ([A-Z][A-Za-z]*),";
              defaulted = "    #[default]";
              at = lib.lists.findFirstIndex (line: line == defaulted) null body;
              fail = why: { failed = "enum Kind ${why}"; };
            in
            if from == null || to == null then
              fail "has no lines 'enum Kind {' to '}'"
            else if !ended (lib.drop reach before) then
              fail "has over its attributes, past blank lines, other than an item's end"
            else if !lib.all told above || count deriving != 1 || count (l: l == renaming) != 1 then
              fail "has over it other than doc lines, a Deserialize derive and a lowercase rename"
            else if !lib.all (line: line == defaulted || variant line != null) body then
              fail "has a line in its body other than a doc line, a plain variant or #[default]"
            else if at == null || at + 1 == lib.length body || variant (lib.elemAt body (at + 1)) == null then
              fail "has no variant marked default"
            else
              {
                read = {
                  words = map (line: lib.toLower (lib.head (variant line))) (
                    lib.filter (line: variant line != null) body
                  );
                  unwritten = lib.toLower (lib.head (variant (lib.elemAt body (at + 1))));
                };
              };
          kinds = reading (kindScan lines);
          # An enum laid out as the lender's Kind is, which the scan reads,
          # and with each change that has serde read it otherwise, or that
          # the scan can't read, in place of its line AT.
          plainKind = [
            "/// What a budget's upstream is."
            "#[derive(Debug, Default, Deserialize)]"
            "#[serde(rename_all = \"lowercase\")]"
            "enum Kind {"
            "    /// A model relay."
            "    #[default]"
            "    Models,"
            "    Forge,"
            "}"
          ];
          plainKinds = {
            words = [
              "models"
              "forge"
            ];
            unwritten = "models";
          };
          changedKind = at: lines: lib.take at plainKind ++ lines ++ lib.drop (at + 1) plainKind;
          overKind = "other than doc lines, a Deserialize derive and a lowercase rename";
          noDefault = "has no variant marked default";
          unreadKinds = [
            {
              lines = changedKind 0 [
                "#[cfg_attr(test, serde(rename_all = \"UPPERCASE\"))]"
                ""
                "/// What a budget's upstream is."
              ];
              why = pastItem;
            }
            {
              lines = changedKind 0 [
                "#[serde("
                "    deny_unknown_fields,"
                ")]"
              ];
              why = pastItem;
            }
            {
              lines = changedKind 1 [ "#[derive(Debug, Default, DeserializeFromStr)]" ];
              why = overKind;
            }
            # Two derives, each read alone as serde's, which only the count
            # of them refuses.
            {
              lines = changedKind 1 [
                "#[derive(Debug, Default, Deserialize)]"
                "#[derive(Deserialize)]"
              ];
              why = overKind;
            }
            {
              lines = changedKind 2 [ "#[serde(rename_all = \"UPPERCASE\")]" ];
              why = overKind;
            }
            {
              lines = changedKind 2 [ ];
              why = overKind;
            }
            {
              lines = changedKind 2 [
                "#[serde(rename_all = \"lowercase\")]"
                "#[cfg_attr(test, serde(rename_all = \"UPPERCASE\"))]"
              ];
              why = overKind;
            }
            {
              lines = changedKind 3 [ "pub enum Kind {" ];
              why = noLines;
            }
            {
              lines = changedKind 5 [ ];
              why = noDefault;
            }
            # The mark on the body's last line, with no variant after it.
            {
              lines = lib.take 5 plainKind ++ [
                "    Models,"
                "    Forge,"
                "    #[default]"
                "}"
              ];
              why = noDefault;
            }
            {
              lines = changedKind 7 [ "    Forge(String)," ];
              why = inBody;
            }
            {
              lines = changedKind 7 [
                "    #[serde(rename = \"git\")]"
                "    Forge,"
              ];
              why = inBody;
            }
          ];
          # lend.nix's budget kind option, as the module takes it.
          kindOption =
            let
              inherit ((import ./lend.nix { inherit lib; }).getSubOptions [ ]) budgets;
            in
            (budgets.type.nestedTypes.elemType.getSubOptions [ ]).kind;
          # A lend leaving also and kind to their defaults, with a forge
          # budget one machine names beside its models, and the lend the
          # module must write of it, those defaults filled in as the
          # lender would take them.
          forge = {
            upstream = "forge-relay/agents.sock";
            host = "forge.example";
            paths = [ "/git/*" ];
            auth_header = "authorization";
          };
          forging = vm // {
            prefix = "forgers-";
            also = [ "forge" ];
          };
          unsaid = lending // {
            vms = [
              (removeAttrs vm [ "also" ])
              forging
            ];
            budgets = lib.mapAttrs (_: budget: removeAttrs budget [ "kind" ]) lending.budgets // {
              forge = forge // {
                kind = "forge";
              };
            };
          };
          filled = lending // {
            vms = [
              vm
              forging
            ];
            budgets = lending.budgets // {
              forge = forge // {
                kind = "forge";
              };
            };
          };
          writes =
            lend: written:
            given (linux (both // { inherit lend; })).systemd.user.services.mycelium
            == "${machine.pkgs.writeText "mycelium-lend.json" (builtins.toJSON written)}";
          sorted = lib.sort lib.lessThan;
          # A lend capping no machine's spend and pricing no model: written
          # without them, which mycelium reads as lending no model, and
          # warned of rather than refused, so the machine still evaluates.
          uncapped = lending // {
            vms = [ (removeAttrs vm [ "usd_a_day" ]) ];
            budgets = lending.budgets // {
              models = removeAttrs lending.budgets.models [ "prices" ];
            };
          };
          warnings = lend: (linux (both // { inherit lend; })).warnings;
          warnsOf = lend: start: lib.any (lib.hasPrefix start) (warnings lend);
          warns = warnsOf uncapped;
          # A models budget pricing models but naming no spend-watch budget,
          # and one pricing none, each refused by mycelium as one pricing
          # nothing is.
          unwatched = lending // {
            budgets = lending.budgets // {
              models = removeAttrs lending.budgets.models [ "spend_watch" ];
            };
          };
          unpriced = lending // {
            budgets = lending.budgets // {
              models = lending.budgets.models // {
                prices = { };
              };
            };
          };
          # A lend whose every models budget is priced and watched, and whose
          # machine is capped.
          priced = lending // {
            budgets = removeAttrs lending.budgets [ "keyless" ];
          };
          # A machine lending models only beside a forge, uncapped.
          alsoModels = lending // {
            vms = [
              (
                removeAttrs vm [ "usd_a_day" ]
                // {
                  budget = "forge";
                  also = [ "models" ];
                }
              )
            ];
            budgets = lending.budgets // {
              forge = forge // {
                kind = "forge";
              };
            };
          };
          # A model priced without its cache's prices, which are left out.
          cacheUnpriced = withModels {
            prices.a-model = removeAttrs lending.budgets.models.prices.a-model [
              "cache_write"
              "cache_read"
            ];
          };
          # A machine lending only a forge, whose spend no dollar caps.
          forgeOnly = lending // {
            vms = [ (removeAttrs vm [ "usd_a_day" ] // { budget = "forge"; }) ];
            budgets = lending.budgets // {
              forge = forge // {
                kind = "forge";
              };
            };
          };
          # A model with a price of nothing, refused as the machine builds,
          # since it would spend past any daily cap: each price alike.
          freeOf = price: {
            budgets = lending.budgets // {
              models = lending.budgets.models // {
                prices.a-model = lending.budgets.models.prices.a-model // {
                  ${price} = 0;
                };
              };
            };
          };
          budgetWarned = "jitsusama.mycelium.lend.budgets.models names no prices";
          vmWarned = "jitsusama.mycelium.lend.vms'";
          lendWarned = lend: lib.filter (lib.hasPrefix "jitsusama.mycelium.lend") (warnings lend);
        in
        evaluates home
        && given home.systemd.user.services.mycelium == holding
        && given home.systemd.user.services.mycelium-other == null
        && given bothLinux.mycelium == null
        && lib.any refusesOnMac onMac.assertions
        && after onMac.launchd.agents.mycelium.config.ProgramArguments == null
        && joins
        && parses
        && scan plainVm "Vm" == plainFields
        && lib.all (lines: scan lines "Vm" == plainFields) endedVms
        && lib.all (refusedFor (lines: structScan lines "Vm")) unread
        && named "Written" == lib.attrNames lending
        && named "Vm" == lib.attrNames vm
        && named "Budget" == lib.attrNames lending.budgets.models
        && named "Price" == lib.attrNames lending.budgets.models.prices.a-model
        # The module leaves out a budget's prices where it has none, and
        # serde reads every other field it leaves out, each an Option, as
        # none without being told.
        && lib.elem "prices" (defaults "Budget")
        # Only the field whose own line above is the attribute, past its
        # doc lines, is read as defaulted.
        && (structScan plainVm "Vm").defaulted == [ "requests_a_day" ]
        &&
          (structScan (changedVm 6 [
            "    #[serde(default)]"
            "    /// Counted a day."
          ]) "Vm").defaulted == [ "requests_a_day" ]
        &&
          (structScan (changedVm 7 [
            "    requests_a_day: u64,"
            "    most: u64,"
          ]) "Vm").defaulted == [ "requests_a_day" ]
        && (structScan (changedVm 6 [ ]) "Vm").defaulted == [ ]
        && lib.all (struct: fields struct == shapes.${struct}) (lib.attrNames shapes)
        && sorted kindOption.type.functor.payload.values == sorted kinds.words
        && kindOption.default == kinds.unwritten
        && takes unsaid
        && writes unsaid filled
        && writes uncapped uncapped
        && warns "jitsusama.mycelium.lend.vms' agents- names no usd_a_day"
        && warns "${budgetWarned},"
        && warnsOf unwatched "jitsusama.mycelium.lend.budgets.models names no spend_watch,"
        && warnsOf unpriced "${budgetWarned},"
        && warnsOf alsoModels "jitsusama.mycelium.lend.vms' agents- names no usd_a_day"
        && !warnsOf forgeOnly vmWarned
        && !warnsOf forgeOnly "jitsusama.mycelium.lend.budgets.forge"
        # Each lend warned of still meets every assertion, so its machine
        # builds and lends what it can.
        && lib.all (lend: evaluates (linux (both // { inherit lend; }))) [
          uncapped
          unwatched
          unpriced
          alsoModels
          forgeOnly
        ]
        &&
          lendWarned lending == [
            (
              "jitsusama.mycelium.lend.budgets.keyless names no prices and no spend_watch, "
              + "so mycelium refuses every model lent through it."
            )
          ]
        && lendWarned priced == [ ]
        && writes cacheUnpriced cacheUnpriced
        && lib.all (price: typeRefuses (lending // freeOf price)) [
          "input"
          "output"
          "cache_write"
          "cache_read"
        ]
        && reading (kindScan plainKind) == plainKinds
        && lib.all (refusedFor kindScan) unreadKinds
        # A kind the type takes, beside one it refuses, so the refusal is
        # the word's alone.
        && takes (withModels {
          kind = "models";
        })
        && typeRefuses (withModels {
          kind = "Forge";
        })
        && takes (
          lending
          // {
            vms = [
              vm
              (vm // { prefix = "builders-"; })
            ];
          }
        )
        && takes (withModels {
          paths = [
            "/v1"
            "/v1/files/*"
            "/v1/files/a.json"
          ];
        })
        && assertsAgainst { lend = lending; } "jitsusama.mycelium.lend needs jitsusama.mycelium.realm"
        && refuses (
          lending // { vms = [ (vm // { budget = "none"; }) ]; }
        ) "jitsusama.mycelium.lend.vms names a budget"
        && refuses (
          lending // { vms = [ (vm // { also = [ "none" ]; }) ]; }
        ) "jitsusama.mycelium.lend.vms names a budget"
        && takes (lending // { vms = [ (vm // { also = [ "keyless" ]; }) ]; })
        && refuses (
          lending
          // {
            vms = [
              vm
              (vm // { prefix = "agents-big-"; })
            ];
          }
        ) "jitsusama.mycelium.lend.vms has two prefixes"
        && refuses (
          lending
          // {
            vms = [
              (vm // { prefix = "agents-big-"; })
              vm
            ];
          }
        ) "jitsusama.mycelium.lend.vms has two prefixes"
        && refuses (
          lending
          // {
            vms = [
              vm
              vm
            ];
          }
        ) "jitsusama.mycelium.lend.vms has two prefixes"
        # A prefix that would match every machine.
        && typeRefuses (lending // { vms = [ (vm // { prefix = ""; }) ]; })
        && typeRefuses (withModels {
          upstream = "/run/relay/agents.sock";
        })
        && typeRefuses (withModels {
          upstream = "./agents.sock";
        })
        && typeRefuses (withModels {
          host = "https://models.example";
        })
        && typeRefuses (withModels {
          auth_header = "Authorization: Bearer";
        })
        && typeRefuses (withModels {
          key = [ ];
        })
        && typeRefuses (withModels {
          key = [ "/tmp/models-key" ];
        })
        && typeRefuses (withModels {
          key = [ "${modelsKey}/../../tmp/models-key" ];
        })
        # A store path named as text, which no package keeps alive.
        && typeRefuses (withModels {
          key = [ "${builtins.storeDir}/00000000000000000000000000000000-models-key-1.0" ];
        })
        && typeRefuses (withModels {
          paths = [ "/*" ];
        })
        && typeRefuses (withModels {
          paths = [ "/v1/../admin" ];
        })
        && typeRefuses (withModels {
          upstream = "model-relay/../../agents.sock";
        })
        && typeRefuses (withModels {
          paths = [ "/" ];
        })
      );

  # The sources a table may name are the locked mycelium's own, read from its
  # source, since a name the node no longer knows makes it refuse the whole
  # table and every source falls silent with nothing failing.
  mycelium-table-names-mycelium-s-own-sources =
    holds "mycelium-table-names-mycelium-s-own-sources"
      "The sources a machine's stimuli table may name aren't the ones the locked mycelium's receptors read, as its Source::name names them."
      (
        let
          lines = lib.splitString "\n" (builtins.readFile "${self.inputs.mycelium}/src/receptor.rs");
          named = lib.concatMap (
            line:
            let
              matched = builtins.match " *Self::[A-Za-z]+ => \"([a-z]+)\"," line;
            in
            if matched == null then [ ] else matched
          ) lines;
          sorted = lib.sort lib.lessThan;
        in
        named != [ ] && sorted (import ./sources.nix) == sorted named
      );

  # Agents of an account apart from the node's reach the home realm's node
  # through a socket of their own, in a folder made at boot for the node's
  # account alone to write, since the node refuses one another account
  # could change; a further realm's node answers no such agents, a machine
  # naming no account serves none, and a Mac, whose system makes no such
  # folder, an account naming no realm, and a system that makes the folder
  # for another account, are each refused for that alone.
  mycelium-serves-agents-of-another-account =
    holds "mycelium-serves-agents-of-another-account"
      "A workstation naming its agents' account doesn't give the home realm's node their socket in /run/mycelium with that account, gives one to a further realm's node or a machine naming none, a Mac or an account naming no realm takes one or is refused for something else, NixOS doesn't make /run/mycelium for the node's account alone, or takes a node whose account it doesn't make it for."
      (
        let
          socket = lib.escapeShellArgs [
            "--worker-socket"
            "/run/mycelium/home-worker.sock"
            "--worker-uid"
            "1001"
          ];
          folderOn =
            settings:
            (bareNixos [
              self.nixosModules.workstation
              { jitsusama.mycelium = settings; }
            ]).config.systemd.tmpfiles.rules;
          # Whether `home` is refused, and only for naming agents apart.
          refusedForAgentsAlone =
            home:
            !evaluates home
            && lib.all (
              check: check.assertion || lib.hasPrefix "jitsusama.mycelium.agentsUid needs" check.message
            ) home.assertions;
          # What a NixOS system whose account `someone` names agents apart,
          # with `settings` of its own, refuses of that.
          systemRefuses =
            settings:
            lib.any
              (check: !check.assertion && lib.hasInfix "someone sets jitsusama.mycelium.agentsUid" check.message)
              (bareNixos [
                self.nixosModules.workstation
                { home-manager.users.someone.jitsusama.mycelium = apartAgents; }
                { jitsusama.mycelium = settings; }
              ]).config.assertions;
        in
        lib.hasInfix " ${socket} " " ${apartAgentsNode} "
        && !lib.hasInfix "--worker" (toString apartAgentsLinux.mycelium-other.Service.ExecStart)
        && !lib.hasInfix "--worker" linuxNode
        && evaluates (linux apartAgents)
        && refusedForAgentsAlone (mac apartAgents)
        && refusedForAgentsAlone (linux {
          agentsUid = 1001;
        })
        && systemRefuses { }
        && systemRefuses { account = "another"; }
        && !systemRefuses { account = "someone"; }
        && lib.elem "d /run/mycelium 0755 someone root -" (folderOn {
          account = "someone";
        })
        && !lib.any (lib.hasInfix "/run/mycelium") (folderOn { })
      );

  # A machine's system pins each key that links to it, on either system, to
  # its own realm and machine, with a terminal for an attach; one naming
  # none authorizes nothing of mycelium's.
  mycelium-lets-a-link-key-only-link =
    holds "mycelium-lets-a-link-key-only-link"
      "A link key a machine names isn't pinned to `mycelium ssh` as the realm and machine it names, on NixOS or nix-darwin, can't have a terminal for an attach, a machine naming none authorizes a key, a realm or machine name opening with a hyphen is taken, a key of two lines is, or a key's last newline isn't dropped."
      (
        let
          key = "ecdsa-sha2-nistp256 AAAAone";
          pinning = {
            account = "someone";
            linkedFrom.personal.one = key;
          };
          keys =
            bare: role: settings:
            (bare [
              role
              { jitsusama.mycelium = settings; }
            ]).config.users.users.someone.openssh.authorizedKeys.keys;
          pinnedOn =
            bare: role:
            let
              given = keys bare role pinning;
            in
            builtins.length given == 1
            && lib.hasPrefix ''restrict,pty,command="/nix/store/'' (lib.head given)
            && lib.hasSuffix ''/bin/mycelium --realm personal ssh --from one" ${key}'' (lib.head given);
          # The machines' keys as the option gives them.
          linked =
            given:
            (bareNixos [
              self.nixosModules.workstation
              {
                jitsusama.mycelium = pinning // {
                  linkedFrom.personal.one = given;
                };
              }
            ]).config.jitsusama.mycelium.linkedFrom;
          # Whether the name's own assertion refuses a realm and machine so
          # named, read by its message so another assertion failing can't
          # stand in. Only a failed one's message is read, since some are
          # made only then.
          misnamed =
            realm: machine:
            lib.any
              (
                check:
                !check.assertion
                && lib.hasPrefix "Each realm and machine in jitsusama.mycelium.linkedFrom" check.message
              )
              (bareNixos [
                self.nixosModules.workstation
                {
                  jitsusama.mycelium = {
                    account = "someone";
                    linkedFrom.${realm}.${machine} = key;
                  };
                }
              ]).config.assertions;
        in
        pinnedOn bareNixos self.nixosModules.workstation
        && pinnedOn bareDarwin self.darwinModules.workstation
        && keys bareNixos self.nixosModules.workstation { } == [ ]
        # A name opening with a hyphen would reach mycelium as a flag.
        && misnamed "personal" "-h"
        && misnamed "-h" "one"
        && !misnamed "personal" "other-machine"
        # A key's second line would be one of authorized_keys' own, unpinned.
        # The option's own value is read, since NixOS's keys refuse it too
        # and nix-darwin's may not; a key read from its file keeps its last
        # newline, which is taken and dropped.
        && !(builtins.tryEval (builtins.deepSeq (linked "${key}\nssh-ed25519 AAAAtwo") true)).success
        && linked "${key}\n" == { personal.one = key; }
      );

  # The words a link key is pinned to, read from the line a machine's system
  # writes rather than typed again, are ones the locked mycelium takes, so a
  # renamed action or flag, or one newly required, fails here rather than at
  # the first link or attach. They're run as sshd runs them for a login, with
  # no command asked, which the action refuses and keeps, naming the machine
  # the key is from: so the words parsed whole, past clap's checks that a
  # --help skips, and reached the action, which links nothing. The words
  # alone, as the node's flags are, so a Mac's check never builds the Linux
  # package the line names.
  mycelium-takes-its-link-key-s-words =
    let
      nixos = bareNixos [
        self.nixosModules.workstation
        {
          jitsusama.mycelium = {
            account = "someone";
            linkedFrom.personal.one = "ecdsa-sha2-nistp256 AAAAone";
          };
        }
      ];
      lines = nixos.config.users.users.someone.openssh.authorizedKeys.keys;
      line =
        assert lib.assertMsg (lines != [ ]) "a machine linked from one is given no key line";
        lib.head lines;
      pinned = builtins.match ''.*command="([^ "]*)/bin/mycelium ([^"]*)".*'' (
        builtins.unsafeDiscardStringContext line
      );
      words = lib.filter (word: word != "") (lib.splitString " " (lib.elemAt pinned 1));
      # The package the line pins, by its store path alone, which naming
      # builds nothing, so the words run here are the ones sshd would run.
      lockedFor = system: self.inputs.mycelium.packages.${system}.default;
      linePins = builtins.unsafeDiscardStringContext "${lockedFor nixos.pkgs.stdenv.hostPlatform.system}";
      locked = lockedFor pkgs.stdenv.hostPlatform.system;
      mycelium = lib.getExe (locked.overrideAttrs { doCheck = false; });
    in
    assert lib.assertMsg (pinned != null) "no link key line pins mycelium: ${line}";
    assert lib.assertMsg (
      lib.head pinned == linePins
    ) "the link key line pins ${lib.head pinned}, not the locked mycelium ${linePins}";
    # Kept in Nix because the locked mycelium and the words its key line
    # pins come from this evaluation, and a script of its own would only be
    # handed them back as words.
    pkgs.runCommandLocal "mycelium-takes-its-link-key-s-words" { } ''
      mkdir -p home state/mycelium
      ended=0
      env -u SSH_ORIGINAL_COMMAND HOME="$PWD/home" XDG_STATE_HOME="$PWD/state" \
        timeout -k 10 60 ${mycelium} ${lib.escapeShellArgs words} < /dev/null > said 2> told \
        || ended=$?
      # mycelium's errors end 1, its refusal among them, and clap's refusal
      # of the words 2; the record kept below says which error it was.
      case $ended in
        1) ;;
        0) echo "mycelium took a login on its link key's words"; cat said told; exit 1 ;;
        # timeout ends 124 on its TERM, and 137 once it kills a run that
        # ignored that, as a kill from outside, the builder's memory say, also
        # ends.
        124 | 137) echo "mycelium hung, or was killed, on a login on its link key's words"; cat told; exit 1 ;;
        *) echo "mycelium ended $ended on a login on its link key's words"; cat told; exit 1 ;;
      esac
      # Each refusal is a JSON object of its own line, read by its fields,
      # so neither its layout nor another field's text decides. mycelium
      # writes `asked` on every refusal, null where no command was asked.
      # jq's own words on a file it can't read stay in the log.
      kept=state/mycelium/refused-keys.jsonl
      ${lib.getExe pkgs.jq} -e 'select(.from == "one" and has("asked") and .asked == null)' \
        "$kept" > /dev/null || {
        echo "mycelium didn't take its link key's words:" ${lib.escapeShellArgs words}
        cat told
        exit 1
      }
      touch $out
    '';
}
