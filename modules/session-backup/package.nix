# session-backup, built from this directory, its tests run as it builds. It
# takes its shape from the terminal kit (../terminal-kit), so the source is
# cut from the folder holding both.
{
  lib,
  rustPlatform,
  makeWrapper,
  age,
  age-plugin-fido2-hmac,
  age-plugin-tpm,
}:
rustPlatform.buildRustPackage rec {
  pname = "session-backup";
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
  sourceRoot = "${src.name}/session-backup";
  cargoLock.lockFile = ./Cargo.lock;
  nativeBuildInputs = [ makeWrapper ];
  # The plugins age seals through: age-plugin-tag, which comes with age, for
  # a TPM's age1tag1 recipient, and age-plugin-fido2-hmac for a YubiKey's.
  # Sealing touches neither key. A fetch opens a copy with the machine's
  # own TPM identity through age-plugin-tpm.
  postInstall = ''
    wrapProgram $out/bin/session-backup \
      --prefix PATH : ${
        lib.makeBinPath [
          age
          age-plugin-fido2-hmac
          age-plugin-tpm
        ]
      }
  '';
  meta = {
    description = "Copies each settled pi session log, sealed, into a Google Drive folder";
    license = lib.licenses.mit;
    mainProgram = "session-backup";
    platforms = lib.platforms.linux;
  };
}
