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

  declaring = {
    reach = [ "internet" ];
    probes.home = "192.0.2.1:53";
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
in
{
  # A workstation that names machines to link with runs the node in its
  # realm, dialling each one: a user service on Linux, an agent on a Mac. One
  # that names none only has the program, with its realm set for the far
  # end of a link, and one in no realm has neither a realm nor a node. On
  # Linux the node signs in with the agents' key, which asks nothing.
  mycelium-links-what-a-machine-names =
    holds "mycelium-links-what-a-machine-names"
      "A machine's links don't each get dialled in its realm, Linux's node signs in with a key that may ask for a PIN, or a machine that names none runs a node anyway."
      (
        lib.hasInfix "--realm personal node --ssh one --ssh two" linuxNode
        && linuxAgent == [ "SSH_AUTH_SOCK=%t/agent-signing.sock" ]
        &&
          lib.drop 1 macNode == [
            "--realm"
            "personal"
            "node"
          ]
          ++ ssh
        && (mac links).launchd.agents.mycelium.enable
        && !(answering.systemd.user.services ? mycelium)
        && answering.home.sessionVariables.MYCELIUM_REALM == "personal"
        && !(apart.home.sessionVariables ? MYCELIUM_REALM)
        && lib.any (package: (package.pname or "") == "mycelium") apart.home.packages
      );

  # A machine that declares what its node can't measure has it written where
  # mycelium reads it, in mycelium's words; Linux's held sessions outlive
  # their window, the node and a suspend, and a Mac claims none yet. One that
  # declares nothing has no file, which mycelium reads as unknown.
  mycelium-declares-what-a-machine-says =
    holds "mycelium-declares-what-a-machine-says"
      "What a machine declares, its probes included, isn't written in mycelium's shape, a held session's outlives isn't each system's, or a machine declaring nothing gets a file anyway."
      (
        declared linux == {
          reach = [ "internet" ];
          probes.home.connect = "192.0.2.1:53";
          outlives = [
            "window_close"
            "node_restart"
            "sleep"
          ];
          costs_money = false;
          inherit (declaring) taints;
        }
        && (declared mac).outlives == [ ]
        && !(apart.xdg.configFile ? "mycelium/declared.json")
      );
}
