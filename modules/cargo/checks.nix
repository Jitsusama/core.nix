# The checks of cargo, which checks.nix gathers with every other folder's;
# its opening comment says what each kind of check proves.
{
  account,
  holds,
  onLinux,
  pkgs,
  workstation,
  ...
}:
onLinux {
  # cargo builds a program with the config an account gets, and mold is the
  # linker that wrote it.
  cargo-links-with-mold =
    let
      files = (account workstation).home.file;
    in
    pkgs.runCommandCC "cargo-links-with-mold"
      {
        nativeBuildInputs = [
          pkgs.cargo
          pkgs.rustc
        ];
      }
      ''
        export HOME=$PWD CARGO_HOME=$PWD/.cargo
        # The account's build folder is on the machine's filesystem for
        # builds, which the sandbox hasn't got.
        export CARGO_BUILD_BUILD_DIR=$PWD/build
        mkdir -p .cargo hello/src
        cp ${files.".cargo/config.toml".source} .cargo/config.toml
        printf '[package]\nname = "hello"\nversion = "0.1.0"\nedition = "2024"\n' > hello/Cargo.toml
        echo 'fn main() {}' > hello/src/main.rs
        cargo build --offline --quiet --manifest-path hello/Cargo.toml
        readelf -p .comment hello/target/debug/hello | grep mold
        touch $out
      '';

  # A workstation's cargo keeps each workspace's intermediates on the
  # filesystem builds have to themselves, and the disk guard collects them
  # there as it does a target.
  cargo-builds-on-their-own-filesystem =
    let
      home = account workstation;
      folder = workstation.config.jitsusama.cargo.builds.folder;
      guarded = home.jitsusama.disk-guard.settings.sources.cargo-builds or null;
    in
    holds "cargo-builds-on-their-own-filesystem"
      "A workstation account's cargo builds outside the builds' own filesystem, the disk guard doesn't collect there, or nothing mounts it."
      (
        home.programs.cargo.settings.build.build-dir == "${folder}/someone/{workspace-path-hash}"
        && guarded.roots or [ ] == [ "${folder}/someone" ]
        && guarded.depth or null == 2
        && workstation.config.systemd.services ? cargo-builds
      );
}
