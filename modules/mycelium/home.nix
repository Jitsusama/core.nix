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
# Each link is an SSH destination the machine can already reach without a
# prompt, its host key pinned. On Linux the node signs in with the agents'
# key from the TPM (../tpm-signing), which asks nothing, so the far machine
# authorizes that key: Joel's own asks for its PIN, and the node dialling at
# login would hold every agent's signature up until he answered.
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
# to restore, and on the home realm's the machine's waiting and verdicts.
{ mycelium }:
{
  config,
  lib,
  pkgs,
  ...
}:
let
  cfg = config.jitsusama.mycelium;
  package = mycelium.packages.${pkgs.stdenv.hostPlatform.system}.default;
  inherit (pkgs.stdenv.hostPlatform) isDarwin isLinux;
  linking = cfg.links != [ ];
  serving = cfg.realm != null;
  agentsKey = config.systemd.user.sockets ? agent-signing;
  name = lib.types.strMatching "[a-z0-9-]+";
  further = lib.types.submodule (
    { name, ... }:
    {
      options = {
        links = lib.mkOption {
          type = lib.types.listOf lib.types.str;
          default = [ ];
          description = "The machines of this realm this machine dials, as `links` names them.";
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
    }
  );
  taint = lib.types.submodule {
    options = {
      key = lib.mkOption {
        type = name;
        description = "What work names to tolerate the taint.";
      };
      effect = lib.mkOption {
        type = lib.types.enum [
          "never"
          "prefer_not"
        ];
        description = ''
          How hard it refuses work that doesn't tolerate it: never, or only
          while anywhere else will do.
        '';
      };
      why = lib.mkOption {
        type = lib.types.str;
        description = "Why, for a person reading where work went.";
      };
    };
  };
  declares = lib.types.submodule {
    options = {
      reach = lib.mkOption {
        type = lib.types.listOf name;
        example = [
          "home"
          "internet"
        ];
        description = "The networks work on this machine always reaches, wherever it is.";
      };
      probes = lib.mkOption {
        type = lib.types.attrsOf (lib.types.either lib.types.str (lib.types.listOf lib.types.str));
        default = { };
        example = {
          home = "router.lan:53";
          office = [
            "/run/current-system/sw/bin/test"
            "-e"
            "/run/office"
          ];
        };
        # The node tries each every 30 seconds, giving it 3 to answer. A
        # command suits a network an address can't tell apart, such as one
        # known by its Wi-Fi or a tunnel's link; it runs with no shell.
        description = "Networks reached at times, each by a host:port or command answering only then.";
      };
      outlives = lib.mkOption {
        type = lib.types.listOf (
          lib.types.enum [
            "window_close"
            "node_restart"
            "sleep"
            "reboot"
          ]
        );
        # A held session's holder is a process of its own, apart from its
        # window and the node, which Linux keeps through a suspend.
        default = lib.optionals isLinux [
          "window_close"
          "node_restart"
          "sleep"
        ];
        description = "What a session held on this machine outlives.";
      };
      costsMoney = lib.mkOption {
        type = lib.types.bool;
        default = false;
        description = "Whether work on this machine costs money.";
      };
      taints = lib.mkOption {
        type = lib.types.listOf taint;
        default = [ ];
        description = ''
          What this machine refuses, or would rather not take, each by a key
          work tolerates it by. The machine enforces its own.
        '';
      };
      within = lib.mkOption {
        type = lib.types.nullOr another;
        default = null;
        example = {
          machine = "host-machine";
          file = "/run/host/said.json";
        };
        description = ''
          The machine this one runs within, as a virtual machine runs in its
          host, and the file, as this machine reaches it, in which the host
          says its power and room, which this machine then has as its own.
        '';
      };
      hosts = lib.mkOption {
        type = lib.types.listOf another;
        default = [ ];
        description = ''
          The machines that run within this one, each with the file, as this
          machine reaches it, that its node writes its power and room to
          each minute.
        '';
      };
    };
  };
  # One machine within another, as either names it.
  another = lib.types.submodule {
    options = {
      machine = lib.mkOption {
        type = name;
        description = "The other machine's name in the realm.";
      };
      file = lib.mkOption {
        type = lib.types.str;
        description = "The file the two share, as this machine reaches it.";
      };
    };
  };
  # The words that run a realm's node, given what keeps it apart.
  nodeIn =
    realm: links: apart:
    [
      (lib.getExe package)
      "--realm"
      realm
      "node"
    ]
    ++ apart
    ++ lib.concatMap (machine: [
      "--ssh"
      machine
    ]) links
    ++ [
      "--serves"
      (servesFile realm)
    ];
  # The harness's files a realm's node serves whoever reads it, the
  # launcher first, each as a kind of records: the realm's quests and the
  # sessions to restore, and on the home realm's node the machine's
  # waiting and verdicts, which no realm owns, so nothing is heard twice.
  # A file not written yet is served as holding nothing.
  servesIn =
    realm:
    let
      state = config.xdg.stateHome;
      home = realm == cfg.realm;
      harness =
        if home then "${state}/pi/agentic-harness.pi" else "${state}/pi/agentic-harness.pi/realms/${realm}";
    in
    {
      quest = {
        file = "${harness}/quest-workflow/quests.json";
        records = "quests";
        key = "id";
      };
      restore = {
        file = if home then "${state}/pi/restore.json" else "${harness}/restore.json";
        records = "sessions";
        key = "id";
      };
    }
    // lib.optionalAttrs home {
      waiting = {
        folder = "${state}/mission/waiting";
        records = "items";
        key = "id";
        by = "quest";
      };
      verdict = {
        file = "${state}/mission/verdicts.json";
        records = "items";
      };
    };
  servesFile = realm: "${config.xdg.configHome}/mycelium/serves-${realm}.json";
  realms = lib.optional serving cfg.realm ++ lib.attrNames cfg.alsoJoins;
  node = nodeIn cfg.realm cfg.links [ ];
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
    realm: joined: nodeIn realm joined.links (apartIn realm)
  ) cfg.alsoJoins;
  linuxNode = description: words: key: {
    Unit = {
      Description = description;
      Wants = lib.optional (key != null) "${key}.socket";
      After = lib.optional (key != null) "${key}.socket";
    };
    Service = {
      ExecStart = lib.escapeShellArgs words;
      Environment = lib.optional (key != null) "SSH_AUTH_SOCK=%t/${key}.sock";
      # The node lets go of the sessions it carried when told to stop,
      # and redials a dropped link itself, so only its own failure
      # restarts it.
      Restart = "on-failure";
      RestartSec = "5s";
    };
    Install.WantedBy = [ "default.target" ];
  };
  macNode = words: log: {
    enable = true;
    config = {
      ProgramArguments = words;
      # The system's own ssh, which reads the machine's SSH config and
      # the login session's agent.
      EnvironmentVariables = {
        PATH = "/usr/bin:/bin";
        # What the node starts is driven from another machine, where
        # nobody can touch this one's sensor, so its commits sign with
        # the agents' key, even in a held terminal.
        SIGN_AS = "agent";
      };
      RunAtLoad = true;
      # Started again whenever it stops, but not more than every 30 s.
      KeepAlive = true;
      ThrottleInterval = 30;
      StandardErrorPath = "${config.home.homeDirectory}/Library/Logs/${log}.log";
    };
  };
  describe = realm: "Carries the harness mesh between the machines of the ${realm} realm";
