# mycelium carries the harness mesh between a realm's machines, so a session
# on one shows on the others and a message reaches it wherever it runs. Every
# workstation gets the program, which the other end of a link runs over SSH.
# A machine joins a realm by naming it, and then runs the node, which holds
# its sessions, starts the ones other machines ask of it, hears its pis and
# dials each machine it names: a user service on Linux, and on a Mac an agent
# in the login session. A machine sets both in its own repository:
#
#   jitsusama.mycelium.realm = "personal";
#   jitsusama.mycelium.links = [ "other-machine" ];
#
# A machine SSH can't reach, such as one a provider lent, is dialled through
# that provider's program instead, as `PROGRAM:PLACE`, the program by its store path:
#
#   jitsusama.mycelium.vias = [ "${cistern-link}/bin/cistern-link:sandbox-1" ];
#
# A provider's program can make such machines as well, for a start no
# machine of the realm takes, from the shapes it offers when the node asks:
#
#   jitsusama.mycelium.providers = [ "${cistern-cell}/bin/cistern-cell" ];
#
# A machine may join further realms too, each run by a node of its own,
# with its own state, mesh, steer inboxes, socket and key, where the harness
# keeps a realm that isn't the machine's home one. A pi is in one realm, MYCELIUM_REALM, and PI_HOME_REALM tells it
# which realm keeps the harness's usual folders:
#
#   jitsusama.mycelium.alsoJoins.other.links = [ "third-machine" ];
#
# Its links sign in with the realm's own key, served by tpm-signing as
# other-link, which no session's agent reaches.
#
# Nothing crosses between realms but along a flow, a one-way road for text
# a session names for the other realm, never opening its reverse:
#
#   jitsusama.mycelium.flows = [ { from = "personal"; to = "work"; } ];
#
# Both realms' nodes keep it, the sending one handing text across only
# under it and the receiving one taking text only under it, so a fault at
# one end can't open the other way.
#
# Each link is an SSH destination the machine can already reach without a
# prompt, its host key pinned. On Linux the node signs in with the agents'
# key from the TPM (../tpm-signing), which asks nothing, so the far machine
# authorizes that key: Joel's own asks for its PIN, and the node dialling at
# login would hold every agent's signature up until he answered. On a Mac it
# signs in with the agents' key from the Secure Enclave
# (../secure-enclave-signing), by the options that module gives for it, so
# the far machine authorizes that key too, and the login session's agent,
# which may ask for an approval, is never asked.
#
# A machine also declares what its node can't measure of it, for work to be
# placed by: the networks work there always reaches, a probe for each it
# reaches only some of the time (a laptop's home), what a held session
# outlives, whether work costs money, and what it refuses. Only this
# configuration says it, as a file every end of a link on the machine reads:
#
#   jitsusama.mycelium.declares = {
#     reach = [ "internet" ];
#     probes.home = "router.lan:53";
#     probes.office = [ "/run/current-system/sw/bin/test" "-e" "/run/office" ];
#     taints = [ { key = "person-works"; effect = "prefer_not"; why = "..."; } ];
#   };
#
# With it goes the command an agent session starts with in each realm, which
# pi's module sets where pi is installed (jitsusama.mycelium.agents).
#
# Each node also serves the harness's files of its realm to whoever asks it,
# as the launcher does, and hears each change: its quests and the sessions
# to restore, and on the home realm's the machine's waiting, verdicts and
# queue.
{ mycelium }:
{
  config,
  lib,
  pkgs,
  ...
}:
let
  cfg = config.jitsusama.mycelium;
  inherit (config.jitsusama.signing) signer;
  package = mycelium.packages.${pkgs.stdenv.hostPlatform.system}.default;
  # The swarm's teams completed by the names a person types, which only the
  # node knows (completion.zsh).
  completion = pkgs.runCommandLocal "mycelium-zsh-completion" { } ''
    install -D -m 444 ${./completion.zsh} $out/share/zsh/site-functions/_mycelium
  '';
  inherit (pkgs.stdenv.hostPlatform) isDarwin isLinux;
  linking = cfg.links != [ ] || cfg.vias != [ ] || cfg.providers != [ ];
  serving = cfg.realm != null;
  agentsKey = config.systemd.user.sockets ? agent-signing;
  declaration = import ./declaration.nix {
    inherit lib;
    callIsOn = lib.getExe (pkgs.callPackage ../calls/package.nix { });
  } cfg;
  # The node reads the declaration from the store, where only root writes,
  # never from the account's config folder: anything running as the account
  # could repoint that link and widen how agents' commands run.
  declaredArgs = lib.optionals (cfg.declares != null) [
    "--declared"
    "${pkgs.writeText "mycelium-declared.json" declaration}"
  ];
  # The tools every machine of every realm has, whatever its own
  # configuration adds: what a session or run another machine starts here
  # reaches for. ssh stays the system's, which a Mac's keychain and enclave
  # options need. git, git-lfs and gh stay the account's own, since the kit
  # goes first and an account may wrap them, as one whose repositories
  # need their own git or gh does; a realm that wants them adds them.
  # curl speaks WebSockets, since a Mac's websocat reaches TLS only through
  # Secure Transport, which stops at 1.2, and a front taking 1.3 alone needs
  # a wss:// client over OpenSSL there. Going first changes nothing else:
  # the account's curl is core's own, the same build without WebSockets,
  # trusting the same certificates.
  shared = [
    package
    (pkgs.curl.override { websocketSupport = true; })
    pkgs.fd
    pkgs.jq
    pkgs.ripgrep
    pkgs.websocat
  ];
  # A realm's kit as one folder, its `bin` the node puts first on the path
  # of each session and run it starts, after the login profile, so no
  # start-up file hides a tool. Its store path names its version, which
  # each machine record carries.
  kitArgs =
    realm: tools:
    lib.optionals (tools != [ ]) [
      "--kit"
      "${pkgs.buildEnv {
        name = "mycelium-kit-${realm}";
        paths = tools;
        pathsToLink = [ "/bin" ];
      }}"
    ];
  # What the home realm's node lends the worker machines it made, from the
  # store as the declaration is, so the lend is the one the machine's
  # configuration says, not a file in the account's config folder that a
  # command run as the account could change in passing. It guards the lend
  # from the machines lent to, not from the account itself: a command run as
  # the account unfenced could as well give the node words of its own, or
  # bind the relay's socket in the runtime folder, and it already reaches
  # every key that account holds, so the lend opens it nothing new. A key
  # left out is left out of the file, as mycelium reads a budget whose
  # upstream adds its own. Only Linux lends: model-relay, the upstream a lend
  # is built for, is Linux's alone. And on a Mac the node's runtime folder,
  # onto which the lender joins the upstream, is the account's cache folder,
  # DARWIN_USER_CACHE_DIR: nothing yet shows that a fenced agent there can't
  # bind a socket at the upstream and be handed the key.
  lendArgs = lib.optionals (isLinux && cfg.lend != null) [
    "--lend"
    "${pkgs.writeText "mycelium-lend.json" (
      builtins.toJSON {
        vms = map (lib.filterAttrs (_: value: value != null)) cfg.lend.vms;
        # A price's unset cache fields are left out too, as serde reads
        # each Option the file leaves out as none.
        budgets = lib.mapAttrs (
          _: budget:
          lib.filterAttrs (_: value: value != null) (
            budget
            // lib.optionalAttrs (budget.prices != null) {
              prices = lib.mapAttrs (_: lib.filterAttrs (_: value: value != null)) budget.prices;
            }
          )
        ) cfg.lend.budgets;
      }
    )}"
  ];
  # The person's table of which receptors' climbs the node says, from the
  # store as the declaration is: the node refuses one any other account
  # could write, and a table naming nothing is no table, so every source is
  # silent.
  stimuliArgs = lib.optionals (cfg.stimuli != [ ]) [
    "--stimuli"
    "${pkgs.writeText "mycelium-stimuli.json" (builtins.toJSON { say = cfg.stimuli; })}"
  ];
  sources = import ./sources.nix;
  name = lib.types.strMatching "[a-z0-9-]+";
  reaches = import ./reaches.nix { inherit lib; };
  lend = import ./lend.nix { inherit lib; };
  further = lib.types.submodule (
    { name, ... }:
    {
      options = {
        links = lib.mkOption {
          type = lib.types.listOf lib.types.str;
          default = [ ];
          description = "The machines of this realm this machine dials, as `links` names them.";
        };
        inherit (reaches) vias providers;
        kit = lib.mkOption {
          type = lib.types.listOf lib.types.package;
          description = kitDescription;
        };
        signsInWith = lib.mkOption {
          type = lib.types.nullOr (lib.types.strMatching "[a-z0-9-]+");
          # tpm-signing serves each realm's link key so, apart from the
          # agents' key every session's agent reaches.
          default = "${name}-link";
          defaultText = lib.literalExpression ''"<realm>-link"'';
          description = ''
            The socket unit whose agent, at `<name>.sock` in the user's runtime
            folder, holds the key this realm's links sign in with on Linux, or
            null for none, so its links can't sign in. Keep it a key no other
            realm's machines authorize, and out of every session's reach. A
            Mac's node uses its login session's agent.
          '';
        };
      };
      # Every realm's kit holds the shared tools; a machine adds its own.
      config.kit = shared;
    }
  );
  kitDescription = ''
    The tools every session and run this realm's node starts here finds
    first on its path, whatever the login profile sets: the realm's kit,
    which its machine record names, so work needing a tool goes where the
    kit holds it. It holds the tools every machine shares, and a machine's
    own configuration adds to it, such as an account's cloud tools, a
    tool given as `lib.hiPrio` winning over a shared one of the same name;
    empty, the node is given no kit.
  '';
  # What a machine may say of itself, and the flows between realms
  # (declares.nix).
  inherit (import ./declares.nix { inherit lib isLinux name; }) declares flow;
  # The flows a realm's node keeps: each realm its text may cross into,
  # and each whose text it takes.
  flowsOf =
    realm:
    lib.concatMap (one: [
      "--flow-to"
      one.to
    ]) (lib.filter (one: one.from == realm) cfg.flows)
    ++ lib.concatMap (one: [
      "--flow-from"
      one.from
    ]) (lib.filter (one: one.to == realm) cfg.flows);
  # The words that run a realm's node, given what keeps it apart.
  nodeIn =
    realm: links: vias: providers: apart:
    [
      (lib.getExe package)
      "--realm"
      realm
      "node"
    ]
    ++ apart
    ++ declaredArgs
    ++ stimuliArgs
    ++ flowsOf realm
    ++ lib.optionals (isDarwin && signer != null) (
      lib.concatMap (option: [
        "--ssh-option"
        option
      ]) signer.agentSshOptions
    )
    ++ lib.concatMap (machine: [
      "--ssh"
      machine
    ]) links
    ++ lib.concatMap (machine: [
      "--via"
      machine
    ]) vias
    ++ lib.concatMap (program: [
      "--provider"
      program
    ]) providers
    # Every realm's node reads its cells' waits from the one folder the
    # home realm's node serves, so the two can't drift apart.
    ++ [
      "--waiting"
      (servesIn cfg.realm).waiting.folder
      "--serves"
      (servesFile realm)
    ]
    ++ lib.optionals (cfg.healthSocket != null) [
      "--health-socket"
      cfg.healthSocket
    ];
  # The harness's files a realm's node serves, and on the home realm's the
  # machine's own and the balance's latest reading (serves.nix).
  servesIn = import ./serves.nix { inherit config cfg lib; };
  servesFile = realm: "${config.xdg.configHome}/mycelium/serves-${realm}.json";
  realms = lib.optional serving cfg.realm ++ lib.attrNames cfg.alsoJoins;
  builders = import ./builders.nix;
  # What the home realm's node may do to ease the machine (eases.nix).
  eases = import ./eases.nix { inherit config lib mycelium; };
  agentsSocket = import ./agents-socket.nix { inherit config lib isLinux; };
  # The home realm's node alone eases the machine, since a further realm's
  # sessions aren't its to move, and alone lends, since the machines it
  # lends to are the ones it made.
  node =
    nodeIn cfg.realm cfg.links cfg.vias cfg.providers [ ]
    ++ eases.args
    ++ lendArgs
    ++ agentsSocket.args
    ++ lib.optionals isLinux builders
    ++ ownAgent cfg.signsInWith
    ++ kitArgs cfg.realm cfg.kit;
  # Where the harness and mycelium keep a realm that isn't the home one.
  apartIn = realm: [
    "--state"
    "${config.xdg.stateHome}/mycelium/${realm}"
    "--mesh"
    "${config.xdg.stateHome}/pi/agentic-harness.pi/realms/${realm}/mesh-workflow"
    "--steer"
    "${config.xdg.stateHome}/pi/agentic-harness.pi/realms/${realm}/steer"
  ];
  furtherNodes = lib.mapAttrs (
    realm: joined:
    nodeIn realm joined.links joined.vias joined.providers (apartIn realm)
    ++ ownAgent joined.signsInWith
    ++ kitArgs realm joined.kit
  ) cfg.alsoJoins;
  # A Linux node's ssh asks the agent at SSH_AUTH_SOCK, its realm's key,
  # rather than one the account's SSH config names for the machine: that
  # config's IdentityAgent wins over SSH_AUTH_SOCK, so on 2026-11-20 the
  # node asked Joel's own agent, whose gate refuses agents' work, and the
  # work realm lost its Mac. IdentitiesOnly=no lets it offer what that
  # agent holds rather than only the config's IdentityFile. ForwardAgent=no
  # keeps that key, which asks nothing, from being lent to the machine it
  # dials, whatever the config says for it, and AddKeysToAgent=no keeps a
  # key file the config names from being added to the agent agents reach.
  # A node with no key, and a Mac's, which is given its own options, is
  # given none of them.
  ownAgent =
    key:
    lib.optionals (isLinux && key != null) (
      lib.concatMap
        (option: [
          "--ssh-option"
          option
        ])
        [
          "IdentityAgent=SSH_AUTH_SOCK"
          "AddKeysToAgent=no"
          "IdentitiesOnly=no"
          "ForwardAgent=no"
        ]
    );
  linuxNode = description: words: key: realm: {
    Unit = {
      Description = description;
      Wants = lib.optional (key != null) "${key}.socket";
      After = lib.optional (key != null) "${key}.socket";
      # The node reads what it serves only as it starts, so a switch that
      # changes it restarts the node, which the sessions outlive: on
      # 2026-11-06 a switch added the runs and the node served none.
      X-Restart-Triggers = [ (builtins.hashString "sha256" (builtins.toJSON (servesIn realm))) ];
    };
    Service = {
      ExecStart = lib.escapeShellArgs words;
      Environment = lib.optional (key != null) "SSH_AUTH_SOCK=%t/${key}.sock";
      # The node lets go of the sessions it carried when told to stop,
      # and redials a dropped link itself. It is started again however it
      # ends, unless systemd stopped it: a terminate signal from anything
      # else ends it cleanly too, and on 2026-11-14 one left the work
      # realm's node down for seven minutes.
      Restart = "always";
      RestartSec = "5s";
      # Each client, link and session it holds is a file, and at systemd's
      # 1024 a failed accept once ended the node (mycelium #372). A bare
      # number would set the hard limit too, so the hard limit is named
      # as systemd's own default, 524288, rather than lowered.
      LimitNOFILE = "65536:524288";
      # The node wakes with every session it carries, so it runs among
      # them rather than among Joel's programs, whose tasks each wakeup
      # would preempt. Moving it restarts it once, which the sessions,
      # in their terminals' scopes, outlive.
      Slice = "agents.slice";
    };
    Install.WantedBy = [ "default.target" ];
  };
  macNode = words: log: realm: {
    enable = true;
    config = {
      ProgramArguments = words;
      # The system's own ssh first, which reads the machine's SSH config
      # and the login session's agent. The account's profile follows it,
      # since a carry runs the harness's pi-carry by name, as the Linux
      # node finds it through the session's PATH.
      EnvironmentVariables = {
        PATH = "/usr/bin:/bin:${config.home.profileDirectory}/bin";
        # What the node starts is driven from another machine, where
        # nobody can touch this one's sensor, so its commits sign with
        # the agents' key, even in a held terminal.
        SIGN_AS = "agent";
        # The node reads what it serves only as it starts, and launchd
        # reloads an agent only when its own file changes, so what it
        # serves is written into it, as X-Restart-Triggers is on Linux.
        MYCELIUM_SERVES = builtins.hashString "sha256" (builtins.toJSON (servesIn realm));
      };
      RunAtLoad = true;
      # Started again whenever it stops, but not more than every 30 s.
      KeepAlive = true;
      ThrottleInterval = 30;
      # Each client, link and session it holds is a file, and launchd
      # gives an agent 256. 10240 is macOS's OPEN_MAX, and the kernel
      # quietly lowers a soft limit above its kern.maxfilesperproc to that.
      # No hard limit is named, so launchd's stays, unlimited unless
      # someone lowered it.
      SoftResourceLimits.NumberOfFiles = 10240;
      StandardErrorPath = "${config.home.homeDirectory}/Library/Logs/${log}.log";
    };
  };
  describe = realm: "Carries the harness mesh between the machines of the ${realm} realm";
  # Each machine reads, each minute, how evenly the home realm's machines
  # carry its work, and keeps each reading a file a day in mycelium's state,
  # so placement can be judged and its weights tuned from what it did. Each
  # node holds the whole realm's facts, so every machine keeps the same
  # record and none is central. Its stdout repeats what it keeps, so it goes
  # nowhere; what it can't read, it says on stderr.
  balance = [
    (lib.getExe package)
    "--realm"
    cfg.realm
    "balance"
    "--follow"
  ];
