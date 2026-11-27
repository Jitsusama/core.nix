# The checks of tuple, which checks.nix gathers with every other folder's;
# its opening comment says what each kind of check proves.
{
  account,
  graphical,
  holds,
  lib,
  onLinux,
  passesClippy,
  pkgs,
  ...
}:
onLinux {
  # tuple-call's tests run as it builds: against a stand-in tuple and log,
  # its stream followed only while Tuple answers, a call, a ring and a
  # missed call told to a watch, a call that ended kept in Recent and read
  # back after a restart, each act run as Tuple's command, and the socket
  # systemd hands it answered on (call/tests/daemon.rs).
  tuple-call-holds-the-call = pkgs.callPackage ./call/package.nix { };
  # clippy finds nothing in tuple-call, its tests included.
  tuple-call-passes-clippy = passesClippy "tuple-call-passes-clippy" (
    pkgs.callPackage ./call/package.nix { }
  );
  # The daemon runs on the socket the shell's daemons share, systemd's,
  # with the tuple command the module installs.
  tuple-call-runs-on-the-daemons-socket =
    let
      home = account graphical;
      service = home.systemd.user.services.tuple-call;
      # Compared as plain text: a pattern may name no store path.
      plain = builtins.unsafeDiscardStringContext;
      started = plain (lib.concatStringsSep " " (lib.toList (service.Service.ExecStart or "")));
      tuple = lib.findFirst (package: lib.getName package == "tuple") null home.home.packages;
    in
    holds "tuple-call-runs-on-the-daemons-socket"
      "tuple-call doesn't run on systemd's socket at %t/daemons/tuple-call.sock with the installed tuple."
      (
        lib.hasInfix (plain "%t/daemons/tuple-call.sock ${lib.getExe tuple}") started
        && service.Unit.X-SwitchMethod == "restart"
        && lib.elem "tuple-call.socket" service.Unit.Requires
        && home.systemd.user.sockets.tuple-call.Socket.ListenStream == "%t/daemons/tuple-call.sock"
      );

  # The people daemon's Tuple source reads Tuple's contacts, says only who
  # changed, and waits longer each time Tuple's follow ends at once, as it
  # does while nobody is signed in, up to a minute; what Tuple says as it
  # ends goes to the journal once while it says the same.
  tuple-people-follows-the-contacts =
    pkgs.runCommandCC "tuple-people-follows-the-contacts" { nativeBuildInputs = [ pkgs.rustc ]; }
      ''
        rustc --edition 2024 --test -D warnings -o tests ${./people.rs}
        ./tests
        touch $out
      '';

  # Tuple shares a screen through the portal: the switch writes the setting
  # where Tuple keeps it, and Tuple, which ignores a folder it doesn't know
  # and a value not written as it writes one, has to read it back. So the
  # check runs the switch's own steps, with the data folder moved into a
  # folder of its own (tests/tuple-shares-through-the-portal.sh).
  tuple-shares-through-the-portal =
    let
      home = account graphical;
      tuple = lib.findFirst (package: lib.getName package == "tuple") null home.home.packages;
      data = home.xdg.dataHome;
    in
    pkgs.runCommandLocal "tuple-shares-through-the-portal" { nativeBuildInputs = [ tuple ]; } ''
      ${lib.getExe pkgs.bash} ${./tests/tuple-shares-through-the-portal.sh} ${
        lib.escapeShellArgs [
          "${home.home.activationPackage}/activate"
          data
        ]
      }
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

  # Tuple says only that it started, so the service says when it can take
  # a call, after systemd's own line, where health takes it: its own
  # ExecStartPost, run against a stand-in systemctl and a log of its own
  # (tests/tuple-says-when-ready.sh), given too the name, level, filters and
  # output the unit's lines go out with, and the ceiling systemd's own lines
  # about it are kept under, which health reads by. The pattern
  # journalctl picks health's ready lines with is read from health's own
  # source, so the two can't drift apart; the words health's parser times
  # after it, `X: ready in N ms`, are pinned by health's own tests, as the
  # test pins Tuple's.
  tuple-says-when-ready =
    let
      home = account graphical;
      service = home.systemd.user.services.tuple.Service;
      post = service.ExecStartPost;
      setting = name: "${name}=${toString (service.${name} or "")}";
      declared = ''^pub const READY_FOLLOWED: &str = r"(.*)";$'';
      lines = lib.splitString "\n" (builtins.readFile ../health/src/launches.rs);
      lost = "tuple-says-when-ready reads READY_FOLLOWED off one line of launches.rs, a raw string";
      found = lib.findFirst (line: builtins.match declared line != null) (throw lost) lines;
      ready = lib.head (builtins.match declared found);
    in
    pkgs.runCommandLocal "tuple-says-when-ready" { } ''
      ${lib.getExe pkgs.bash} ${./tests/tuple-says-when-ready.sh} ${
        lib.escapeShellArgs [
          (lib.removeSuffix " %n" (lib.removePrefix "-" post))
          (lib.getExe' pkgs.systemd "systemctl")
          "${home.xdg.dataHome}/tuple/0/log.txt"
          ready
          (setting "SyslogIdentifier")
          (setting "SyslogLevel")
          (setting "LogLevelMax")
          (setting "LogFilterPatterns")
          (setting "StandardOutput")
        ]
      }
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
  # in a real line editor with the real tuple (tests/tuple-completion.zsh),
  # held to a time, in a folder of its own whose daemon alone it stops
  # (tests/tuple-completes-in-zsh.sh).
  tuple-completes-in-zsh =
    let
      home = account graphical;
      tuple = lib.findFirst (package: lib.getName package == "tuple") null home.home.packages;
    in
    pkgs.runCommandLocal "tuple-completes-in-zsh"
      {
        nativeBuildInputs = [
          tuple
          pkgs.zsh
        ];
      }
      ''
        ${lib.getExe pkgs.bash} ${./tests/tuple-completes-in-zsh.sh} \
          ${./tests/tuple-completion.zsh} ${tuple}/share/zsh/site-functions/_tuple
        touch $out
      '';

  # The launcher calls, joins and stars a contact after "--", so a handle
  # that starts with a dash is a name, and a Tuple update that stopped
  # ending its flags there would leave every such row doing nothing. Each is
  # asked with one name too many, which the real tuple refuses by counting,
  # in a sandbox with no network or account, so it reaches nobody
  # (tests/tuple-ends-its-flags.sh). It stops Tuple's daemon, so it's run
  # first from the shells a person would run it from, and as the build with
  # each of its signs moved, each of which it must refuse, and as the build
  # is, which it must let through (tests/tuple-ends-its-flags-outside.sh). To see what a
  # command takes by hand, `tuple help <cmd>` prints it without running it,
  # as package.nix says.
  tuple-ends-its-flags =
    let
      home = account graphical;
      tuple = lib.findFirst (package: lib.getName package == "tuple") null home.home.packages;
    in
    pkgs.runCommandLocal "tuple-ends-its-flags" { nativeBuildInputs = [ tuple ]; } ''
      ${lib.getExe pkgs.bash} ${./tests/tuple-ends-its-flags-outside.sh} \
        ${./tests/tuple-ends-its-flags.sh}
      export HOME=$PWD XDG_DATA_HOME=$PWD/data XDG_RUNTIME_DIR=$PWD/run
      mkdir -m 700 run
      ${lib.getExe pkgs.bash} ${./tests/tuple-ends-its-flags.sh}
      touch $out
    '';

  # A call link opened in a browser comes back as tuple://join, which only
  # the desktop entry's command takes, so the check runs it as the entry
  # does, with stand-ins for tuple and notify-send, over links like the
  # page's and ones it mustn't run. xdg-open finds the entry by what it
  # says it opens, beside a list of defaults a browser wrote, which names
  # none for Tuple's links; gio, and so the portal, are asked too, and
  # find it only through niri's own list (tests/tuple-opens-its-links.sh).
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
        ${lib.getExe pkgs.bash} ${./tests/tuple-opens-its-links.sh} ${
          lib.escapeShellArgs [
            (pkgs.writeText "tuple.desktop" entry)
            (pkgs.writeText "niri-mimeapps.list" defaults)
            (lib.getExe tuple)
          ]
        }
        touch $out
      '';
}
