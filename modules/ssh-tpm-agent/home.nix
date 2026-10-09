# ssh-tpm-agent holds SSH keys sealed in the TPM, so no private key ever sits
# on the disk, and it serves every SSH login and SSH-signed commit. When a key
# wants its PIN or a confirmation, the agent runs SSH_ASKPASS, which asks
# through bemenu, in the theme. The keys are made on the machine itself; see
# docs/installing.md.
{ lib, pkgs, ... }:
let
  # OpenSSH's askpass contract: the prompt is the only argument, the answer
  # goes to standard output, and SSH_ASKPASS_PROMPT=confirm asks a yes or no
  # question that the exit status answers.
  askpass = pkgs.writeShellApplication {
    name = "ssh-askpass";
    runtimeInputs = [ pkgs.bemenu ];
    text = ''
      if [ "''${SSH_ASKPASS_PROMPT:-}" = confirm ]; then
        answer=$(printf 'Allow\nDeny\n' | bemenu --prompt "$1") || exit 1
        [ "$answer" = Allow ]
      else
        bemenu --password indicator --prompt "$1" </dev/null
      fi
    '';
  };
in
{
  imports = [ ../bemenu/home.nix ];

  services.ssh-tpm-agent.enable = true;

  home.packages = [ askpass ];
  home.sessionVariables.SSH_ASKPASS = lib.getExe askpass;
  systemd.user.sessionVariables.SSH_ASKPASS = lib.getExe askpass;
}
