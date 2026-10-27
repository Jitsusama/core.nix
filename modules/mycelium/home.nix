# mycelium carries the harness mesh between a realm's machines, so a session
# on one shows on the others and a message reaches it wherever it runs. Every
# workstation gets the program, which the other end of a link runs over SSH.
# A machine joins a realm by naming it, and one that keeps links also runs
# the node, dialling each machine it names: a user service on Linux, and on a
# Mac an agent in the login session. A machine sets both in its own
# repository:
#
#   jitsusama.mycelium.realm = "personal";
#   jitsusama.mycelium.links = [ "other-machine" ];
#
# Each link is an SSH destination the machine can already reach without a
# prompt, its host key pinned. On Linux the node signs in with the agents'
# key from the TPM (../tpm-signing), which asks nothing, so the far machine
# authorizes that key: Joel's own asks for its PIN, and the node dialling at
# login would hold every agent's signature up until he answered.
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
  agentsKey = config.systemd.user.sockets ? agent-signing;
  node = [
    (lib.getExe package)
    "--realm"
    cfg.realm
    "node"
  ]
  ++ lib.concatMap (machine: [
    "--ssh"
    machine
  ]) cfg.links;
in
{
  options.jitsusama.mycelium = {
    realm = lib.mkOption {
      type = lib.types.nullOr (lib.types.strMatching "[A-Za-z0-9_-]+");
      default = null;
      example = "personal";
      description = ''
        The realm this machine belongs to, the set of machines whose nodes
        link with nothing crossing between realms, or null for a machine in
        none. A machine keeps the realm it first joined, so changing it later
        is refused until mycelium's state is cleared.
      '';
    };
    links = lib.mkOption {
      type = lib.types.listOf lib.types.str;
      default = [ ];
      example = [ "other-machine" ];
      description = ''
        The machines of the realm this machine dials and keeps linked, as SSH
        destinations. Two machines link once, so only one of each pair names
        the other. With none, the machine only answers links.
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
      ];
      home.packages = [ package ];
    }
    (lib.mkIf (cfg.realm != null) {
      home.sessionVariables.MYCELIUM_REALM = cfg.realm;
    })
    (lib.mkIf (isLinux && linking) {
      systemd.user.services.mycelium = {
        Unit = {
          Description = "Carries the harness mesh between the machines of the ${cfg.realm} realm";
          Wants = lib.optional agentsKey "agent-signing.socket";
          After = lib.optional agentsKey "agent-signing.socket";
        };
        Service = {
          ExecStart = lib.escapeShellArgs node;
          Environment = lib.optional agentsKey "SSH_AUTH_SOCK=%t/agent-signing.sock";
          # The node lets go of the sessions it carried when told to stop,
          # and redials a dropped link itself, so only its own failure
          # restarts it.
          Restart = "on-failure";
          RestartSec = "5s";
        };
        Install.WantedBy = [ "default.target" ];
      };
    })
    (lib.mkIf (isDarwin && linking) {
      launchd.agents.mycelium = {
        enable = true;
        config = {
          ProgramArguments = node;
          # The system's own ssh, which reads the machine's SSH config and
          # the login session's agent.
          EnvironmentVariables.PATH = "/usr/bin:/bin";
          RunAtLoad = true;
          # Started again whenever it stops, but not more than every 30 s.
          KeepAlive = true;
          ThrottleInterval = 30;
          StandardErrorPath = "${config.home.homeDirectory}/Library/Logs/mycelium.log";
        };
      };
    })
  ];
}
