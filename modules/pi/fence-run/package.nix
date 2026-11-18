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
    rev = "53426e4b17edcd72ddf0edebca374ae4a7b06229";
    hash = "sha256-NXaM4jqo+YEoTCgXGbyXEUilsmmXL5M5el/hE+ymUis=";
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
    sed -n '/^importers:/,/^packages:/p' ${harness}/pnpm-lock.yaml \
      | grep -A2 -x '      yaml:' | grep -qx '        version: ${yamlVersion}' || {
      echo "the harness's lock no longer locks yaml ${yamlVersion}" >&2
      exit 1
    }
    cp -r ${harness} $out
    chmod -R u+w $out
    mkdir -p $out/node_modules/yaml
    tar -xzf ${yaml} -C $out/node_modules/yaml --strip-components=1
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
