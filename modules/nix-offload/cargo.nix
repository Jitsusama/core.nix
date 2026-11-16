# The dev shell's cargo, ahead of cargo itself on its path (cargo.sh says
# what it does). Made with runCommand rather than writeShellApplication,
# since it's cargo by another door, answering as cargo does, not a command
# of its own with its own help.
{
  bash,
  cargo,
  coreutils,
  lib,
  replaceVars,
  runCommand,
}:
runCommand "cargo-for-agents"
  {
    nativeBuildInputs = [ bash ];
    meta.mainProgram = "cargo";
  }
  ''
    install -Dm755 ${
      replaceVars ./cargo.sh {
        cargo = lib.getExe cargo;
        true = lib.getExe' coreutils "true";
      }
    } $out/bin/cargo
    patchShebangs $out/bin/cargo
  ''
