# LS_COLORS from the theme, through vivid, read by lsd, fd, tree and zsh's
# completion menus: most names in the text colour, directories in the
# strongest, what runs in the program colour, links in the voice colour and
# what nobody needs to read muted. vivid runs once when the account builds
# and the shell sources what it wrote, so no shell starts a program for it.
{
  config,
  lib,
  pkgs,
  ...
}:
let
  inherit (config.jitsusama.theme) colours;
  # vivid wants a style for every kind of file it knows, and a colour as
  # hex without its #.
  drawn = colour: { foreground = lib.removePrefix "#" colour; };
  plain = drawn colours.text;
  theme = {
    colors = { };
    core = {
      normal_text = { };
      regular_file = { };
      reset_to_normal = { };
      directory = drawn colours.strong // {
        font-style = "bold";
      };
      symlink = drawn colours.voice;
      multi_hard_link = { };
      fifo = drawn colours.machine;
      socket = drawn colours.machine;
      door = drawn colours.machine;
      block_device = drawn colours.machine;
      character_device = drawn colours.machine;
      broken_symlink = drawn colours.alert;
      missing_symlink_target = drawn colours.alert;
      setuid = drawn colours.warning;
      setgid = drawn colours.warning;
      file_with_capability = drawn colours.warning;
      sticky_other_writable = drawn colours.warning;
      other_writable = drawn colours.warning;
      sticky = { };
      executable_file = drawn colours.program;
    };
    text = plain // {
      special = drawn colours.strong;
      todo = drawn colours.strong;
      licenses = drawn colours.muted;
    };
    markup = plain;
    programming = plain;
    media = drawn colours.person;
    office = plain;
    archives = drawn colours.machine;
    executable = drawn colours.program;
    unimportant = drawn colours.muted;
  };
  file = pkgs.writeText "theme.yml" (builtins.toJSON theme);
  lsColours = pkgs.runCommand "ls-colors.zsh" { nativeBuildInputs = [ pkgs.vivid ]; } ''
    echo "export LS_COLORS='$(vivid generate ${file})'" > $out
  '';
in
{
  imports = [ ../theme/home.nix ];

  programs.zsh.initContent = lib.mkOrder 520 "source ${lsColours}";
}
