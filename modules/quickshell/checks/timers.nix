# The checks of the launcher's timers, and the time they keep, which the
# folder's checks.nix gathers with every other place's; its opening comment says
# why each place keeps its own.
{
  account,
  copied,
  graphical,
  noFonts,
  passesClippy,
  pkgs,
  qs,
  shellFolderLint,
  singleton,
  ...
}:
{
  # A timer's row is kept under the node's row: though the daemon keys it as
  # one of the launcher's own kinds, while its acts ask by the record's own
  # key, and what the daemon refuses of a timer, a typed start or the
  # stopwatch is told in the foot by its name and what wasn't done
  # (modules/quickshell/tests/timers.qml), with the timers daemon's held
  # timers stood in, answering each ask as the case says.
  the-launcher-keys-and-tells-the-timers =
    let
      held = singleton "HeldTimers.qml" ''
        property var timers: []
        property var stopwatch: null
        property var asked: []
        property var refusal: null
        property var started: null
        signal heard(var keys)
        function timer(key) { return null; }
        function asking(act, key, then) {
          asked = asked.concat([key === "" ? act : `''${act} ''${key}`]);
          then?.(refusal);
        }
        function start(span, name, then) { then?.(started); }
        function pause(key, then) { asking("pause", key, then); }
        function resume(key, then) { asking("resume", key, then); }
        function again(key, then) { asking("again", key, then); }
        function add(key, span, then) { asking("add", key, then); }
        function end(key, then) { asking("end", key, then); }
        function run(then) { asking("run", "", then); }
        function stop(then) { asking("stop", "", then); }
        function lap(then) { asking("lap", "", then); }
        function reset(then) { asking("reset", "", then); }
      '';
    in
    pkgs.runCommandLocal "the-launcher-keys-and-tells-the-timers" { } ''
      mkdir launcher
      ${copied [
        "Words.qml"
        "launcher/Becoming.js"
        "launcher/Provider.qml"
        "launcher/Timers.qml"
        "launcher/words.json"
      ]}
      cp ${(account graphical).xdg.configFile."quickshell/Theme.qml".source} Theme.qml
      cp ${../tests/timers.qml} shell.qml
      cp ${../../../tests/lib/Expect.js} Expect.js
      cp ${held} HeldTimers.qml
      export HOME=$TMPDIR XDG_RUNTIME_DIR=$TMPDIR/runtime LANG=C.UTF-8 QT_QPA_PLATFORM=offscreen
      mkdir -m 700 $XDG_RUNTIME_DIR
      export FONTCONFIG_FILE=${noFonts}
      ${shellFolderLint}
      timeout 60 ${qs} -p shell.qml
      touch $out
    '';

  # A timer's span and name are read from what's typed after "timer", its time
  # left told as a clock and its span in its largest units, and timers listed
  # rung first, and what Now's Timers row says, as
  # modules/quickshell/tests/timing.qml holds launcher/Timing.js to it in Qt's
  # own engine.
  a-timer-is-read-from-what-is-typed =
    let
      qt = pkgs.qt6.qtdeclarative;
    in
    pkgs.runCommandLocal "a-timer-is-read-from-what-is-typed" { } ''
      cp ${../tests/timing.qml} timing.qml
      cp ${../launcher/Timing.js} Timing.js
      export HOME=$TMPDIR LANG=C.UTF-8 QT_FORCE_STDERR_LOGGING=1 QT_QPA_PLATFORM=offscreen
      export FONTCONFIG_FILE=${noFonts}
      ${shellFolderLint}
      QML_IMPORT_PATH=${qt}/lib/qt-6/qml timeout 60 ${qt}/bin/qml timing.qml
      touch $out
    '';

  # clippy finds nothing in timekeeping, its tests included.
  timekeeping-passes-clippy = passesClippy "timekeeping-passes-clippy" (
    pkgs.callPackage ../timekeeping/package.nix { }
  );

  # The timers daemon's tests run as it builds: a timer started is told to
  # every watch and rings as a critical notice once its end passes, the
  # notice's Again sets it going afresh and putting it away ends it, ending
  # one rung takes its notice back, a ring with no server to take it is
  # said once one comes, what's kept outlasts the daemon, one that ended
  # meanwhile ringing late, and an ask it can't do is refused.
  timers-are-kept-and-rung = pkgs.callPackage ../timers/package.nix { };

  # clippy finds nothing in the timers daemon, its tests included.
  timers-passes-clippy = passesClippy "timers-passes-clippy" (
    pkgs.callPackage ../timers/package.nix { }
  );
}
