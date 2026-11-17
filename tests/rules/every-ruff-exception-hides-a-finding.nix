# The check that every per-file exception in core's ruff config still hides a
# finding: every-ruff-exception-hides-a-finding.py, beside this, says what
# counts. Repositories built on core lint with core's ruff.toml, through its
# treefmt module, so this is core's alone, not a `core.lib` rule. It first sees
# the rule fail on a planted tree, so a rule that passes everything can't pass.
{
  lib,
  python3,
  ruff,
  runCommandLocal,
  writeText,
}:
let
  rule = "${lib.getExe python3} ${./every-ruff-exception-hides-a-finding.py}";
  # Each file but clean.py holds an unused import, which the planted config's
  # exceptions hide as its comments say.
  planted = runCommandLocal "every-ruff-exception-hides-a-finding-planted" { } ''
    mkdir $out && cp ${./every-ruff-exception-hides-a-finding-planted.toml} $out/ruff.toml && touch $out/clean.py
    for name in hidden prefixed twice; do echo "import os" > $out/$name.py; done
  '';
  said = writeText "said" ''
    ruff.toml: F401 in clean.py hides no finding another exception doesn't: take it off.
    ruff.toml: F401 in twice.py hides no finding another exception doesn't: take it off.
  '';
in
{ src }:
runCommandLocal "every-ruff-exception-hides-a-finding" { nativeBuildInputs = [ ruff ]; } ''
  if ${rule} ${planted} ${planted}/ruff.toml 2> planted-said; then exit 1; fi
  diff ${said} planted-said
  ${rule} ${src} ${src}/ruff.toml
  touch $out
''
