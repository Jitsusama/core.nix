# The check that every fixed wait in a repository's tests polls or says why,
# for core and for any repository built on it, as
# `core.lib.everyFixedWaitSaysWhy pkgs`: every-fixed-wait-says-why.py, beside
# this, says what counts. It first sees the rule judge a planted Rust test,
# every-fixed-wait-says-why-planted.rs.in, and name just its nine bare
# sleeps, past the strings, characters and comments that would pass a sleep
# or name one, the `for` of a trait or a bound, which opens no loop, and a
# `fn sleep` that defines one.
{
  lib,
  python3,
  runCommandLocal,
  writeText,
}:
let
  # The rule beside the Rust reader it shares, so it finds the reader.
  rule = lib.fileset.toSource {
    root = ./.;
    fileset = lib.fileset.unions [
      ./every-fixed-wait-says-why.py
      ./rust_source.py
    ];
  };
  # The planted test's sleeps that neither poll nor say why.
  said = writeText "said" ''
    tests/waits.rs:14
    tests/waits.rs:19
    tests/waits.rs:58
    tests/waits.rs:67
    tests/waits.rs:74
    tests/waits.rs:80
    tests/waits.rs:97
    tests/waits.rs:109
    tests/waits.rs:124
  '';
in
{
  # The repository's tree, as its flake sees it.
  src,
  # How many fixed waits without a reason each test file held before the rule,
  # while each says why or becomes a wait on its event. The check fails on a
  # file holding more, and on one holding fewer until its count is lowered.
  waiting ? { },
  name ? "every-fixed-wait-says-why",
}:
# Kept in Nix because its lines only lay out store paths Nix names (the
# planted test, the rule and the tree) and run the rule twice; the rule
# itself lives in every-fixed-wait-says-why.py.
runCommandLocal name { } ''
  mkdir -p planted/tests
  cp ${./every-fixed-wait-says-why-planted.rs.in} planted/tests/waits.rs
  echo '{}' > none.json
  if ${lib.getExe python3} ${rule}/every-fixed-wait-says-why.py planted none.json 2> heard; then
    echo "every-fixed-wait-says-why passed a planted test holding bare sleeps" >&2
    exit 1
  fi
  # A run naming nothing still reaches the diff, which then shows what the
  # rule said instead.
  grep -o 'tests/waits.rs:[0-9][0-9]*' heard > named || true
  if ! diff ${said} named; then
    echo "every-fixed-wait-says-why named other sleeps of the planted test than its bare ones; it said:" >&2
    cat heard >&2
    exit 1
  fi
  ${lib.getExe python3} ${rule}/every-fixed-wait-says-why.py ${src} ${writeText "waiting.json" (builtins.toJSON waiting)}
  touch $out
''
