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

  # btop names its window as it starts, keeping the terminal's own title on
  # the terminal's stack, and gives that title back as it ends, even when
  # it's told to end, so the launcher lists btop's keys only while it runs.
  btop-names-its-window =
    let
      home = account workstation;
    in
    pkgs.runCommandLocal "btop-names-its-window"
      {
        nativeBuildInputs = [
          home.programs.btop.package
          pkgs.util-linux
        ];
      }
      ''
        export HOME=$PWD LANG=C.UTF-8
        script -q -c 'stty cols 120 rows 40; timeout 3 btop' drawn > /dev/null || true
        named=$(printf '\033[22;0t\033]2;btop\033\\')
        given=$(printf '\033[23;0t')
        if ! grep -q -F "$named" drawn; then
          echo "btop didn't keep the terminal's title and name its window btop."
          exit 1
        fi
        if ! tail -c 200 drawn | grep -q -F "$given"; then
          echo "btop didn't give the terminal its title back as it ended."
          exit 1
        fi
        touch $out
      '';

  # Escape backs out of btop a step at a time: alone it ends btop, and with
  # a process chosen by Down, one leaves btop running and a second ends it,
  # where btop's own Escape opens its menu and never ends it. The keys go
  # through tmux, as a terminal sends them.
  btop-backs-out-on-escape =
    let
      home = account workstation;
    in
    pkgs.runCommandLocal "btop-backs-out-on-escape"
      {
        nativeBuildInputs = [
          home.programs.btop.package
          pkgs.tmux
        ];
      }
      ''
        export HOME=$PWD LANG=C.UTF-8 TMUX_TMPDIR=$PWD
        # btop in a terminal of its own, given these keys a second apart;
        # then whether it has ended, waiting up to ten seconds for a btop
        # expected to end so a busy machine can't fail it.
        after() {
          expected=$1
          shift
          rm -f ended
          tmux new-session -d -x 120 -y 40 "btop; touch $PWD/ended; sleep 60"
          sleep 2
          for key in "$@"; do
            tmux send-keys "$key"
            sleep 1
          done
          for _ in $(seq 100); do
            [ -e ended ] || [ "$expected" = running ] || { sleep 0.1; continue; }
            break
          done
          if [ -e ended ]; then got=ended; else got=running; fi
          tmux kill-server
          if [ "$got" != "$expected" ]; then
            echo "after $*, btop is $got, not $expected"
            exit 1
          fi
        }
        after ended Escape
        after running Down Escape
        after ended Down Escape Escape
        touch $out
      '';
}
