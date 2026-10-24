# Joel's signing keys sealed in a Linux machine's TPM, which ssh-tpm-agent
# holds; ../signing says how a key is picked and what git is told. Joel's key
# asks for its PIN once a session, then for a confirmation on every
# signature.
#
# ssh-tpm-agent loads every key under ~/.ssh when it starts, without asking
# to confirm, so Joel's key lives outside it and the signing-key unit adds it
# with confirmation whenever the agent starts. docs/installing.md makes both.
{
  config,
  lib,
  pkgs,
  ...
}:
let
  joelKey = "${config.xdg.dataHome}/signing/joel-signing";
  agentKey = "${config.home.homeDirectory}/.ssh/agent-signing";
  sshTpmAdd = lib.getExe' config.services.ssh-tpm-agent.package "ssh-tpm-add";
in
{
  imports = [
    ../signing/home.nix
    ../ssh-tpm-agent/home.nix
  ];

  # ssh-keygen signs through the TPM's agent by each key's public half,
  # whatever agent SSH itself is using.
  jitsusama.signing.signer = {
    name = "tpm-sign";
    sshKeygen = lib.getExe' pkgs.openssh "ssh-keygen";
    joelKey = "${joelKey}.pub";
    agentKey = "${agentKey}.pub";
    makesKeys = "docs/installing.md's step 9";
    environment.SSH_AUTH_SOCK = "\${SSH_TPM_AUTH_SOCK:-$XDG_RUNTIME_DIR/ssh-tpm-agent.sock}";
  };

  systemd.user.services.signing-key = {
    Unit = {
      Description = "Joel's signing key in ssh-tpm-agent, confirmed on every use";
      After = [ "ssh-tpm-agent.service" ];
      PartOf = [ "ssh-tpm-agent.service" ];
      ConditionPathExists = "${joelKey}.tpm";
    };
    Service = {
      Type = "oneshot";
      RemainAfterExit = true;
      ExecStart = "${sshTpmAdd} -c ${joelKey}.tpm";
    };
    Install.WantedBy = [ "ssh-tpm-agent.service" ];
  };
}
