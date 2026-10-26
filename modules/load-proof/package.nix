# load-proof, built from this directory, its tests run as it builds.
{
  cargo,
  curl,
  lib,
  pipewire,
  rustc,
  rustPlatform,
  stdenv,
  symlinkJoin,
}:
let
  src = lib.fileset.toSource {
    root = ./.;
    fileset = lib.fileset.unions [
      ./Cargo.toml
      ./Cargo.lock
      ./src
    ];
  };
  cargoDeps = rustPlatform.importCargoLock { lockFile = ./Cargo.lock; };
  # A run's builds compile load-proof's own source with these, and fill
  # the line with curl, all found in one folder.
  tools = symlinkJoin {
    name = "load-proof-tools";
    paths = [
      cargo
      curl
      rustc
      stdenv.cc
    ];
  };
in
rustPlatform.buildRustPackage {
  pname = "load-proof";
  version = "0.1.0";
  inherit src cargoDeps;
  env = {
    # The sound signal plays silence and reads the graph's errors with
    # PipeWire's own tools, found here rather than on the path.
    PIPEWIRE_BIN = "${lib.getBin pipewire}/bin";
    # A run's load builds this source, kept beside the program, offline
    # from the crates it was built with.
    LOAD_PROOF_SOURCE = "${placeholder "out"}/share/load-proof/source";
    LOAD_PROOF_VENDOR = "${cargoDeps}";
    LOAD_PROOF_TOOLS = "${tools}/bin";
  };
  postInstall = ''
    mkdir -p $out/share/load-proof
    cp -r ${src} $out/share/load-proof/source
  '';
  meta = {
    description = "Says whether what Joel uses still feels idle under full load";
    license = lib.licenses.mit;
    mainProgram = "load-proof";
    platforms = lib.platforms.linux;
  };
}
