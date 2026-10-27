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
}
