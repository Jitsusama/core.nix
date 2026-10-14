# LS_COLORS from the theme, through vivid, read by lsd, fd, tree and zsh's
# completion menus: most names in the text colour, directories in the
# strongest, what runs in the program colour, links in the voice colour and
# what nobody needs to read muted. vivid runs once when the account builds
# and the shell sources what it wrote, so no shell starts a program for it,
# and oh-my-zsh, finding LS_COLORS set, runs no dircolors either.
{
  config,
  lib,
  pkgs,
  ...
}:
let
  inherit (config.jitsusama.theme) roles;
  # vivid wants a style for every kind of file it knows, and a colour as
  # hex without its #.
  drawn = colour: { foreground = lib.removePrefix "#" colour; };
  plain = drawn roles.text;
  theme = {
    colors = { };
    core = {
      normal_text = { };
      regular_file = { };
      reset_to_normal = { };
      directory = drawn roles.strong // {
        font-style = "bold";
      };
      symlink = drawn roles.voice;
      multi_hard_link = { };
      fifo = drawn roles.machine;
      socket = drawn roles.machine;
      door = drawn roles.machine;
      block_device = drawn roles.machine;
      character_device = drawn roles.machine;
      broken_symlink = drawn roles.alert;
      missing_symlink_target = drawn roles.alert;
      setuid = drawn roles.warning;
      setgid = drawn roles.warning;
      file_with_capability = drawn roles.warning;
      sticky_other_writable = drawn roles.warning;
      other_writable = drawn roles.warning;
      sticky = { };
      executable_file = drawn roles.program;
    };
    text = plain // {
      special = drawn roles.strong;
      todo = drawn roles.strong;
      licenses = drawn roles.muted;
    };
    markup = plain;
    programming = plain;
    media = drawn roles.person;
    office = plain;
    archives = drawn roles.machine;
    executable = drawn roles.program;
    unimportant = drawn roles.muted;
  };
  colours = pkgs.runCommand "ls-colors.zsh" { nativeBuildInputs = [ pkgs.vivid ]; } ''
    echo "export LS_COLORS='$(vivid generate ${pkgs.writeText "theme.yml" (builtins.toJSON theme)})'" > $out
  '';
in
{
  imports = [ ../theme/home.nix ];

  programs.zsh.initContent = lib.mkOrder 520 "source ${colours}";
}
