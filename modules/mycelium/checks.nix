# The checks of mycelium, which checks.nix gathers with every other folder's;
# its opening comment says what each kind of check proves.
{
  account,
  bareDarwin,
  bareNixos,
  holds,
  lib,
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
  macNode = (mac links).launchd.agents.mycelium.config.ProgramArguments;
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
  declared =
    bare:
    builtins.fromJSON (bare { declares = declaring; }).xdg.configFile."mycelium/declared.json".text;
  # A machine within another, and the host it runs in, each naming the other.
  declaredAs =
    extra:
    builtins.fromJSON
      (linux { declares = declaring // extra; }).xdg.configFile."mycelium/declared.json".text;
  inVm = declaredAs {
    within = {
      machine = "host-machine";
      file = "/run/host/said.json";
    };
  };
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
in
{
  # A workstation in a realm runs the node in it, dialling each machine it
  # names: a user service on Linux, an agent on a Mac. One that names none
  # runs it dialling nothing, since its node still holds its sessions and
  # starts what others ask of it, and one in no realm has neither a realm
  # nor a node. On Linux the node signs in with the agents' key, which asks
  # nothing, unless its realm names a key of its own; on a Mac what it
  # starts signs commits with that key, since nobody at another machine can
  # touch this one's sensor.
  mycelium-links-what-a-machine-names =
    holds "mycelium-links-what-a-machine-names"
      "A machine's links don't each get dialled in its realm, Linux's node signs in with a key that may ask for a PIN or not the one its realm names, a Mac's node starts sessions whose commits wait on its sensor, a machine in a realm naming none runs no node, or one in none runs one."
      (
        lib.hasInfix "--realm personal node --ssh one --ssh two" linuxNode
        && linuxAgent == [ "SSH_AUTH_SOCK=%t/agent-signing.sock" ]
        &&
          (linux (links // { signsInWith = "personal-link"; }))
          .systemd.user.services.mycelium.Service.Environment == [ "SSH_AUTH_SOCK=%t/personal-link.sock" ]
        &&
          lib.drop 1 macNode == [
            "--realm"
            "personal"
            "node"
          ]
          ++ ssh
          ++ [
            "--serves"
            "${(mac links).xdg.configHome}/mycelium/serves-personal.json"
          ]
        && (mac links).launchd.agents.mycelium.enable
        && (mac links).launchd.agents.mycelium.config.EnvironmentVariables.SIGN_AS == "agent"
        && lib.hasInfix "--realm personal node" (
          toString answering.systemd.user.services.mycelium.Service.ExecStart
        )
        && !(lib.hasInfix "--ssh" (toString answering.systemd.user.services.mycelium.Service.ExecStart))
        && (mac { realm = "personal"; }).launchd.agents.mycelium.enable
        && !(apart.systemd.user.services ? mycelium)
        && answering.home.sessionVariables.MYCELIUM_REALM == "personal"
        && !(apart.home.sessionVariables ? MYCELIUM_REALM)
        && lib.any (package: (package.pname or "") == "mycelium") apart.home.packages
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
        && lib.hasSuffix "--realm home node --ssh one --serves /home/someone/.config/mycelium/serves-home.json" (
          toString bothLinux.mycelium.Service.ExecStart
        )
        && bothLinux.mycelium.Service.Environment == [ "SSH_AUTH_SOCK=%t/agent-signing.sock" ]
        && lib.hasInfix "/.local/state/pi/agentic-harness.pi/realms/other/steer --ssh two --serves " (
          toString (mac both).launchd.agents.mycelium-other.config.ProgramArguments
        )
        && (linux both).home.sessionVariables.PI_HOME_REALM == "home"
        && (linux both).home.sessionVariables.MYCELIUM_REALM == "home"
      );

  # Each realm's node serves the harness's files of its realm, as the
  # harness keeps them: the home realm's in the harness's usual folders, a
  # further realm's in its own, and the machine's waiting and verdicts on
  # the home realm's node alone, so nothing is heard twice.
  mycelium-serves-the-harness-files =
    holds "mycelium-serves-the-harness-files"
      "A realm's node doesn't serve its realm's quests and restore list from that realm's folders, or the machine's waiting and verdicts aren't served by the home realm's node alone."
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
          restore = {
            file = "${state}/pi/restore.json";
            records = "sessions";
            key = "id";
          };
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
        }
        &&
          serves "other" == {
            quest = {
              file = "${state}/pi/agentic-harness.pi/realms/other/quest-workflow/quests.json";
              records = "quests";
              key = "id";
            };
            restore = {
              file = "${state}/pi/agentic-harness.pi/realms/other/restore.json";
              records = "sessions";
              key = "id";
            };
          }
        && lib.hasSuffix "--serves /home/someone/.config/mycelium/serves-other.json" otherNode
        && !(apart.xdg.configFile ? "mycelium/serves-home.json")
      );

  # A machine that declares what its node can't measure has it written where
  # mycelium reads it, in mycelium's words; Linux's held sessions outlive
  # their window, the node and a suspend, and a Mac claims none yet. One that
  # declares nothing has no file, which mycelium reads as unknown.
  mycelium-declares-what-a-machine-says =
    holds "mycelium-declares-what-a-machine-says"
      "What a machine declares, its probes of either kind included, isn't written in mycelium's shape, a held session's outlives isn't each system's, or a machine declaring nothing gets a file anyway."
      (
        declared linux == {
          reach = [ "internet" ];
          probes.home.connect = "192.0.2.1:53";
          probes.office.run = declaring.probes.office;
          outlives = [
            "window_close"
            "node_restart"
            "sleep"
          ];
          costs_money = false;
          inherit (declaring) taints;
        }
        && (declared mac).outlives == [ ]
        && !((declared linux) ? within)
        && !((declared linux) ? hosts)
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
          (builtins.fromJSON (saying { home = "an-agent"; }).xdg.configFile."mycelium/declared.json".text)
          .agents == {
            home = "an-agent";
          }
        && !refusesAgents (saying {
          home = "an-agent";
        })
        && refusesAgents (saying {
          elsewhere = "an-agent";
        })
      );

  # A machine's system pins each key that links to it, on either system, to
  # its own realm and machine, with a terminal for an attach; one naming
  # none authorizes nothing of mycelium's.
  mycelium-pins-the-keys-that-link =
    holds "mycelium-pins-the-keys-that-link"
      "A link key a machine names isn't pinned to `mycelium ssh` as the realm and machine it names, on NixOS or nix-darwin, can't have a terminal for an attach, or a machine naming none authorizes a key."
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
        in
        pinnedOn bareNixos self.nixosModules.workstation
        && pinnedOn bareDarwin self.darwinModules.workstation
        && keys bareNixos self.nixosModules.workstation { } == [ ]
      );
}
