# The check that every per-file exception in core's ruff config still hides a
# finding: every-ruff-exception-hides-a-finding.py, beside this, says what
# counts. Repositories built on core lint with core's ruff.toml, through its
# treefmt module, so this is core's alone, not a `core.lib` rule. It first sees
# the rule fail on a planted tree and refuse a planted config, so a rule that
# passes everything can't pass.
{
  lib,
  python3,
  ruff,
  runCommandLocal,
  writeText,
}:
let
  rule = "${lib.getExe python3} ${./every-ruff-exception-hides-a-finding.py}";
  # Each file but clean.py holds what the planted config's comments say.
  planted = runCommandLocal "every-ruff-exception-hides-a-finding-planted" { } ''
    mkdir $out
    cp ${./every-ruff-exception-hides-a-finding-planted.toml} $out/ruff.toml
    touch $out/clean.py
    for name in hidden prefixed twice pair; do echo "import os" > $out/$name.py; done
    echo "print(1)" > $out/printed.py
    mkdir $out/refused
    cp ${./every-ruff-exception-hides-a-finding-refused.toml} $out/refused/ruff.toml
    touch $out/refused/base.toml
    echo "import os" > $out/refused/b.py
  '';
  said = writeText "said" ''
    ruff.toml: F401 in clean.py hides no finding another exception doesn't: take it off.
    ruff.toml: F401 in twice.py hides no finding another exception doesn't: take it off.
    ruff.toml: F4 in pair.py hides no finding another exception doesn't: take it off.
    ruff.toml: T201 in printed.py hides no finding another exception doesn't: take it off.
  '';
  refused = writeText "refused" ''
    ruff.toml: lint.extend-per-file-ignores isn't read: move it to lint.per-file-ignores.
    ruff.toml: extend isn't read: fold the config it names in.
  '';
in
{ src }:
runCommandLocal "every-ruff-exception-hides-a-finding" { nativeBuildInputs = [ ruff ]; } ''
  if ${rule} ${planted} ${planted}/ruff.toml 2> planted-said; then
    echo "the rule passed its planted tree, which it must fail"; exit 1
  fi
  diff ${said} planted-said
  if ${rule} ${planted}/refused ${planted}/refused/ruff.toml 2> refused-said; then
    echo "the rule passed its refused config, which it must refuse"; exit 1
  fi
  head -n 2 refused-said | diff ${refused} -
  grep -q "^ruff couldn't check .* with ruff.toml:$" refused-said
  ${rule} ${src} ${src}/ruff.toml
  touch $out
''
