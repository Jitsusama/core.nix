# Boots a kernel that knows the keys its modules are signed with, so each
# module it loads is checked against its signature rather than refused a
# check and the kernel tainted. NixOS signs every module with a key of its
# own and keeps the certificates beside them, but the kernel image it builds
# carries none; nixpkgs' rebuildImage writes them into the image's reserved
# room and relinks it, without compiling the kernel again. Secure Boot
# checks the image lanzaboote signs, so with this the modules it loads are
# checked too.
{
  config,
  lib,
  options,
  pkgs,
  ...
}:
let
  kernelFile = config.system.boot.loader.kernelFile;
  image = config.boot.kernelPackages.rebuildImage {
    target = kernelFile;
    inherit (config.system) modulesTree;
  };
  kernel = "${image}/${kernelFile}";
in
{
  system.systemBuilderCommands = lib.mkAfter ''
    ln -sf -- ${lib.escapeShellArg kernel} "$out/kernel"
  '';

  # Bootspec names the kernel package's own image, and lanzaboote builds
  # each generation's stub from what bootspec names, so the image with the
  # certificates goes in its place there too.
  boot.bootspec.writer = ''
    ${options.boot.bootspec.writer.default}
    ${lib.getExe pkgs.buildPackages.jq} --sort-keys --arg kernel ${lib.escapeShellArg kernel} \
      '."org.nixos.bootspec.v1".kernel = $kernel' \
      "$out/${config.boot.bootspec.filename}" > "$TMPDIR/bootspec.json"
    mv -- "$TMPDIR/bootspec.json" "$out/${config.boot.bootspec.filename}"
  '';
}
