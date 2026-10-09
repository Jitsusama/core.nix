# 1Password, with the two things its desktop app needs from the system: a
# helper in the onepassword group, which its Chrome extension talks to, and a
# polkit policy naming the accounts that may unlock it with their own password,
# which every account on the machine may.
{ config, lib, ... }:
{
  programs._1password-gui = {
    enable = true;
    polkitPolicyOwners = lib.attrNames (
      lib.filterAttrs (_: user: user.isNormalUser) config.users.users
    );
  };
}
