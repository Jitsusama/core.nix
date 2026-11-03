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
  # The account's server starts as its unit starts it, among the jobs. Then
  # two workspaces in folders of their own, as two worktrees are, each build
  # a dependency at one place, through what the account's shells and its
  # user manager are given, and the second build's compile of it comes from
  # the cache. A path dependency would build incrementally, which sccache
  # passes through, so incremental builds are off the same way in both, as a
  # registry's crates are.
  sccache-shares-builds-across-worktrees =
    let
      home = account workstation;
      unit = home.systemd.user.services.sccache;
      # The unit's settings for a shell, with the runtime folder systemd
      # gives as %t.
      serverEnvironment = lib.concatMapStrings (
        setting: "export ${lib.replaceStrings [ "%t" ] [ "$XDG_RUNTIME_DIR" ] setting}\n"
      ) (lib.toList unit.Service.Environment);
      # Left unquoted inside double quotes, so the shell expands the runtime
      # folder each value names, as a login does.
      exported =
        variables:
        lib.concatStrings (
          lib.mapAttrsToList (name: value: "export ${name}=\"${value}\"\n") (
            lib.filterAttrs (name: _: name == "RUSTC_WRAPPER" || lib.hasPrefix "SCCACHE_" name) variables
          )
        );
      builds = place: variables: ''
        (
          # The server has the unit's settings and the builds only their own,
          # as under the user manager, and an empty cache each time, so a hit
          # can only be the first tree's.
          (
            ${serverEnvironment}
            SCCACHE_DIR=$(mktemp -d -p "$PWD")
            export SCCACHE_DIR
            exec ${lib.escapeShellArgs (lib.toList unit.Service.ExecStart)}
          ) &
          server=$!
          (
            ${exported variables}
            # The socket the builds name is the one the unit's server made.
            for _ in $(seq 100); do [ -S "$SCCACHE_SERVER_UDS" ] && break; sleep 0.1; done
            [ -S "$SCCACHE_SERVER_UDS" ] \
              || { echo "builds in ${place} find no server at $SCCACHE_SERVER_UDS"; exit 1; }
            for tree in one two; do
              cargo build --offline --quiet --manifest-path $tree/Cargo.toml
            done
            hits=$($RUSTC_WRAPPER --show-stats --stats-format json | jq '.stats.cache_hits.counts.Rust // 0')
            [ "$hits" -ge 1 ] || { echo "the second worktree in ${place} hit the cache $hits times"; exit 1; }
          ) || { kill $server; exit 1; }
          kill $server
          wait $server || true
          rm -rf one/target two/target
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
        [ ${unit.Service.Slice} = jobs.slice ] \
          || { echo "sccache's server runs in ${unit.Service.Slice}, not among the jobs"; exit 1; }
        export HOME=$PWD CARGO_HOME=$PWD/.cargo CARGO_INCREMENTAL=0 XDG_RUNTIME_DIR=$PWD/run
        mkdir -p run shared/src
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
