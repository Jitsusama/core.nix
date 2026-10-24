# The checks of tuple, which checks.nix gathers with every other folder's;
# its opening comment says what each kind of check proves.
{
  account,
  graphical,
  holds,
  lib,
  onLinux,
  pkgs,
  ...
}:
onLinux {
  # Tuple shares a screen through the portal: the switch writes the setting
  # where Tuple keeps it, and Tuple, which ignores a folder it doesn't know
  # and a value not written as it writes one, has to read it back. So the
  # check runs the switch's own steps, with the data folder moved here.
  tuple-shares-through-the-portal =
    let
      home = account graphical;
      tuple = lib.findFirst (package: lib.getName package == "tuple") null home.home.packages;
      data = home.xdg.dataHome;
    in
    pkgs.runCommandLocal "tuple-shares-through-the-portal" { nativeBuildInputs = [ tuple ]; } ''
      export HOME=$PWD XDG_DATA_HOME=$PWD/data XDG_RUNTIME_DIR=$PWD/run
      mkdir -m 700 run
      sed -n '\|^run .*${lib.escapeRegex data}/tuple/|{s|^run ||;s|${data}|'"$PWD"'/data|g;p}' \
        ${home.home.activationPackage}/activate > steps
      sh -e steps
      tuple settings > said 2>&1 || true
      tuple stop > /dev/null 2>&1 || true
      if ! grep -q '^capture | portal |' said; then cat said; exit 1; fi
      touch $out
    '';

  # Tuple's daemon writes its log afresh as it starts, so the service keeps
  # the last three first: run here five times over a folder standing in
  # for Tuple's, then once more with an empty log, which mustn't push the
  # oldest out.
  tuple-keeps-its-logs =
    let
      home = account graphical;
      keeps = home.systemd.user.services.tuple.Service.ExecStartPre;
      data = "${home.xdg.dataHome}/tuple/0";
    in
    pkgs.runCommandLocal "tuple-keeps-its-logs" { } ''
      mkdir -p data
      sed 's|${data}|'"$PWD"'/data|g' ${keeps} > keeps
      chmod +x keeps
      grep -q "$PWD/data" keeps
      ./keeps
      for n in 1 2 3 4 5; do
        echo "log $n" > data/log.txt
        ./keeps
      done
      : > data/log.txt
      ./keeps
      said=$(for n in 1 2 3; do cat data/log.$n.txt; done)
      if [ "$said" != "$(printf 'log 5\nlog 4\nlog 3')" ] || [ -e data/log.4.txt ]; then
        echo "kept, newest first:"
        head data/log.*.txt
        exit 1
      fi
      touch $out
    '';

  # WebRTC inside Tuple asks real-time priorities in an order the account's
  # limit inverts, granting only its low-priority thread one, so the daemon
  # is given none, and comes first by its share and its nice value alone.
  tuple-takes-no-real-time-priority =
    let
      service = (account graphical).systemd.user.services.tuple.Service;
    in
    holds "tuple-takes-no-real-time-priority"
      "Tuple's daemon may take a real-time priority, which only its low-priority thread gets, ahead of its sound and the desktop."
      ((service.LimitRTPRIO or null) == 0 && service.Nice < 0 && service.CPUWeight > 100);

  # The tuple command completes in zsh with Tuple's own completion, which
  # only shows when Tab offers nothing, so the check completes commands
  # in a real line editor with the real tuple (tests/tuple-completion.zsh).
  tuple-completes-in-zsh =
    let
      home = account graphical;
      tuple = lib.findFirst (package: lib.getName package == "tuple") null home.home.packages;
      completion = lib.findFirst (
        package: lib.getName package == "tuple-zsh-completion"
      ) null home.home.packages;
    in
    pkgs.runCommandLocal "tuple-completes-in-zsh"
      {
        nativeBuildInputs = [
          tuple
          pkgs.zsh
        ];
      }
      ''
        export HOME=$PWD XDG_DATA_HOME=$PWD/data XDG_RUNTIME_DIR=$PWD/run
        mkdir -m 700 run
        zsh -f ${../../tests/tuple-completion.zsh} ${completion}/share/zsh/site-functions/_tuple
        tuple stop > /dev/null 2>&1 || true
        touch $out
      '';

  # A call link opened in a browser comes back as tuple://join, which only
  # the desktop entry's command takes, so the check runs it as the entry
  # does, with stand-ins for tuple and notify-send, over links like the
  # page's and ones it mustn't run. xdg-open finds the entry by what it
  # says it opens, beside a list of defaults a browser wrote, which names
  # none for Tuple's links; gio, and so the portal, are asked too, and
  # find it only through niri's own list.
  tuple-opens-its-links =
    let
      home = account graphical;
      tuple = lib.findFirst (package: lib.getName package == "tuple") null home.home.packages;
      entry = home.home.file."${home.xdg.dataHome}/applications/tuple.desktop".text;
      defaults = home.xdg.configFile."niri-mimeapps.list".text;
    in
    pkgs.runCommandLocal "tuple-opens-its-links"
      {
        nativeBuildInputs = [
          pkgs.xdg-utils
          pkgs.glib.bin
        ];
      }
      ''
        mkdir -p data/applications config
        printf '%s' ${lib.escapeShellArg entry} > tuple.desktop
        cp tuple.desktop data/applications/
        printf '[Default Applications]\nx-scheme-handler/https=browser.desktop\n' \
          > config/mimeapps.list
        printf '%s' ${lib.escapeShellArg defaults} > config/niri-mimeapps.list
        found=$(HOME=$PWD XDG_DATA_HOME=$PWD/data XDG_DATA_DIRS=$PWD/none \
          XDG_CONFIG_HOME=$PWD/config XDG_CURRENT_DESKTOP=niri \
          xdg-mime query default x-scheme-handler/tuple)
        if [ "$found" != tuple.desktop ]; then
          echo "xdg-open opens Tuple's links with '$found'"
          exit 1
        fi
        said=$(HOME=$PWD XDG_DATA_HOME=$PWD/data XDG_DATA_DIRS=$PWD/none \
          XDG_CONFIG_HOME=$PWD/config XDG_CONFIG_DIRS=$PWD/none XDG_CURRENT_DESKTOP=niri \
          gio mime x-scheme-handler/tuple 2>&1)
        if ! printf '%s\n' "$said" \
          | grep -qxF 'Default application for “x-scheme-handler/tuple”: tuple.desktop' \
          || printf '%s\n' "$said" | grep -q WARNING; then
          echo "gio opens Tuple's links with: $said"
          exit 1
        fi
        opens=$(sed -n 's|^Exec=\(.*\) %u$|\1|p' tuple.desktop)
        sed -e 's|${lib.getExe tuple}|'"$PWD"'/tuple|g' \
          -e 's|/nix/store/[^ ]*/bin/notify-send|'"$PWD"'/notify-send|g' "$opens" > opens
        for stand_in in tuple notify-send; do
          printf '#!/bin/sh\necho "%s $*" > "%s/asked"\n' "$stand_in" "$PWD" > "$stand_in"
        done
        chmod +x opens tuple notify-send

        failed=0
        opening() {
          rm -f asked
          ./opens "$1"
          if [ "$(cat asked)" != "$2" ]; then
            echo "'$1' asked '$(cat asked)', not '$2'"
            failed=1
          fi
        }
        opening 'tuple://join?call_url_slug=U7BVv2' 'tuple call join https://tuple.app/c/U7BVv2'
        opening 'tuple://join?from=web&call_url_slug=a_b-9' 'tuple call join https://tuple.app/c/a_b-9'
        opening 'tuple://join?call_url_slug=a;b' "notify-send --app-name=Tuple Tuple can't open that link tuple://join?call_url_slug=a;b"
        opening 'tuple://join?call_url_slug=*' "notify-send --app-name=Tuple Tuple can't open that link tuple://join?call_url_slug=*"
        opening 'tuple://auth?code=abc' "notify-send --app-name=Tuple Tuple can't open that link tuple://auth?code=abc"
        opening "" 'tuple start'
        [ "$failed" = 0 ]
        touch $out
      '';
}
