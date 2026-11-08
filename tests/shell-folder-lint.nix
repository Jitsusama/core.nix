# The QML linter the offscreen checks run on the shell each lays out
# (shell-folder-lint.sh), with what it runs on its path.
{
  callPackage,
  coreutils,
  findutils,
  gnugrep,
  kdePackages,
  writeShellApplication,
}:
writeShellApplication {
  name = "shell-folder-lint";
  runtimeInputs = [
    coreutils
    findutils
    gnugrep
    kdePackages.qtdeclarative
    (callPackage ./qmllint-counted.nix { })
  ];
  text = builtins.readFile ./shell-folder-lint.sh;
}
