# The Linux console, which shows the boot's lines, the disk's PIN prompt and
# tuigreet should the greeter fail, in the colours of the theme the machine's
# own account wears, as the greeter does (../greetd/nixos.nix). The kernel
# takes the console's sixteen colours on its command line, so they hold from
# its first line. A machine without such an account, or without a theme for
# it, keeps the kernel's own.
{
  config,
  lib,
  options,
  ...
}:
let
  account = if options ? jitsusama.login.account then config.jitsusama.login.account else null;
  home =
    if account != null && options ? home-manager then
      config.home-manager.users.${account} or null
    else
      null;
  terminal = lib.attrByPath [ "jitsusama" "theme" "terminal" ] null home;

  # The console's order: the eight colours, then the bright eight, each as
  # six hex digits, since it has no transparency.
  eight = [
    "black"
    "red"
    "green"
    "yellow"
    "blue"
    "magenta"
    "cyan"
    "white"
  ];
  colours = map (name: lib.substring 1 6 terminal.${name}) (
    eight ++ map (name: "bright_${name}") eight
  );
in
{
  config = lib.mkIf (terminal != null) {
    console.colors = colours;
  };
}
