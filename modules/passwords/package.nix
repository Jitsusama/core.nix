# The passwords daemon and command, built from this directory. Its tests
# run as it builds once it's handed the launcher's vocabulary, `words`,
# which its rows are said in and its tests read whole rather than keep a
# copy of; a machine's own build, handed none, runs none, as
# passwords-holds-no-secret runs them.
{
  lib,
  openssh,
  rustPlatform,
  words ? null,
}:
rustPlatform.buildRustPackage {
  pname = "passwords";
  version = "0.1.0";
  # The ready-rows and terminal-kit crates it names by path come beside it,
  # and it builds from its own folder within.
  src = lib.fileset.toSource {
    root = ../.;
    fileset = lib.fileset.unions [
      ./Cargo.toml
      ./Cargo.lock
      ./src
      ./tests
      ../ready-rows/Cargo.toml
      ../ready-rows/src
      ../terminal-kit/Cargo.toml
      ../terminal-kit/src
    ];
  };
  sourceRoot = "source/passwords";
  cargoLock.lockFile = ./Cargo.lock;
  doCheck = words != null;
  # `passwords keys` lists 1Password's SSH agent's keys with these, named
  # as it's built so it never runs whichever an account's PATH finds first.
  env = {
    PASSWORDS_SSH_ADD = lib.getExe' openssh "ssh-add";
    PASSWORDS_SSH_KEYGEN = lib.getExe' openssh "ssh-keygen";
  }
  // lib.optionalAttrs (words != null) { LAUNCHER_WORDS = "${words}"; };
  meta = {
    description = "Holds Joel's passwords ready for the shell and the command line, from each password manager";
    license = lib.licenses.mit;
    mainProgram = "passwords";
    platforms = lib.platforms.linux;
  };
}
