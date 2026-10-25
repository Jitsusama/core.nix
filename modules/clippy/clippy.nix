# How core holds Rust to clippy, for its own checks and, through the flake's
# `lib.clippy`, for a repository built on it, so both are held to the same
# lints from one place.
{ lib, clippy }:
rec {
  # clippy's own lints and its pedantic group, less three about a published
  # library's API, which nothing here is: the results a caller must use, and
  # the errors and panics its docs must name. A warning fails. A lint is
  # silenced only with `#[expect]` and its reason, never `#[allow]`, so each
  # says why, and one whose lint no longer fires is caught and taken out.
  lints = [
    "-D"
    "warnings"
    "-W"
    "clippy::pedantic"
    "-W"
    "clippy::allow_attributes"
    "-W"
    "clippy::allow_attributes_without_reason"
    "-A"
    "clippy::must_use_candidate"
    "-A"
    "clippy::missing_errors_doc"
    "-A"
    "clippy::missing_panics_doc"
  ];

  # A folder holding core's clippy.toml, its settings apart from the rest of
  # the tree, so a change elsewhere doesn't rebuild every check that reads it.
  config = lib.fileset.toSource {
    root = ../..;
    fileset = ../../clippy.toml;
  };

  # A check that `package`, built with rustPlatform, passes clippy: its build
  # replaced by clippy, its tests and examples read too, and nothing
  # installed. `settings` is the folder its clippy.toml is in, core's unless
  # the repository has names of its own to teach it.
  passes =
    {
      name,
      package,
      settings ? config,
    }:
    package.overrideAttrs (old: {
      pname = name;
      nativeBuildInputs = old.nativeBuildInputs ++ [ clippy ];
      CLIPPY_CONF_DIR = settings;
      buildPhase = ''
        cargo clippy --all-targets --offline -- ${lib.escapeShellArgs lints}
      '';
      doCheck = false;
      installPhase = ''
        touch $out
      '';
    });
}
