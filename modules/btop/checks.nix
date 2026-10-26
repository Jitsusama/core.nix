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

  # Escape backs out of btop a step at a time: with a process chosen by
  # Down, one Escape leaves btop running, and a second ends it, where
  # btop's own Escape opens its menu and never ends it.
  btop-backs-out-on-escape =
    let
      home = account workstation;
    in
    pkgs.runCommandLocal "btop-backs-out-on-escape"
      {
        nativeBuildInputs = [
          home.programs.btop.package
          pkgs.util-linux
        ];
      }
      ''
        export HOME=$PWD LANG=C.UTF-8
        # btop's keys, a second apart so each reads as its own: Down, then
        # as many Escapes as asked; then what btop did within 8 seconds.
        backs() {
          { sleep 2; printf '\033[B'; for _ in $(seq "$1"); do sleep 1; printf '\033'; done; sleep 9; } |
            script -q -c 'stty cols 120 rows 40; timeout 8 btop; echo "ended $?" > ended' /dev/null > /dev/null || true
          cat ended
        }
        once=$(backs 1)
        twice=$(backs 2)
        if [ "$once" != "ended 124" ]; then echo "one Escape with a process chosen: $once, not still running"; exit 1; fi
        if [ "$twice" != "ended 0" ]; then echo "two Escapes: $twice, not btop ended"; exit 1; fi
        touch $out
      '';
}
