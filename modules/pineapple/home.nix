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
  home.packages = [ pineapple ];

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
}
