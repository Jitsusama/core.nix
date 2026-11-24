# code-health, built from this directory, its tests run as it builds, with
# git for the one that times a repository's merges, and tofu, tflint (with
# Google's rules beside its own) and trivy, which it runs over a
# repository's Terraform, found on its own PATH wherever it runs
# (src/terraform.rs), and the providers tofu
# validate readies a workspace with (providers.nix), on the systems it has
# them for. It takes its shape
# from the terminal kit (../terminal-kit), so the source is cut from the
# folder holding both.
{
  callPackage,
  git,
  lib,
  makeWrapper,
  opentofu,
  rustPlatform,
  tflint,
  trivy,
}:
let
  providers = callPackage ./providers.nix { };
  linter = tflint.withPlugins (plugins: [ plugins.tflint-ruleset-google ]);
in
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
    linter
    trivy
  ];
  postInstall = ''
    wrapProgram $out/bin/code-health --suffix PATH : ${
      lib.makeBinPath [
        opentofu
        linter
        trivy
      ]
    } ${
      lib.optionalString (providers != null) "--set-default CODE_HEALTH_TERRAFORM_PROVIDERS ${providers}"
    }
  '';
  meta = {
    description = "Writes the day's figures of how messy each repository it's given is";
    license = lib.licenses.mit;
    mainProgram = "code-health";
  };
}
