# The QML linter the offscreen checks run on the shell each lays out
# (shell-folder-lint.sh), with what it runs on its path.
{
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
  ];
  text = builtins.readFile ./shell-folder-lint.sh;
}
