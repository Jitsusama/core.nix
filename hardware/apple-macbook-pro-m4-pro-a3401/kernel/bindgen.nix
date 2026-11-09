# The bindgen Aurora's kernel is built with: upstream's 0.73.2 with the two
# layout repairs its tree carries in tools/aurora-bindgen (`tools` here).
# Stock bindgen gives some of the kernel's C types the wrong alignment in
# Rust, and the kernel's own layout assertions then refuse to compile, so
# every one of Aurora's builds uses this one. The version and the tarball's
# hash are those its sources.json pins. It is nixpkgs' own bindgen package
# otherwise, built from the whole workspace rather than the command's crate,
# since the repairs are to the library beneath it.
{
  lib,
  fetchurl,
  rustPlatform,
  llvmPackages,
  rustc,
  rustfmt,
  python3,
  tools,
}:
let
  # bindgen writes rustfmt's output with nightly features, as nixpkgs says.
  rustfmt-nightly = rustfmt.override { asNightly = true; };
in
rustPlatform.buildRustPackage rec {
  pname = "aurora-bindgen";
  version = "0.73.2";

  src = fetchurl {
    name = "rust-bindgen-${version}.tar.gz";
    url = "https://codeload.github.com/rust-lang/rust-bindgen/tar.gz/refs/tags/v${version}";
    hash = "sha256-X8MnfdM05Lu9e9uMrV1PuHpmey6VorrjUKjVQkwoBik=";
  };

  cargoHash = "sha256-eiIw/FcTb+s1Syf7hLnlLyeU1+TEZa8Mz4c1NC1dEN0=";

  patches = [
    "${tools}/patches/0001-empty-alignment.patch"
    "${tools}/patches/0002-anonymous-tag-members.patch"
  ];

  preConfigure = ''
    export LIBCLANG_PATH="${lib.getLib llvmPackages.libclang}/lib"
  '';

  # The command alone, linked to libclang rather than loading it at run
  # time.
  cargoBuildFlags = [
    "-p"
    "bindgen-cli"
  ];
  buildNoDefaultFeatures = true;
  buildFeatures = [ "logging" ];

  env.RUSTFMT = "${rustfmt-nightly}/bin/rustfmt";

  # The workspace's own tests are bindgen's, which the repairs leave alone.
  # What proves the repairs is the tree's test.py, which compiles C and Rust
  # fixtures of each case it fixes and checks their layouts agree.
  doCheck = false;
  doInstallCheck = true;
  nativeInstallCheckInputs = [
    llvmPackages.clang
    python3
    rustc
  ];
  installCheckPhase = ''
    runHook preInstallCheck
    python3 ${tools}/test.py --bindgen $out/bin/bindgen \
      --rustc ${lib.getExe' rustc "rustc"} --clang ${lib.getExe' llvmPackages.clang "clang"} \
      --output-dir "$TMPDIR/layout-checks"
    runHook postInstallCheck
  '';

  meta = {
    description = "bindgen with the layout repairs Aurora Silicon's kernel needs";
    homepage = "https://github.com/rust-lang/rust-bindgen";
    license = lib.licenses.bsd3;
    mainProgram = "bindgen";
  };
}
