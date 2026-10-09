# bemenu draws the prompts that ask Joel for a PIN or a yes: the askpass that
# SSH and ssh-tpm-agent call. Every bemenu reads its look from BEMENU_OPTS, so
# this sets it from jitsusama.theme for shells and for the services systemd
# starts.
# bemenu's own guide to the flags: bemenu --help.
{
  config,
  lib,
  pkgs,
  ...
}:
let
  inherit (config.jitsusama.theme) colors font shape;

  flags = lib.concatStringsSep " " [
    "--fn '${font.family} ${font.size}'"
    "--center --width-factor 0.3 --list 8"
    "--border ${toString shape.border} --border-radius ${toString shape.radius}"
    "--hp ${toString shape.gap} --line-height ${toString (shape.gap * 3)}"
    "--bdr '${colors.accent}'"
    "--tb '${colors.background}' --tf '${colors.accent}'"
    "--fb '${colors.background}' --ff '${colors.foreground}'"
    "--cb '${colors.bright_foreground}' --cf '${colors.background}'"
    "--nb '${colors.background}' --nf '${colors.foreground}'"
    "--ab '${colors.background}' --af '${colors.foreground}'"
    "--hb '${colors.selection}' --hf '${colors.bright_foreground}'"
    "--sb '${colors.selection}' --sf '${colors.bright_foreground}'"
    "--fbb '${colors.background}' --fbf '${colors.muted}'"
    "--scb '${colors.background}' --scf '${colors.muted}'"
  ];
in
{
  imports = [ ../theme/home.nix ];

  home.packages = [ pkgs.bemenu ];

  home.sessionVariables.BEMENU_OPTS = flags;
  systemd.user.sessionVariables.BEMENU_OPTS = flags;
}
