# The check that every file read whose failure becomes nothing says why, in
# a repository's Rust outside its tests, for core and for any repository
# built on it, as `core.lib.everyFailedReadSaysWhy pkgs`:
# every-failed-read-says-why.py, beside this, says what counts. It first
# sees the rule judge a planted tree, every-failed-read-says-why-planted.rs
# with its test module beside it, and name just the reads that go unsaid.
{
  lib,
  python3,
  runCommandLocal,
  writeText,
}:
let
  # The planted file's reads that go unsaid, one shape each; those not
  # named say why, propagate their error, fail loudly, are test code, or
  # hold their default only in another's argument or an inner match.
  said = writeText "said" ''
    src/lib.rs holds 20 reads whose failure is taken as nothing, 0 of them waiting: say that it couldn't read, as an error or a measure left unjudged, or, where a file not there truly holds nothing, say why in a comment on its statement or on a line of its own just above it.
      src/lib.rs:16: fs::read_to_string(path).unwrap_or_default()
      src/lib.rs:30: fs::read_to_string(path).unwrap_or(String::new())
      src/lib.rs:35: fs::read_to_string(path).unwrap_or_else(|_| String::new())
      src/lib.rs:48: fs::read_to_string(path).map_or(0, |text| text.lines().count())
      src/lib.rs:52: fs::read_to_string(path).is_ok_and(|text| !text.is_empty())
      src/lib.rs:56: let text = fs::read_to_string(path).ok()?;
      src/lib.rs:61: let Ok(text) = fs::read_to_string(path) else {
      src/lib.rs:68: if let Ok(text) = fs::read_to_string(path) {
      src/lib.rs:76: match fs::read_to_string(path) {
      src/lib.rs:84: Ok(text) => Ok(text + &fs::read_to_string(other).unwrap_or_default()),
      src/lib.rs:92: .map(|path| fs::read_to_string(path).ok())
      src/lib.rs:98: path.map(|path| fs::read_to_string(path))
      src/lib.rs:113: fs::read_to_string(path).unwrap_or_default()
      src/lib.rs:124: fs::read_to_string(path).unwrap_or_default()
      src/lib.rs:128: fs::read_to_string(path).into_iter().collect()
      src/lib.rs:132: match fs::read_to_string(path) {
      src/lib.rs:139: match fs::read_to_string(path) {
      src/lib.rs:146: match fs::read(path) {
      src/lib.rs:181: fs::read_to_string(path).unwrap_or_else(|_| std::process::exit(0))
      src/lib.rs:189: fs::read_to_string(path).map(|text| text + &fs::read_to_string(other).unwrap_or_default())
  '';
in
{
  # The repository's tree, as its flake sees it.
  src,
  # How many reads taking a failure as nothing each file held before the
  # rule, while each says why or says it couldn't read. The check fails on a
  # file holding more, and on one holding fewer until its count is lowered.
  waiting ? { },
  name ? "every-failed-read-says-why",
}:
# Kept in Nix because its lines only lay out store paths Nix names (the
# planted files, the rule and the tree) and run the rule twice; the rule
# itself lives in every-failed-read-says-why.py.
runCommandLocal name { } ''
  mkdir -p planted/src
  cp ${./every-failed-read-says-why-planted.rs} planted/src/lib.rs
  cp ${./every-failed-read-says-why-planted-golden.rs} planted/src/golden.rs
  echo '{}' > none.json
  if ${lib.getExe python3} ${./every-failed-read-says-why.py} planted none.json 2> heard; then
    echo "every-failed-read-says-why passed a planted tree holding unsaid reads" >&2
    exit 1
  fi
  diff ${said} heard
  ${lib.getExe python3} ${./every-failed-read-says-why.py} ${src} ${writeText "waiting.json" (builtins.toJSON waiting)}
  touch $out
''