in
{
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
    declares = lib.mkOption {
      type = lib.types.nullOr declares;
      default = null;
      description = ''
        What this machine says of itself that its node can't measure, for
        work to be placed by, or null to declare nothing, which mycelium
        reads as unknown.
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
      '';
    };
  };

  config = lib.mkMerge [
    {
      assertions = [
        {
          assertion = linking -> cfg.realm != null;
          message = "jitsusama.mycelium.links needs jitsusama.mycelium.realm, the realm they link in.";
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
          assertion = lib.all (lib.types.strMatching "[a-z0-9-]+").check (lib.attrNames cfg.alsoJoins);
          message = "jitsusama.mycelium.alsoJoins names a realm in other than lowercase letters, digits and dashes.";
        }
      ];
      home.packages = [ package ];
    }
    (lib.mkIf (cfg.realm != null) {
      home.sessionVariables = {
        MYCELIUM_REALM = cfg.realm;
        # A pi started in another realm keeps that realm's folders apart.
        PI_HOME_REALM = cfg.realm;
      };
    })
    (lib.mkIf (cfg.declares != null) {
      # Where mycelium reads it: XDG_CONFIG_HOME's, or ~/.config's.
      xdg.configFile."mycelium/declared.json".text = builtins.toJSON (
        {
          inherit (cfg.declares) reach outlives taints;
          probes = lib.mapAttrs (
            _: probe: if builtins.isList probe then { run = probe; } else { connect = probe; }
          ) cfg.declares.probes;
          costs_money = cfg.declares.costsMoney;
        }
        // lib.optionalAttrs (cfg.agents != { }) { inherit (cfg) agents; }
        // lib.optionalAttrs (cfg.declares.within != null) { inherit (cfg.declares) within; }
        // lib.optionalAttrs (cfg.declares.hosts != [ ]) { inherit (cfg.declares) hosts; }
      );
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
        mycelium = linuxNode (describe cfg.realm) node cfg.signsInWith;
      }
      // lib.mapAttrs' (
        realm: words:
        lib.nameValuePair "mycelium-${realm}" (
          linuxNode (describe realm) words cfg.alsoJoins.${realm}.signsInWith
        )
      ) furtherNodes;
    })
    (lib.mkIf (isDarwin && serving) {
      launchd.agents = {
        mycelium = macNode node "mycelium";
      }
      // lib.mapAttrs' (
        realm: words: lib.nameValuePair "mycelium-${realm}" (macNode words "mycelium-${realm}")
      ) furtherNodes;
    })
  ];
}
