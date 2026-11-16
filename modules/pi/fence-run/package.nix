# pi-fence: one command in pi's own agent fence, for a caller outside pi,
# such as the mycelium node's launcher for agents' runs. It is the
# harness's program (scripts/pi-fence.ts) at a pinned revision, run by the
# store's node, so the launcher a fence declares lives in the store, where
# no agent can change it, and lays the same fence a session does rather
# than a copy of its rules kept here. Moving the pin takes the harness's
# fence as it then stands.
{
  bash,
  fetchFromGitHub,
  lib,
  nodejs,
  writeShellApplication,
}:
let
  harness = fetchFromGitHub {
    owner = "Jitsusama";
    repo = "agentic-harness.pi";
    rev = "e84873b79f3278149daead4f0563587debecfe04";
    hash = "sha256-IWDfMHGNyW832r/fxFwL78uWcK+7PN6U0+KJpGw7g0U=";
  };
in
writeShellApplication {
  name = "pi-fence";
  # The bash it runs the fenced line with, found even on a path as bare as
  # a service's; bubblewrap and landrun on Linux, and sandbox-exec on a
  # Mac, it takes from the system alone.
  runtimeInputs = [ bash ];
  text = ''
    exec ${lib.getExe nodejs} --disable-warning=MODULE_TYPELESS_PACKAGE_JSON \
      ${harness}/scripts/pi-fence.ts "$@"
  '';
  passthru = { inherit harness; };
  meta = {
    description = "Run one command in pi's own agent fence";
    platforms = lib.platforms.linux ++ lib.platforms.darwin;
  };
}
