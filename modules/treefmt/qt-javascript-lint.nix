# The JavaScript linter `nix fmt` runs (qt-javascript-lint.sh), with what it
# runs on its path. biome.jsonc says which rules it holds the files to.
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
  text = builtins.readFile ./qt-javascript-lint.sh;
}
