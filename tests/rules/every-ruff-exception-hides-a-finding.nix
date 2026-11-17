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
  # The three configs the check plants, each named as it is here.
  configs = lib.fileset.toSource {
    root = ./.;
    fileset = lib.fileset.unions [
      ./every-ruff-exception-hides-a-finding-planted.toml
      ./every-ruff-exception-hides-a-finding-refused.toml
      ./every-ruff-exception-hides-a-finding-broken.toml
    ];
  };
  said = writeText "said" ''
    ruff.toml: F401 in clean.py hides no finding another exception doesn't: take it off.
    ruff.toml: F401 in twice.py hides no finding another exception doesn't: take it off.
    ruff.toml: F4 in pair.py hides no finding another exception doesn't: take it off.
    ruff.toml: T201 in printed.py hides no finding another exception doesn't: take it off.
  '';
  refusals = writeText "refusals" ''
    ruff.toml: extend isn't read: fold the config it names in.
    ruff.toml: per-file-ignores isn't read: move it to lint.per-file-ignores.
    ruff.toml: extend-per-file-ignores isn't read: move it to lint.per-file-ignores.
    ruff.toml: lint.extend-per-file-ignores isn't read: move it to lint.per-file-ignores.
    ruff.toml is refused, so none of its exceptions is judged.
  '';
in
{ src }:
runCommandLocal "every-ruff-exception-hides-a-finding" { nativeBuildInputs = [ ruff ]; } ''
  ${lib.getExe bash} ${./every-ruff-exception-hides-a-finding.sh} \
    ${lib.getExe python3} ${./every-ruff-exception-hides-a-finding.py} ${configs} \
    ${said} ${refusals} ${src}
  touch $out
''
