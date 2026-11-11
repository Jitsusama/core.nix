# slack-people and slack-unreads, built from this directory, their tests run
# as they build.
{
  lib,
  rustPlatform,
  systemd,
}:
rustPlatform.buildRustPackage {
  pname = "slack-people";
  version = "0.1.0";
  src = lib.fileset.toSource {
    root = ./.;
    fileset = lib.fileset.unions [
      ./Cargo.toml
      ./Cargo.lock
      ./src
      # The stand-in alone: the QML cases beside it aren't the crate's.
      ./tests/stand_in.rs
    ];
  };
  cargoLock.lockFile = ./Cargo.lock;
  # A test starts slack-unreads as systemd does for its socket, through
  # systemd-socket-activate, handing it the socket it answers on.
  nativeCheckInputs = [ systemd ];
  # The launcher's own vocabulary, which a test holds the rows' words to
  # (src/rows.rs), as the unit hands it to slack-unreads.
  env.LAUNCHER_WORDS = "${../quickshell/launcher/words.json}";
  meta = {
    description = "Says who Slack knows by an address, and what in Slack wants Joel, only reading";
    license = lib.licenses.mit;
    mainProgram = "slack-people";
    platforms = lib.platforms.linux;
  };
}
