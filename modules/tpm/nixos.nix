# The TPM holds Joel's everyday keys: the disk's, the SSH keys that log in and
# sign commits, and an age identity for secrets, none of which can leave the
# chip. Every account can reach it, since ssh-tpm-agent runs as the account,
# and tpm2-tools is there to read and test it.
{
  config,
  lib,
  pkgs,
  ...
}:
{
  security.tpm2 = {
    enable = true;
    # Points tpm2-tools and ssh-tpm-agent at the kernel's resource manager,
    # which shares the TPM between programs.
    tctiEnvironment.enable = true;
  };

  users.users = lib.mapAttrs (_: _: {
    extraGroups = [ config.security.tpm2.tssGroup ];
  }) config.home-manager.users;

  environment.systemPackages = [
    pkgs.tpm2-tools
    pkgs.age-plugin-tpm
  ];
}
