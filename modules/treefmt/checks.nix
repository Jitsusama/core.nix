# The checks of treefmt, which checks.nix gathers with every other folder's;
# its opening comment says what each kind of check proves.
{ onLinux, pkgs, ... }:
onLinux {
  # Two files no formatter reads, listed first in a repository of thousands,
  # fail the run inside a minute and are named, where released treefmt waits
  # on git for good (package.nix).
  a-file-no-linter-reads-fails-at-once =
    pkgs.runCommandLocal "a-file-no-linter-reads-fails-at-once"
      {
        nativeBuildInputs = [
          (pkgs.callPackage ./package.nix { })
          pkgs.git
        ];
      }
      ''
        export HOME=$TMPDIR
        cd "$(mktemp -d)"
        git init -q .
        mkdir a
        echo unread > a/first.unread
        echo unread > a/second.unread
        for i in $(seq 3000); do echo read > "file-$i.sh"; done
        cat > treefmt.toml <<'EOF'
        on-unmatched = "fatal"
        excludes = ["treefmt.toml"]
        [formatter.nothing]
        command = "true"
        includes = ["*.sh"]
        EOF
        status=0
        timeout -s KILL 60 treefmt --no-cache > said 2>&1 || status=$?
        cat said
        if [ "$status" -eq 137 ]; then
          echo "treefmt was still waiting after a minute rather than failing" >&2
          exit 1
        fi
        if [ "$status" -eq 0 ] || ! grep -q 'no formatter for path: a/' said; then
          echo "treefmt exited $status without naming the unread file" >&2
          exit 1
        fi
        touch $out
      '';
}
