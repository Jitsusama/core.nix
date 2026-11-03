# The checks of sccache, which checks.nix gathers with every other folder's;
# its opening comment says what each kind of check proves.
{
  account,
  lib,
  onLinux,
  pkgs,
  workstation,
  ...
}:
onLinux {
  # Two workspaces in folders of their own, as two worktrees are, each build
  # a dependency at one place through the wrapper the account's shells and
  # user manager are given, and the second build's compile of it comes from
  # the cache. A path dependency would build incrementally, which sccache
  # passes through, so incremental builds are off the same way in both, as a
  # registry's crates are.
  sccache-shares-builds-across-worktrees =
    let
      home = account workstation;
      exported =
        variables:
        lib.concatStrings (
          lib.mapAttrsToList (name: value: "export ${name}=${lib.escapeShellArg value}\n") (
            lib.filterAttrs (name: _: name == "RUSTC_WRAPPER" || lib.hasPrefix "SCCACHE_" name) variables
          )
        );
      builds = place: variables: ''
        (
          ${exported variables}
          export SCCACHE_DIR=$PWD/${place}-cache
          sccache=''${RUSTC_WRAPPER:?the account in ${place} has no RUSTC_WRAPPER}
          $sccache --start-server >/dev/null
          for tree in one two; do
            $sccache --zero-stats >/dev/null
            cargo build --offline --quiet --manifest-path $tree/Cargo.toml
          done
          hits=$($sccache --show-stats --stats-format json | jq '.stats.cache_hits.counts.Rust // 0')
          $sccache --stop-server >/dev/null
          rm -rf one/target two/target
          [ "$hits" -ge 1 ] || { echo "the second worktree in ${place} hit the cache $hits times"; exit 1; }
        )
      '';
    in
    pkgs.runCommandCC "sccache-shares-builds-across-worktrees"
      {
        nativeBuildInputs = [
          pkgs.cargo
          pkgs.jq
          pkgs.rustc
        ];
      }
      ''
        export HOME=$PWD CARGO_HOME=$PWD/.cargo CARGO_INCREMENTAL=0
        export SCCACHE_SERVER_UDS=$PWD/sccache.sock SCCACHE_IDLE_TIMEOUT=0
        mkdir -p shared/src
        printf '[package]\nname = "shared"\nversion = "0.1.0"\nedition = "2024"\n' > shared/Cargo.toml
        echo 'pub fn answer() -> u32 { 42 }' > shared/src/lib.rs
        for tree in one two; do
          mkdir -p $tree/src
          printf '[package]\nname = "%s"\nversion = "0.1.0"\nedition = "2024"\n\n[dependencies]\nshared = { path = "../shared" }\n' $tree > $tree/Cargo.toml
          echo 'fn main() { println!("{}", shared::answer()); }' > $tree/src/main.rs
        done
        ${builds "a shell" home.home.sessionVariables}
        ${builds "the user manager" home.systemd.user.sessionVariables}
        touch $out
      '';
}
