# The check that every per-file exception in core's ruff config still hides a
# finding: every-ruff-exception-hides-a-finding.py, beside this, says what
# counts. Repositories built on core lint with core's ruff.toml, through its
# treefmt module, so this is core's alone, not a `core.lib` rule. It first sees
# the rule fail a planted tree, refuse a planted config and fail one ruff can't
# read, as every-ruff-exception-hides-a-finding.sh says.
{
  bash,
  lib,
  python3,
  ruff,
  runCommandLocal,
  writeText,
}:
let
  # Each file but clean.py holds what the planted config's comments say.
  planted = runCommandLocal "every-ruff-exception-hides-a-finding-planted" { } ''
    mkdir $out $out/refused $out/broken
    cp ${./every-ruff-exception-hides-a-finding-planted.toml} $out/ruff.toml
    touch $out/clean.py
    for name in hidden prefixed twice pair; do echo "import os" > $out/$name.py; done
    echo "print(1)" > $out/printed.py
    cp ${./every-ruff-exception-hides-a-finding-refused.toml} $out/refused/ruff.toml
    cp ${./every-ruff-exception-hides-a-finding-broken.toml} $out/broken/ruff.toml
    echo "import os" | tee $out/refused/b.py > $out/broken/b.py
  '';
  said = writeText "said" ''
    ruff.toml: F401 in clean.py hides no finding another exception doesn't: take it off.
    ruff.toml: F401 in twice.py hides no finding another exception doesn't: take it off.
    ruff.toml: F4 in pair.py hides no finding another exception doesn't: take it off.
    ruff.toml: T201 in printed.py hides no finding another exception doesn't: take it off.
  '';
  refused = writeText "refused" ''
    ruff.toml: extend isn't read: fold the config it names in.
    ruff.toml: ignore isn't read: move it to lint.ignore.
    ruff.toml: lint.extend-ignore isn't read: move it to lint.ignore.
    ruff.toml: lint.extend-per-file-ignores isn't read: move it to lint.per-file-ignores.
    ruff.toml: lint.select isn't ALL, the one selection this judges under.
    ruff.toml: lint.ignore's F4 is broader than exception code F401: name its rules.
    ruff.toml is refused, so none of its exceptions is judged.
  '';
in
{ src }:
runCommandLocal "every-ruff-exception-hides-a-finding" { nativeBuildInputs = [ ruff ]; } ''
  ${lib.getExe bash} ${./every-ruff-exception-hides-a-finding.sh} \
    ${lib.getExe python3} ${./every-ruff-exception-hides-a-finding.py} \
    ${planted} ${said} ${refused} ${src}
  touch $out
''
