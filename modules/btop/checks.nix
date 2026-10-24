# The checks of btop, which checks.nix gathers with every other folder's;
# its opening comment says what each kind of check proves.
{
  account,
  lib,
  onLinux,
  pkgs,
  workstation,
  ...
}:
onLinux {
  # btop draws in the theme with the files an account gets: a theme it can't
  # find falls back to btop's own without a word, so the check looks for the
  # theme's text colour in what btop writes to its terminal.
  btop-draws-in-the-theme =
    let
      home = account workstation;
      files = home.xdg.configFile;
      foreground = lib.concatMapStringsSep ";" (byte: toString (lib.fromHexString byte)) (
        builtins.match "#(..)(..)(..)" home.jitsusama.theme.roles.text
      );
    in
    pkgs.runCommandLocal "btop-draws-in-the-theme"
      {
        nativeBuildInputs = [
          home.programs.btop.package
          pkgs.util-linux
        ];
      }
      ''
        export HOME=$PWD LANG=C.UTF-8
        mkdir -p .config/btop/themes
        cp ${files."btop/btop.conf".source} .config/btop/btop.conf
        cp ${files."btop/themes/theme.theme".source} .config/btop/themes/theme.theme
        script -q -c 'stty cols 120 rows 40; timeout 3 btop' drawn > /dev/null || true
        if ! grep -q '38;2;${foreground}m' drawn; then cat -v drawn | head -c 2000; exit 1; fi
        touch $out
      '';
}