in
{
  imports = [
    ../signing/home.nix
    ./state-subvolume.nix
  ];

  options.jitsusama.mycelium = {
    realm = lib.mkOption {
      type = lib.types.nullOr name;
      default = null;
      example = "personal";
      description = ''
        The realm this machine belongs to, the set of machines whose nodes
        link with nothing crossing between realms, or null for a machine in
        none. A machine keeps the realm it first joined, so changing it later
        is refused until mycelium's state is cleared. This is its home realm,
        whose sessions keep the harness's usual folders.
      '';
    };
    signsInWith = lib.mkOption {
      type = lib.types.nullOr name;
      default = if agentsKey then "agent-signing" else null;
      defaultText = lib.literalExpression ''"agent-signing", where the agents' key is served, else null'';
      example = "work-link";
      description = ''
        The socket unit whose agent, at `<name>.sock` in the user's runtime
        folder, holds the key the home realm's links sign in with on Linux,
        or null for none. The agents' key by default; a key of the realm's
        own (link-keys) lets the machines it links to pin it, which they
        can't do to a key sessions use too. A Mac's node uses its login
        session's agent.
      '';
    };
    alsoJoins = lib.mkOption {
      type = lib.types.attrsOf further;
      default = { };
      example = {
        other.links = [ "third-machine" ];
      };
      description = ''
        Further realms this machine belongs to, by name, each run by a node
        of its own, with its own state, mesh, steer inboxes, socket and key,
        so no session or message crosses between realms. What the machine
        declares of itself is the same in each.
      '';
    };
    links = lib.mkOption {
      type = lib.types.listOf lib.types.str;
      default = [ ];
      example = [ "other-machine" ];
      description = ''
        The machines of the realm this machine dials and keeps linked, as SSH
        destinations. Two machines link once, so only one of each pair names
        the other. With none, the machine's node answers links and dials
        nothing.
      '';
    };
    inherit (reaches) vias providers;
    flows = lib.mkOption {
      type = lib.types.listOf flow;
      default = [ ];
      example = [
        {
          from = "personal";
          to = "work";
        }
      ];
      description = ''
        The one-way roads between realms this machine's nodes keep, each
        letting text a session names for `to` cross from `from`, and never
        the reverse. Only a fyi, finding or question to peers by address
        crosses, and only on a machine joining both realms, between their
        two nodes; a machine joining `from` alone keeps what is sent for a
        link to carry to one. Realms are apart wherever no flow is named.
      '';
    };
    stimuli = lib.mkOption {
      type = lib.types.listOf (lib.types.enum sources);
      default = [ ];
      example = [ "github" ];
      description = ''
        The sources whose receptors say on their outside network when more
        is asked of the person, as counts alone, in every realm this
        machine joins. A source left out stays silent, and so does every
        source when none is named. mycelium's PROTOCOL.md says what a
        signal holds.
      '';
    };
    declares = lib.mkOption {
      type = lib.types.nullOr declares;
      default = null;
      description = ''
        What this machine says of itself that its node can't measure, for
        work to be placed by, or null to declare nothing, which mycelium
        reads as unknown.
      '';
    };
    kit = lib.mkOption {
      type = lib.types.listOf lib.types.package;
      description = kitDescription;
    };
    lend = lib.mkOption {
      type = lib.types.nullOr lend;
      default = null;
      example = lib.literalExpression ''
        {
          vms = [
            { prefix = "agents-"; budget = "models"; requests_a_day = 500; usd_a_day = 25; }
          ];
          budgets.models = {
            upstream = "model-relay/agents.sock";
            host = "models.example";
            paths = [ "/v1/messages" ];
            auth_header = "authorization";
            key = [ "''${models-key}/bin/models-key" ];
            prices.a-model = { input = 3; output = 15; most_output = 64000; };
            spend_watch = "models";
          };
        }
      '';
      description = ''
        What the home realm's node lends the worker machines it made, or
        null to lend nothing, so each one's lend is refused; only Linux
        lends. Its names are those of the file mycelium reads, a store file
        it reads at every request; mycelium's PROTOCOL.md says what each
        means. Lending a machine model calls through a budget hands it no
        key: the node adds the key to each request it carries upstream. To
        stop lending at once, set it to null and switch, which restarts the
        node.
      '';
    };
    healthSocket = lib.mkOption {
      type = lib.types.nullOr lib.types.str;
      default =
        if isLinux then "%t/daemons/system.sock" else "${config.xdg.stateHome}/daemons/system.sock";
      defaultText = lib.literalExpression ''"%t/daemons/system.sock" on Linux, "''${config.xdg.stateHome}/daemons/system.sock" on a Mac'';
      description = ''
        The socket of the daemon that says how this machine stands, whose
        words each node carries in the machine record it shares, so every
        machine of the realm sees them; or null to carry none. The system
        daemon's (../system): on Linux in the user's runtime folder, `%t`,
        and on a Mac, which has none, in the account's state folder. A
        socket nobody answers reads as health unknown.
      '';
    };
    agents = lib.mkOption {
      type = lib.types.attrsOf lib.types.str;
      default = { };
      example = {
        work = "pi";
        personal = "pi-claude";
      };
      description = ''
        The command an agent session starts with here, by the realm it
        joins, for whatever starts one, such as the launcher. Each realm's
        node tells only its own. It is declared with the rest, so a machine
        declaring nothing says none, and whatever starts a session picks.
        Held fenced or strict, on Linux or on a Mac declaring
        `declares.piFencesItself`, it starts outside the fence's launcher,
        so it must fence its own commands, as pi does.
      '';
    };
    # Internal, so no machine widens it; only a test sets it otherwise. An
    # agent asking as fenced runs strict, which is stricter than it needs,
    # never looser; a Mac runs its strict for either, which writes the
    # run's folder and the account's temporary folder. For one release the
    # node's socket is there too, beside the cache folder's, for a session
    # that still looks there, so a fenced run can reach it.
    fenceRuns = lib.mkOption {
      type = lib.types.nullOr (lib.types.listOf lib.types.str);
      default = [
        (lib.getExe (pkgs.callPackage ../pi/fence-run/package.nix { }))
        "--kind"
        "strict"
        "--"
      ];
      defaultText = lib.literalExpression ''[ "\''${pi-fence}/bin/pi-fence" "--kind" "strict" "--" ]'';
      internal = true;
      description = ''
        The words an agent's `mycelium run` is put after to run in its
        fence here, the first a program in the Nix store, as each fenced
        and strict fence the machine declares says them: pi's own fence,
        strict, on Linux and on a Mac, whose strict reads the home folder
        but what it hides, where Linux's reads only what it lists. A held
        fenced or strict start goes after them too, except, on Linux and on a
        Mac declaring `declares.piFencesItself`, the realm's agent program,
        as `agents` names it or else pi, told its fence, fenced as it
        fences itself. Null refuses every agent's run and each held start
        that would go after them; the agent program held alone still starts.
      '';
    };
  }
  // eases.options
  // agentsSocket.options;

  config = lib.mkMerge [
    { inherit (eases) warnings; }
    { inherit (agentsSocket) assertions; }
    # Warned of rather than refused, so a machine lending models before
    # its lend caps their spend still evaluates; mycelium refuses each such
    # model lend 403, so nothing is spent uncapped either way.
    (lib.mkIf (cfg.lend != null) {
      warnings =
        let
          models = lib.filterAttrs (_: budget: budget.kind == "models") cfg.lend.budgets;
          lendsModels = vm: lib.any (budget: models ? ${budget}) ([ vm.budget ] ++ vm.also);
          missing =
            budget:
            lib.optional (budget.prices == null || budget.prices == { }) "prices"
            ++ lib.optional (budget.spend_watch == null) "spend_watch";
        in
        lib.mapAttrsToList (
          name: budget:
          "jitsusama.mycelium.lend.budgets.${name} names no ${lib.concatStringsSep " and no " (missing budget)}, so mycelium refuses every model lent through it."
        ) (lib.filterAttrs (_: budget: missing budget != [ ]) models)
        ++ map (
          vm:
          "jitsusama.mycelium.lend.vms' ${vm.prefix} names no usd_a_day, so mycelium lends its machines no model."
        ) (lib.filter (vm: lendsModels vm && vm.usd_a_day == null) cfg.lend.vms);
    })
    {
      assertions = [
        {
          assertion = linking -> cfg.realm != null;
          message = "jitsusama.mycelium.links, vias and providers need jitsusama.mycelium.realm, the realm they link in.";
        }
        {
          assertion = cfg.alsoJoins != { } -> cfg.realm != null;
          message = "jitsusama.mycelium.alsoJoins needs jitsusama.mycelium.realm, the home realm.";
        }
        {
          assertion = !(cfg.alsoJoins ? ${toString cfg.realm});
          message = "jitsusama.mycelium.alsoJoins names the home realm, which jitsusama.mycelium.realm already joins.";
        }
        {
          assertion = lib.all (realm: realm == cfg.realm || cfg.alsoJoins ? ${realm}) (
            lib.attrNames cfg.agents
          );
          message = "jitsusama.mycelium.agents names a realm this machine doesn't join.";
        }
        {
          assertion = lib.all (
            one: one.from != one.to && (lib.elem one.from realms || lib.elem one.to realms)
          ) cfg.flows;
          message = "jitsusama.mycelium.flows has a flow from a realm into itself, or between two realms this machine joins neither of.";
        }
        {
          assertion = cfg.lend != null -> cfg.realm != null;
          message = "jitsusama.mycelium.lend needs jitsusama.mycelium.realm, the realm whose node lends.";
        }
        {
          # Both reasons lendArgs gives are in the message, so neither
          # going alone reads as leave to lend from a Mac.
          assertion = isDarwin -> cfg.lend == null;
          message = "jitsusama.mycelium.lend is set on a Mac, where model-relay, the upstream a lend is built for, doesn't run, and nothing shows a fenced agent can't bind the upstream in the node's runtime folder and take the key: lend from Linux.";
        }
        {
          assertion =
            cfg.lend != null
            -> lib.all (
              vm: lib.all (budget: cfg.lend.budgets ? ${budget}) ([ vm.budget ] ++ vm.also)
            ) cfg.lend.vms;
          message = "jitsusama.mycelium.lend.vms names a budget, in budget or also, jitsusama.mycelium.lend.budgets doesn't hold.";
        }
        {
          # mycelium refuses a machine whose name starts as two prefixes do,
          # so two that overlap would refuse every machine they both match.
          # Compared by place, so a prefix named twice is caught too.
          assertion =
            cfg.lend != null
            -> (
              let
                prefixes = map (vm: vm.prefix) cfg.lend.vms;
                places = lib.range 0 (lib.length prefixes - 1);
                at = builtins.elemAt prefixes;
                apart = one: other: one == other || !(lib.hasPrefix (at one) (at other));
              in
              lib.all (one: lib.all (apart one) places) places
            );
          message = "jitsusama.mycelium.lend.vms has two prefixes where one starts the other, so every machine both match is refused.";
        }
        {
          assertion = !(cfg.alsoJoins ? balance);
          message = "jitsusama.mycelium.alsoJoins names a realm balance, whose node's unit would be the one that reads the home realm's balance.";
        }
        {
          assertion = lib.all (lib.types.strMatching "[a-z0-9-]+").check (lib.attrNames cfg.alsoJoins);
          message = "jitsusama.mycelium.alsoJoins names a realm in other than lowercase letters, digits and dashes.";
        }
      ];
      home.packages = [
        package
        completion
      ];
      # Every realm's kit holds the shared tools; a machine adds its own.
      jitsusama.mycelium.kit = shared;

      # So an agent asked what the sessions are doing answers from the node,
      # in words, the same model every other view draws.
      home.file.".pi/agent/skills/mycelium/SKILL.md".source = ./SKILL.md;
    }
    (lib.mkIf (cfg.realm != null) {
      home.sessionVariables = {
        MYCELIUM_REALM = cfg.realm;
        # A pi started in another realm keeps that realm's folders apart.
        PI_HOME_REALM = cfg.realm;
      };
    })
    (lib.mkIf (cfg.declares != null) {
      # Only a Mac's pi lays a sandbox profile round each command it runs,
      # and only a Mac's node acts on the word, so it is refused where it
      # can't be true.
      assertions = [
        {
          assertion = cfg.declares.piFencesItself -> isDarwin;
          message = "jitsusama.mycelium.declares.piFencesItself is set on Linux; it says pi fences each command it runs with a sandbox profile, which only a Mac's pi does; set it only on a Mac.";
        }
      ];
      # Where the account's commands read it: XDG_CONFIG_HOME's, or
      # ~/.config's. The node reads its own copy in the store
      # (declaredArgs), and an end of a link another machine dials asks
      # the node.
      xdg.configFile."mycelium/declared.json".text = declaration;
    })
    (lib.mkIf serving {
      xdg.configFile = lib.listToAttrs (
        map (
          realm:
          lib.nameValuePair "mycelium/serves-${realm}.json" { text = builtins.toJSON (servesIn realm); }
        ) realms
      );
    })
    (lib.mkIf (isLinux && serving) {
      systemd.user.services = {
        mycelium = linuxNode (describe cfg.realm) node cfg.signsInWith cfg.realm;
        mycelium-balance = {
          Unit = {
            Description = "Reads how evenly the ${cfg.realm} realm's machines carry its work";
            After = [ "mycelium.service" ];
          };
          Service = {
            ExecStart = lib.escapeShellArgs balance;
            StandardOutput = "null";
            # It reads again whatever ends it, as its node does, but no
            # faster than its node comes back.
            Restart = "always";
            RestartSec = "30s";
            Slice = "agents.slice";
          };
          Install.WantedBy = [ "default.target" ];
        };
      }
      // lib.mapAttrs' (
        realm: words:
        lib.nameValuePair "mycelium-${realm}" (
          linuxNode (describe realm) words cfg.alsoJoins.${realm}.signsInWith realm
        )
      ) furtherNodes;
    })
    (lib.mkIf (isDarwin && serving) {
      launchd.agents = {
        mycelium = macNode node "mycelium" cfg.realm;
        mycelium-balance = {
          enable = true;
          config = {
            ProgramArguments = balance;
            RunAtLoad = true;
            KeepAlive = true;
            ThrottleInterval = 30;
            StandardErrorPath = "${config.home.homeDirectory}/Library/Logs/mycelium-balance.log";
          };
        };
      }
      // lib.mapAttrs' (
        realm: words: lib.nameValuePair "mycelium-${realm}" (macNode words "mycelium-${realm}" realm)
      ) furtherNodes;
    })
  ];
}
