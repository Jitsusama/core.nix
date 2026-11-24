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
  fetchurl,
  lib,
  nodejs,
  runCommandLocal,
  writeShellApplication,
}:
let
  harness = fetchFromGitHub {
    owner = "Jitsusama";
    repo = "agentic-harness.pi";
    rev = "209da838798ad54e63ce67d9d5662ce94531b535";
    hash = "sha256-cZ9Rc0MfhigrYEgW0PiqLFaYnW55a97Ru/VaH7lbh2Y=";
  };
  # The census (`pi-fence census`) loads yaml, the one package anything
  # pi-fence runs takes at run time; its other imports are only types. It
  # is the version the harness's pnpm-lock.yaml locks, which the build
  # below holds it to, so moving the pin past a move of yaml's fails
  # until this follows: the same tarball as the lock's sha512, named
  # here by its sha256.
  yamlVersion = "2.9.0";
  yaml = fetchurl {
    url = "https://registry.npmjs.org/yaml/-/yaml-${yamlVersion}.tgz";
    hash = "sha256-AI+iBMsbpwDgJyugRau/Cab/5jRW6BRrqXysbCrR75E=";
  };
  # Node looks for a package in node_modules beside the real path of the
  # file importing it, so the harness is copied in beside it, not linked.
  program = runCommandLocal "pi-fence-harness" { } ''
    ${lib.getExe bash} ${./harness.sh} ${harness} ${yaml} ${yamlVersion} $out
  '';
in
writeShellApplication {
  name = "pi-fence";
  # The bash it runs the fenced line with, found even on a path as bare as
  # a service's; bubblewrap and landrun on Linux, and sandbox-exec on a
  # Mac, it takes from the system alone.
  runtimeInputs = [ bash ];
  text = ''
    exec ${lib.getExe nodejs} --disable-warning=MODULE_TYPELESS_PACKAGE_JSON \
      ${program}/scripts/pi-fence.ts "$@"
  '';
  passthru = { inherit harness; };
  meta = {
    description = "Run one command in pi's own agent fence";
    platforms = lib.platforms.linux ++ lib.platforms.darwin;
  };
}
