# Brings a Mac's pi sessions back after it restarts, so nobody types
# `pi --session` for each (src/main.rs says how). An agent in the login runs
# each minute and as the login starts: it keeps the pi sessions each realm's
# node holds on this machine, and after a newer boot starts each again in
# its own folder, through the node, which starts it through the account's
# login shell. It's a Mac's alone, where restarts ended sessions someone
# then typed back, and an account in no realm has none to keep.
{ mycelium }:
{
  config,
  lib,
  pkgs,
  ...
}:
let
  package = pkgs.callPackage ./package.nix { };
  myceliumProgram = lib.getExe mycelium.packages.${pkgs.stdenv.hostPlatform.system}.default;
  realm = config.jitsusama.mycelium.realm or null;
  realms =
    lib.optional (realm != null) realm ++ lib.attrNames (config.jitsusama.mycelium.alsoJoins or { });
in
lib.mkIf (pkgs.stdenv.hostPlatform.isDarwin && realms != [ ]) {
  launchd.agents.sessions-come-back = {
    enable = true;
    config = {
      ProgramArguments = [
        (lib.getExe package)
        myceliumProgram
      ]
      ++ realms;
      # As the login starts, which is the first chance after a restart, and
      # each minute after, to keep what's held and to try again what the
      # node wasn't ready to start.
      RunAtLoad = true;
      StartInterval = 60;
      # Whatever the account runs comes first.
      ProcessType = "Background";
      StandardErrorPath = "${config.home.homeDirectory}/Library/Logs/sessions-come-back.log";
    };
  };
}
