# The JavaScript linter `nix fmt` runs (qt-javascript-lint.sh), with what it
# runs on its path. Core's biome.jsonc says which rules it holds the files
# to, given by its place in the store, so every repository taking core's
# linters holds its QML's JavaScript to the same rules.
{
  biome,
  gnused,
  writeShellApplication,
}:
writeShellApplication {
  name = "qt-javascript-lint";
  runtimeInputs = [
    biome
    gnused
  ];
  runtimeEnv.QT_JAVASCRIPT_LINT_CONFIG = "${../../biome.jsonc}";
  text = builtins.readFile ./qt-javascript-lint.sh;
}
