# The program that tells the shaped machine on this line what this one
# sends up, built from this directory, its tests run as it builds. It takes
# its shape from the terminal kit (../terminal-kit), so the source is cut
# from the folder holding both.
{
  lib,
  rustPlatform,
  stdenv,
}:
let
  # The tests a Mac's Nix sandbox denies, which open a socket to send by
  # the loopback. Measured on 23 November.
  deniedInMacSandbox = [
    "route::tests::this_machine_sends_to_itself_by_its_loopback"
    "sent::tests::the_loopbacks_count_grows_by_what_it_sends"
  ];
in
rustPlatform.buildRustPackage rec {
  pname = "house-report";
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
  sourceRoot = "${src.name}/house-report";
  cargoLock.lockFile = ./Cargo.lock;
  meta = {
    description = "Tells the shaped machine on this line what this one sends up";
    license = lib.licenses.mit;
    mainProgram = "house-report";
    platforms = lib.platforms.linux ++ lib.platforms.darwin;
  };
}
# A sandboxed Mac build skips what its sandbox denies, so a Mac's system
# build doesn't fail on them. It knows the sandbox by /Users, which only a
# sandboxed build can't list, so a Mac building outside it runs every test,
# as Linux does. `--exact` skips those whole names alone, so a test added
# later still runs.
// lib.optionalAttrs stdenv.hostPlatform.isDarwin {
  preCheck = ''
    if ! ls /Users > /dev/null 2>&1; then
      echo "In a Mac's Nix sandbox: skipping the tests that send by the loopback"
      checkFlags="''${checkFlags-} --exact ${
        lib.concatMapStringsSep " " (test: "--skip=${test}") deniedInMacSandbox
      }"
    fi
  '';
}
