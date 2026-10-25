# A qemu for the VM tests whose qemu-system programs start only once the
# sockets they connect to are there (./qemu-after-its-sockets.sh), so a test
# never hangs on a virtiofsd that was slower to start than qemu. Everything
# else, qemu-img among it, is the qemu given. checks.nix puts it under every
# VM test's own qemu.
{
  lib,
  coreutils,
  inotify-tools,
  procps,
  runCommandLocal,
  runtimeShell,
  writeShellScript,
}:
qemu:
let
  waits = writeShellScript "qemu-after-its-sockets" ''
    export PATH="${
      lib.makeBinPath [
        coreutils
        inotify-tools
        procps
      ]
    }"
    ${builtins.readFile ./qemu-after-its-sockets.sh}
  '';
in
runCommandLocal "${qemu.name}-after-its-sockets"
  {
    # nixpkgs reads the host's system from the qemu a test runs.
    passthru = { inherit (qemu) stdenv; };
  }
  ''
    mkdir -p $out/bin
    for path in ${qemu}/*; do
      [ "''${path##*/}" = bin ] || ln -s "$path" $out/
    done
    for program in ${qemu}/bin/*; do
      name=''${program##*/}
      case $name in
        qemu-system-*)
          printf '#!%s\nexec %s %s "$@"\n' ${runtimeShell} ${waits} "$program" > $out/bin/$name
          chmod +x $out/bin/$name
          ;;
        *) ln -s "$program" $out/bin/$name ;;
      esac
    done
  ''
