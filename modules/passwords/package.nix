# The passwords daemon and command, built from this directory, its tests
# run as it builds.
{
  lib,
  openssh,
  rustPlatform,
}:
rustPlatform.buildRustPackage {
  pname = "passwords";
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
  # `passwords keys` lists 1Password's SSH agent's keys with these, named
  # as it's built so it never runs whichever an account's PATH finds first.
  env = {
    PASSWORDS_SSH_ADD = lib.getExe' openssh "ssh-add";
    PASSWORDS_SSH_KEYGEN = lib.getExe' openssh "ssh-keygen";
  };
  meta = {
    description = "Holds Joel's passwords ready for the shell and the command line, from each password manager";
    license = lib.licenses.mit;
    mainProgram = "passwords";
    platforms = lib.platforms.linux;
  };
}
