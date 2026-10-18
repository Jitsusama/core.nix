# linux-id, which isn't in nixpkgs: a FIDO security key emulated over uhid,
# its keys made in the TPM, so Chrome sees a passkey that can't leave the
# laptop. It asks whether Joel is there through pinentry, which
# pinentry-askpass answers by asking the shell.
{
  lib,
  buildGoModule,
  fetchFromGitHub,
}:
buildGoModule (finalAttrs: {
  pname = "linux-id";
  version = "0.2.4";

  src = fetchFromGitHub {
    owner = "matejsmycka";
    repo = "linux-id";
    tag = "v${finalAttrs.version}";
    hash = "sha256-zDlZrQLPi3Y/ntoAGCTyMUJ2j/Zzm0gax8y2MjyIGSk=";
  };

  vendorHash = "sha256-G+MNVGrTAiWhJ4XQa5dQ0ChhHTSvO5VMQ/Cy5CWP7tk=";

  ldflags = [
    "-s"
    "-w"
  ];

  meta = {
    description = "A FIDO security key whose keys the TPM holds";
    homepage = "https://github.com/matejsmycka/linux-id";
    license = lib.licenses.mit;
    mainProgram = "linux-id";
    platforms = lib.platforms.linux;
  };
})
