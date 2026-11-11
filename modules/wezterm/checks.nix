# The checks of WezTerm, which checks.nix gathers with every other folder's;
# its opening comment says what each kind of check proves.
{
  account,
  bareNixos,
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
  files = alone.xdg.configFile;
in
onLinux {
  # WezTerm finds the theme's monospaced face in the files its configuration
  # names, rather than only among the system's fonts: a Mac's switch swaps
  # the system's font folder while WezTerm reloads its new configuration, and
  # a face missing for that moment raised a window on the screen. With no
  # system fonts here, WezTerm only draws in the face if it finds it itself.
  wezterm-finds-its-font-itself =
    pkgs.runCommandLocal "wezterm-finds-its-font-itself"
      {
        nativeBuildInputs = [ pkgs.wezterm ];
      }
      ''
        export HOME=$PWD XDG_CACHE_HOME=$PWD/cache XDG_RUNTIME_DIR=$PWD
        # Where WezTerm looks, so its configuration finds theme.lua beside it.
        mkdir -p .config/wezterm
        cp ${files."wezterm/wezterm.lua".source} .config/wezterm/wezterm.lua
        cp ${files."wezterm/theme.lua".source} .config/wezterm/theme.lua
        wezterm ls-fonts --text a > drawn 2>&1 || true
        cat drawn
        if grep -q 'runtime error' drawn; then
          echo "WezTerm couldn't read its configuration."
          exit 1
        fi
        if ! grep -q -F 'wezterm.font("${alone.jitsusama.theme.font.mono.family}"' drawn; then
          echo "WezTerm drew in another face than the theme's, so it can't find its own."
          exit 1
        fi
        touch $out
      '';
}
