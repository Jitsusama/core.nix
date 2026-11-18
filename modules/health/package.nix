# health, built from this directory, its tests run as it builds.
{
  lib,
  runtimeShell,
  rustPlatform,
  stdenv,
  systemd,
}:
rustPlatform.buildRustPackage rec {
  pname = "health";
  version = "0.1.0";
  src = lib.fileset.toSource {
    root = ../.;
    fileset = lib.fileset.unions [
      ./Cargo.toml
      ./Cargo.lock
      ./src
      ../terminal-kit/Cargo.toml
      ../terminal-kit/src
    ];
  };
  sourceRoot = "${src.name}/health";
  cargoLock.lockFile = ./Cargo.lock;
  # The shell the replay's tests start their stand-in journalctls on, which
  # a build's sandbox has where it has no /bin/sh.
  env = {
    STAND_IN_SHELL = runtimeShell;
  }
  # systemd's own journal-remote, which writes the day the tests read with
  # the machine's journalctl, so the histories and followers are proven
  # on systemd itself rather than on stand-ins. A Mac has no journal.
  // lib.optionalAttrs stdenv.hostPlatform.isLinux {
    STAND_IN_JOURNAL_REMOTE = "${systemd}/lib/systemd/systemd-journal-remote";
  };
  nativeCheckInputs = lib.optionals stdenv.hostPlatform.isLinux [ systemd ];
  meta = {
    description = "Keeps the day's tally of what the machine's journals said";
    license = lib.licenses.mit;
    mainProgram = "health";
    # It reads systemd's journals, or a Mac's unified log.
    platforms = lib.platforms.linux ++ lib.platforms.darwin;
  };
}
