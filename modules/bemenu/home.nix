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
  inherit (config.jitsusama.theme) roles font shape;

  flags = lib.concatStringsSep " " [
    "--fn '${font.mono.family} ${font.mono.size}'"
    "--center --width-factor 0.3 --list 8"
    "--border ${toString shape.border} --border-radius ${toString shape.radius}"
    "--hp ${toString shape.gap} --line-height ${toString (shape.gap * 3)}"
    "--bdr '${roles.accent}'"
    "--tb '${roles.background}' --tf '${roles.accent}'"
    "--fb '${roles.background}' --ff '${roles.text}'"
    "--cb '${roles.strong}' --cf '${roles.background}'"
    "--nb '${roles.background}' --nf '${roles.text}'"
    "--ab '${roles.background}' --af '${roles.text}'"
    "--hb '${roles.selection}' --hf '${roles.strong}'"
    "--sb '${roles.selection}' --sf '${roles.strong}'"
    "--fbb '${roles.background}' --fbf '${roles.muted}'"
    "--scb '${roles.background}' --scf '${roles.muted}'"
  ];
in
{
  imports = [ ../theme/home.nix ];

  home.packages = [ pkgs.bemenu ];

  home.sessionVariables.BEMENU_OPTS = flags;
  systemd.user.sessionVariables.BEMENU_OPTS = flags;
}
