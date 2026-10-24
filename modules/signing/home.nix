# Joel's commits and tags are signed with SSH keys sealed in the TPM, which
# ssh-tpm-agent holds. There are two, as the Macs keep in their Secure
# Enclave:
#
#   joel-signing   a PIN once a session, then a confirmation on every signature
#   agent-signing  never asks, for pi and the other harnesses
#
# tpm-sign, git's signing program, picks one each time it signs. When its
# standard input is a terminal and it can open its controlling terminal, a
# person is at the keyboard and Joel's key signs; otherwise a harness is
# driving git over pipes and the agent's key signs. git hands the signer its
# own standard input, so this is the input of `git commit` itself. A harness
# started from a terminal can still open /dev/tty, which is why both are
# checked. TPM_SIGNER=joel or TPM_SIGNER=agent decides it outright.
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
  cfg = config.jitsusama.signing;

  joelKey = "${config.xdg.dataHome}/signing/joel-signing";
  agentKey = "${config.home.homeDirectory}/.ssh/agent-signing";
  sshTpmAdd = lib.getExe' config.services.ssh-tpm-agent.package "ssh-tpm-add";

  # Signs with the key for whoever drives git (tpm-sign.sh), given both.
  tpmSign = pkgs.writeShellApplication {
    name = "tpm-sign";
    runtimeInputs = [ pkgs.openssh ];
    text = lib.concatLines [
      "joel_key=${lib.escapeShellArg joelKey} agent_key=${lib.escapeShellArg agentKey}"
      (builtins.readFile ./tpm-sign.sh)
    ];
  };
in
{
  imports = [
    ../git/home.nix
    ../ssh-tpm-agent/home.nix
  ];

  options.jitsusama.signing.allowedSigners = lib.mkOption {
    type = lib.types.listOf lib.types.str;
    default = [ ];
    example = [ ''joel@example.com namespaces="git" ecdsa-sha2-nistp256 AAAA...'' ];
    description = ''
      Lines of git's allowed_signers file: every key that has signed Joel's
      work, on every machine, so git can say whose a signature is. It's a
      register that only grows; a retired key gets valid-before="YYYYMMDD"
      rather than being removed, so what it signed still verifies.
    '';
  };

  config = {
    home.packages = [ tpmSign ];

    # Last, so an include another tool writes, such as dev's YubiKey
    # settings, can't take signing back.
    programs.git.includes = lib.mkAfter [
      {
        # Recursive, since a plain // would put the signers' gpg section in
        # place of the one that says how to sign.
        contents =
          lib.recursiveUpdate
            {
              gpg.format = "ssh";
              gpg.ssh.program = lib.getExe tpmSign;
              user.signingKey = "${joelKey}.pub";
              commit.gpgSign = true;
              tag.gpgSign = true;
            }
            (
              lib.optionalAttrs (cfg.allowedSigners != [ ]) {
                gpg.ssh.allowedSignersFile = "${config.xdg.configHome}/git/allowed_signers";
              }
            );
      }
    ];

    xdg.configFile."git/allowed_signers" = lib.mkIf (cfg.allowedSigners != [ ]) {
      text = lib.concatLines cfg.allowedSigners;
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
  };
}
