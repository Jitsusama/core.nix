# The safe word that stops Joel's agents, read where it's said: Slack for
# now, through the harness's Slack sign-in, only read, and in shadow, so
# its stops are written down and nothing acts on them yet (src/lib.rs says
# how the pieces fit). A machine signed in to Slack as Joel imports it.
{
  lib,
  pkgs,
  ...
}:
let
  pineapple = pkgs.callPackage ./package.nix { };
  # Prints pi's Slack sign-in from the desktop's keyring, where pi has kept
  # it since moving sign-ins there, or fails while it holds none, so the
  # old file is read instead (../slack's `Account`), as slack-unreads
  # reads it. secret-tool's output goes to the program alone.
  keyring = pkgs.writeShellScript "pineapple-sign-in" ''
    exec ${lib.getExe' pkgs.libsecret "secret-tool"} lookup \
      service agentic-harness username slack-credentials
  '';
in
{
  imports = [ ../quickshell/providers.nix ];

  home.packages = [ pineapple ];

  # Its rows in For You (SafeWord.qml), read from `pineapple serve` below;
  # on a machine without the shell nothing reads it. Beside Sound's, since
  # both only give For You what to do.
  jitsusama.quickshell.launcher.providers.pineapple = {
    order = 46;
    source = ./SafeWord.qml;
  };

  systemd.user.services.pineapple-slack = {
    Unit = {
      Description = "Reads Slack for the safe word that stops Joel's agents, in shadow";
      # Restarted after a switch reloads the units, it comes back as the
      # new one.
      X-SwitchMethod = "restart";
    };
    Service = {
      Slice = "background.slice";
      # Shown in ps by its name, not its store path (docs/daemons.md).
      ExecStart = "@${lib.getExe pineapple} pineapple-slack run";
      # The keyring's reader, run each round, so a sign-in made or renewed
      # while it runs is read without a restart. With none, it says so in
      # the journal and reads nothing until there is one.
      Environment = [ "SIGN_IN_KEYRING=${keyring}" ];
      Restart = "on-failure";
      RestartSec = 30;
    };
    # Started at login rather than once a sign-in file appears, since pi
    # keeps its sign-in in the keyring now and the file may never come.
    Install.WantedBy = [ "default.target" ];
  };

  # The safe word's daemon for the launcher (src/serve.rs): it serves the
  # stops, alerts and resume requests as Needs You's rows over
  # %t/daemons/pineapple.sock, takes Joel's presses only from his shell,
  # and knocks as each first appears. In shadow it serves nothing and only
  # writes down what it would have told. It only reads what pineapple-slack
  # writes, and writes only under ~/.local/state/pineapple.
  systemd.user.services.pineapple = {
    Unit = {
      Description = "Shows Joel the safe word's stops in the launcher";
      X-SwitchMethod = "restart";
      # Its socket is systemd's, handed to it as it starts.
      Requires = [ "pineapple.socket" ];
      After = [ "pineapple.socket" ];
    };
    Service = {
      Slice = "background.slice";
      # Started as @<program> pineapple, so ps reads its name rather than a
      # store path (docs/daemons.md).
      ExecStart = lib.escapeShellArgs [
        "@${pineapple}/bin/pineapple"
        "pineapple"
        "serve"
        "--socket"
        "%t/daemons/pineapple.sock"
        "--vocabulary"
        "${(import ../quickshell/launcher/words.nix).file}"
        "--notify-send"
        (lib.getExe' pkgs.libnotify "notify-send")
      ];
      Restart = "on-failure";
      RestartSec = 5;
    };
    # Started at login rather than when the launcher first asks, so a stop
    # knocks even before Joel opens it.
    Install.WantedBy = [ "default.target" ];
  };

  systemd.user.sockets.pineapple = {
    Unit.Description = "Where the launcher asks for the safe word's rows";
    Socket = {
      ListenStream = "%t/daemons/pineapple.sock";
      SocketMode = "0600";
      DirectoryMode = "0700";
    };
    Install.WantedBy = [ "sockets.target" ];
  };
}
