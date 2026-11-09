# qmllint printing every message it counts toward its exit, which its own
# text report can leave out (qmllint-counted.sh), with what it runs on its
# path.
{
  coreutils,
  jq,
  kdePackages,
  writeShellApplication,
}:
writeShellApplication {
  name = "qmllint-counted";
  runtimeInputs = [
    coreutils
    jq
    kdePackages.qtdeclarative
  ];
  text = builtins.readFile ./qmllint-counted.sh;
}
