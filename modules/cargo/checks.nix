# The checks of cargo, which checks.nix gathers with every other folder's;
# its opening comment says what each kind of check proves.
{
  account,
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
        mkdir -p .cargo hello/src
        cp ${files.".cargo/config.toml".source} .cargo/config.toml
        printf '[package]\nname = "hello"\nversion = "0.1.0"\nedition = "2024"\n' > hello/Cargo.toml
        echo 'fn main() {}' > hello/src/main.rs
        cargo build --offline --quiet --manifest-path hello/Cargo.toml
        readelf -p .comment hello/target/debug/hello | grep mold
        touch $out
      '';
}
