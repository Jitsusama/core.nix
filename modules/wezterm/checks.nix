# The checks of WezTerm, which checks.nix gathers with every other folder's;
# its opening comment says what each kind of check proves.
{
  account,
  bareNixos,
  lib,
  onLinux,
  pkgs,
  self,
  ...
}:
let
  # WezTerm's home half alone, on a machine with no fonts of its own, so the
  # only faces it can find are the ones its own files name.
  alone = account (bareNixos [
    { home-manager.users.someone.imports = [ self.homeModules.wezterm ]; }
  ]);
in
onLinux {
  # WezTerm finds the theme's monospaced face in the files its configuration
  # names, rather than only among the system's fonts: a Mac's switch swaps
  # the system's font folder while WezTerm reloads its new configuration, and
  # a face missing for that moment raised a window on the screen
  # (tests/wezterm-finds-its-font-itself.sh).
  wezterm-finds-its-font-itself = pkgs.runCommandLocal "wezterm-finds-its-font-itself" { } ''
    ${lib.getExe pkgs.bash} ${./tests/wezterm-finds-its-font-itself.sh} ${
      lib.escapeShellArgs [
        alone.home-files
        (lib.getExe pkgs.wezterm)
        alone.jitsusama.theme.font.mono.family
      ]
    }
    touch $out
  '';
}
