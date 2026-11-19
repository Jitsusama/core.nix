# Where every pi keeps node's compiled-code cache (NODE_COMPILE_CACHE, not
# the glossary's compile cache, which is sccache's). node reads it back as
# compiled code when pi next starts, outside the agents' fence. Left to
# itself it keeps the cache in the temporary folder, which a fenced command
# can write, so every pi compiles into pi's state, which the fence holds
# read-only. The harness names the same folder for every pi it starts
# itself; this covers the first, started from a shell or the launcher.
{
  config,
  lib,
  pkgs,
  ...
}:
let
  compileCache = "${config.xdg.stateHome}/pi/agentic-harness.pi/node-compile-cache";
in
{
  home.sessionVariables.NODE_COMPILE_CACHE = compileCache;

  # What the launcher and the user manager start reads this one. A session
  # already running keeps the variable it started with until the next login.
  # A Mac has no such place: what launchd starts there gets neither, and
  # relies on the harness naming the folder for every pi it starts.
  systemd.user.sessionVariables = lib.mkIf pkgs.stdenv.hostPlatform.isLinux {
    NODE_COMPILE_CACHE = compileCache;
  };
}
