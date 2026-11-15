# code-health, built from this directory, its tests run as it builds, with
# git for the one that times a repository's merges, and tofu, tflint and
# trivy, which it runs over a repository's Terraform, found on its own
# PATH wherever it runs (src/terraform.rs). It takes its shape
# from the terminal kit (../terminal-kit), so the source is cut from the
# folder holding both.
{
  git,
  lib,
  makeWrapper,
  opentofu,
  rustPlatform,
  tflint,
  trivy,
}:
rustPlatform.buildRustPackage rec {
  pname = "code-health";
  version = "0.1.0";
  src = lib.fileset.toSource {
    root = ./..;
    fileset = lib.fileset.unions [
      ./Cargo.toml
      ./Cargo.lock
      ./src
      ../terminal-kit/Cargo.toml
      ../terminal-kit/src
    ];
  };
  sourceRoot = "${src.name}/code-health";
  cargoLock.lockFile = ./Cargo.lock;
  nativeBuildInputs = [ makeWrapper ];
  nativeCheckInputs = [
    git
    opentofu
    tflint
    trivy
  ];
  postInstall = ''
    wrapProgram $out/bin/code-health --suffix PATH : ${
      lib.makeBinPath [
        opentofu
        tflint
        trivy
      ]
    }
  '';
  meta = {
    description = "Writes the day's figures of how messy each repository it's given is";
    license = lib.licenses.mit;
    mainProgram = "code-health";
  };
}
