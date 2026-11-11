# The SSH agent a fenced pi command reaches, built from this folder, its
# tests run as it builds (tests/agent_proxy.rs). It signs only logins to
# the hosts it's told of, each proved by the session's own binding.
{ lib, rustPlatform }:
rustPlatform.buildRustPackage {
  pname = "pi-agent-proxy";
  version = "0.1.0";
  src = lib.fileset.toSource {
    root = ./.;
    fileset = lib.fileset.unions [
      ./Cargo.toml
      ./Cargo.lock
      ./src
      ./tests
    ];
  };
  cargoLock.lockFile = ./Cargo.lock;
  meta = {
    description = "An SSH agent for pi's fenced commands that signs only logins to the hosts it's told of";
    license = lib.licenses.mit;
    mainProgram = "pi-agent-proxy";
    platforms = lib.platforms.unix;
  };
}
