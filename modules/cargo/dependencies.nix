# A Rust program's dependencies built once, apart from the program, so a
# change to its own source compiles only its own crate. rustPlatform builds
# every crate a lock names in the same derivation as the program, so the
# reader's 330 compiled from scratch on every change to it or to the key
# language it reads: on 6 November #2052 waited 15.5 minutes on that alone
# before it reached the machine.
#
# `build` takes what rustPlatform.buildRustPackage would, and `manifests`,
# the program's Cargo.toml and Cargo.lock laid out as its `src` lays them
# out, and gives the same program. Its dependencies are built first by a
# derivation of their own from the same arguments, with a stub for the
# program's source, so it builds again only when those two files change:
# for the program and its tests, and for clippy (../clippy/clippy.nix),
# whose dependencies cargo builds as a check. The program's build starts in
# the target folder that left, with the stub's own crate taken out of it.
# cargo reads a vendored crate as unchanging, by its version, so nothing it
# holds is built again.
{ zstd }:
{
  build =
    rustPlatform: manifests: args:
    let
      dependencies = rustPlatform.buildRustPackage (
        args
        // {
          pname = "${args.pname}-dependencies";
          src = manifests;
          nativeBuildInputs = (args.nativeBuildInputs or [ ]) ++ [ zstd ];
          postPatch = ''
            mkdir -p src
            echo 'fn main() {}' > src/main.rs
          '';
          # Built as the program builds and tests itself, and as clippy
          # checks it, each with the stub, which runs nothing.
          cargoTestFlags = [ "--no-run" ];
          preCheck = "";
          postCheck = ''
            cargo check --all-targets --offline
          '';
          installPhase = ''
            mkdir -p $out
            tar --zstd -cf $out/target.tar.zst target
          '';
          desktopItems = [ ];
          postInstall = "";
          dontFixup = true;
        }
      );
    in
    rustPlatform.buildRustPackage (
      args
      // {
        nativeBuildInputs = (args.nativeBuildInputs or [ ]) ++ [ zstd ];
        # Before configuring, which clippy's check keeps though it replaces
        # the build. Without its fingerprint, the stub's crate is built again
        # from the program's source. cargo builds a crate again when a file
        # its build script watches, or a crate under it, is newer than what
        # it left, and the setup copied the vendored crates stamped with the
        # time it ran: they are dated back to the store's own time, and the
        # target folder keeps the times it was built at.
        preConfigure = ''
          find "$NIX_BUILD_TOP/$(stripHash "$cargoDeps")" -exec touch -h -d @1 {} +
          tar --zstd -xf ${dependencies}/target.tar.zst
          find target -path '*/.fingerprint/${args.pname}-*' -prune -exec rm -rf {} +
        ''
        + (args.preConfigure or "");
        passthru = (args.passthru or { }) // {
          inherit dependencies;
        };
      }
    );
}
