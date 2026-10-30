# Surface UEFI for the Surface Laptop 4 with AMD, as a folder fwupd reads as a
# remote: its capsule, taken from Microsoft's package of drivers and firmware
# for the model, in a cabinet with the metadata LVFS would have given it.
# The build fails unless the capsule is the UEFI's, at the version named here
# (capsule-is-for.sh), so a newer package only needs its URL, hash, version
# and date changed.
{
  lib,
  stdenvNoCC,
  fetchurl,
  fwupd,
  msitools,
  replaceVars,
}:
let
  version = "4.501.140";
  # Surface UEFI's entry in this model's ESRT, which the capsule updates.
  guid = "6722ccaf-93b4-49c4-a028-24c809b7dd4c";
  # The version as the firmware reads it: 8, 16 and 8 bits.
  number =
    let
      parts = map lib.toInt (lib.splitString "." version);
    in
    lib.elemAt parts 0 * 16777216 + lib.elemAt parts 1 * 256 + lib.elemAt parts 2;
  metainfo = replaceVars ./uefi.metainfo.xml {
    inherit guid version;
    # Its release through Windows Update.
    date = "2026-08-13";
  };
in
stdenvNoCC.mkDerivation {
  pname = "surface-laptop-4-amd-uefi";
  inherit version;

  # 702 MB of drivers and firmware, of which the capsule is 7 MB.
  src = fetchurl {
    url = "https://download.microsoft.com/download/7ee743a7-aac6-4b0c-83bf-218070e93e39/SurfaceLaptop4_AMD_Win11_22631_26.072.27963.0.msi";
    hash = "sha256-hCLPuXA27JqhC9lCdIgltCc7gl2t1wXDrLuDsDySDtI=";
  };
  dontUnpack = true;

  nativeBuildInputs = [
    fwupd
    msitools
  ];

  buildPhase = ''
    runHook preBuild
    msiextract -C package "$src" > /dev/null
    cp package/SurfaceUpdate/surfaceuefi/Surface_UEFI_${version}.0.bin firmware.bin
    bash ${./capsule-is-for.sh} firmware.bin ${guid} ${toString number}
    cp ${metainfo} firmware.metainfo.xml
    HOME=$TMPDIR fwupdtool build-cabinet surface-uefi-${version}.cab \
      firmware.bin firmware.metainfo.xml
    runHook postBuild
  '';

  installPhase = ''
    runHook preInstall
    install -Dm444 -t $out surface-uefi-${version}.cab
    runHook postInstall
  '';

  meta = {
    description = "Surface UEFI for the Surface Laptop 4 with AMD, for fwupd";
    homepage = "https://support.microsoft.com/en-us/surface/updates/surface-laptop-4-update-history";
    license = lib.licenses.unfree;
    sourceProvenance = [ lib.sourceTypes.binaryFirmware ];
    platforms = [ "x86_64-linux" ];
  };
}
