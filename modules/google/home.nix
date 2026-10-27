# The Google account, for the desktop: google asks Google's own API for the
# meetings on Joel's calendar and the mail that wants him, and keeps them
# for the shell on a socket, $XDG_RUNTIME_DIR/google.sock, and in a file
# beside it, google.json, which the shell watches (src/main.rs says how).
#
# It asks as the account the harness signed in, from the harness's own
# file, which pi writes when Joel signs in with /google-auth and rewrites
# each time it refreshes a token. google only reads it. Until the file
# exists nothing runs, so a machine nobody signed in to Google on asks
# Google nothing; once it appears, google starts.
#
# It's background work, a small request a minute over the line, so it runs
# in background.slice, behind everything Joel uses.
{
  config,
  lib,
  pkgs,
  ...
}:
let
  google = pkgs.callPackage ./package.nix { };
  credentials = "${config.xdg.configHome}/agentic-harness/google/credentials.json";
in
{
  systemd.user.paths.google = {
    Unit.Description = "Starts the Google account's watch once pi has signed in";
    Path.PathExists = credentials;
    Install.WantedBy = [ "default.target" ];
  };

  systemd.user.services.google = {
    Unit.Description = "Keeps the Google account's meetings and mail for the shell";
    Service = {
      Slice = "background.slice";
      ExecStart = "${lib.getExe google} %t/google.sock ${lib.escapeShellArg credentials}";
      Restart = "on-failure";
      RestartSec = 10;
      # What it held goes with it, so the shell shows no account rather than
      # what was true when it stopped.
      ExecStopPost = "${pkgs.coreutils}/bin/rm -f %t/google.json %t/google.json.new";
    };
  };
}
