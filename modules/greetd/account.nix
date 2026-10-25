# The account a machine signs in as (jitsusama.login.account, ./nixos.nix),
# its home configuration and its theme, each null where the machine names no
# such account, has no home-manager, or has no theme for it. The system's own
# surfaces read it to wear the account's look: the console, kmscon, Plymouth,
# the fonts and the greeter.
{
  config,
  lib,
  options,
}:
rec {
  account = if options ? jitsusama.login.account then config.jitsusama.login.account else null;
  home =
    if account != null && options ? home-manager then
      config.home-manager.users.${account} or null
    else
      null;
  theme = lib.attrByPath [ "jitsusama" "theme" ] null home;
}
