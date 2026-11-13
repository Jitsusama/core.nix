# github-broker, built from this directory, its tests run as it builds. It
# takes its shape from the terminal kit (../terminal-kit), so the source is
# cut from the folder holding both.
{
  lib,
  rustPlatform,
  makeWrapper,
  age,
  age-plugin-tpm,
}:
rustPlatform.buildRustPackage rec {
  pname = "github-broker";
  version = "0.1.0";
  src = lib.fileset.toSource {
    root = ../.;
    fileset = lib.fileset.unions [
      ./Cargo.toml
      ./Cargo.lock
      ./src
      ./tests
      ../terminal-kit/Cargo.toml
      ../terminal-kit/src
    ];
  };
  sourceRoot = "${src.name}/github-broker";
  cargoLock.lockFile = ./Cargo.lock;
  nativeBuildInputs = [ makeWrapper ];
  # The App is opened with age and the machine's own TPM identity, through
  # age-plugin-tpm, which asks for no PIN, since nobody is there to type
  # one when a sandbox asks.
  postInstall = ''
    wrapProgram $out/bin/github-broker \
      --prefix PATH : ${
        lib.makeBinPath [
          age
          age-plugin-tpm
        ]
      }
  '';
  meta = {
    description = "Hands a GitHub App's hour-long installation tokens to its socket's callers";
    license = lib.licenses.mit;
    mainProgram = "github-broker";
    platforms = lib.platforms.linux;
  };
}
