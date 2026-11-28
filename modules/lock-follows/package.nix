# The lock-follows command (lock-follows.sh): git and jq are its own, while
# nix and the pull request command come from the PATH it's run with, the
# account's, so a check can give stand-ins for both.
{
  coreutils,
  gawk,
  git,
  gnused,
  jq,
  writeShellApplication,
}:
writeShellApplication {
  name = "lock-follows";
  runtimeInputs = [
    coreutils
    gawk
    git
    gnused
    jq
  ];
  text = builtins.readFile ./lock-follows.sh;
}
