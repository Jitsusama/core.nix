# The disk guard's finished predicate for a checkout of GitHub's
# (merged-on-github.sh), given the gh it asks, so a check can give it a
# stand-in.
{
  gh,
  gnugrep,
  gnused,
  writeShellApplication,
}:
writeShellApplication {
  name = "disk-guard-merged-on-github";
  runtimeInputs = [
    gh
    gnugrep
    gnused
  ];
  text = builtins.readFile ./merged-on-github.sh;
}
