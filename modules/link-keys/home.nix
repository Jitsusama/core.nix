# A key sealed in a Linux machine's TPM for each realm the machine's node
# links in, besides the one the account's agents sign with, so a link in one
# realm can't open a machine in another (mycelium says what a realm is).
#
# Each realm's key has an ssh-tpm-agent of its own, holding only that key,
# which asks nothing, since a node signs in with nobody at the keyboard. It
# serves <realm>-link.sock under the runtime folder, which only that realm's
# node is given; no session is, and the key signs no commits. The key lives
# outside ~/.ssh, as Joel's signing key does, since ssh-tpm-agent's own agent
# loads every key there and would offer it to anything that asks. Like the
# others, jitsusama.tpmKeys names it for the guide to make on a machine it
# installed; on one installed before, `ssh-tpm-keygen -C "$USER@$(hostname)
# <realm> link" -f ~/.local/share/links/<realm>/<realm>-link` with an empty
# PIN makes it.
#
# A realm whose node signs in with the agents' key, as an employer's realm
# shared with its own machines may, needs no key here.
{
  config,
  lib,
  ...
}:
let
  cfg = config.jitsusama.linkKeys;
  sshTpmAgent = lib.getExe config.services.ssh-tpm-agent.package;
  folder = realm: "${config.xdg.dataHome}/links/${realm}";
  unit = realm: "${realm}-link";
in
{
  imports = [ ../ssh-tpm-agent/home.nix ];

  options.jitsusama.linkKeys = lib.mkOption {
    type = lib.types.listOf (lib.types.strMatching "[a-z][a-z0-9-]*");
    default = [ ];
    example = [ "personal" ];
    description = ''
      The realms whose node signs in to other machines with a key of its
      own, each served on <realm>-link.sock in the runtime folder by an agent
      holding only that key.
    '';
  };

  config = {
    # Each agent's folder, there and private before its key is made.
    systemd.user.tmpfiles.rules = map (realm: "d ${folder realm} 0700 - - -") cfg;

    systemd.user.sockets = lib.genAttrs (map unit cfg) (name: {
      Unit.Description = "The ${name} key's own ssh-tpm-agent, which asks nothing";
      Socket = {
        ListenStream = "%t/${name}.sock";
        SocketMode = "0600";
        Service = "${name}.service";
      };
      Install.WantedBy = [ "sockets.target" ];
    });

    systemd.user.services = lib.listToAttrs (
      map (realm: {
        name = unit realm;
        value = {
          Unit = {
            Description = "The ${unit realm} key's own ssh-tpm-agent, which asks nothing";
            Requires = [ "${unit realm}.socket" ];
            After = [ "${unit realm}.socket" ];
          };
          Service = {
            Environment = "SSH_TPM_AUTH_SOCK=%t/${unit realm}.sock";
            ExecStart = "${sshTpmAgent} --key-dir ${folder realm}";
            # As ssh-tpm-agent's own unit has it.
            SuccessExitStatus = 2;
          };
        };
      }) cfg
    );

    jitsusama.tpmKeys = map (realm: {
      file = "${folder realm}/${unit realm}";
      label = "${realm} link";
      pin = false;
    }) cfg;
  };
}
