# 1Password, with the three things its desktop app needs from the system: a
# helper in the onepassword group, which its Chrome extension talks to; its
# command-line tool, setgid in the onepassword-cli group, the only way the
# app will answer it once the app's own setting lets it, asking the person
# before it reads the vault; and a polkit policy naming the accounts that
# may unlock it with their own password, which every account on the machine
# may.
{ config, lib, ... }:
{
  programs._1password.enable = true;

  programs._1password-gui = {
    enable = true;
    polkitPolicyOwners = lib.attrNames (
      lib.filterAttrs (_: user: user.isNormalUser) config.users.users
    );
  };
}
