# The NPU's graph compiler, which turns a model into the NPU's own program.
# Intel builds it from its open NPU compiler and releases it beside the NPU
# driver, of the same version, prebuilt; nixpkgs's driver is built without it,
# so every model failed to compile with ZE_RESULT_ERROR_UNSUPPORTED_FEATURE.
# The driver loads it from its own directory, so it goes into the graphics
# driver directory with the driver.
{
  lib,
  stdenv,
  fetchurl,
  autoPatchelfHook,
  dpkg,
  onetbb,
  zlib,
  zstd,
}:
let
  releases = "https://github.com/intel/linux-npu-driver/releases/download";
  build = "20260910-34487311128";
in
stdenv.mkDerivation (finalAttrs: {
  pname = "intel-npu-compiler";
  version = "1.38.0";

  src = fetchurl {
    url =
      let
        v = finalAttrs.version;
      in
      "${releases}/v${v}/linux-npu-driver-v${v}.${build}-ubuntu2604.tar.gz";
    hash = "sha256-g2hebZL02xymXkQoRtSHx/6xM1Y0szmUKSzrOPkzvMc=";
  };

  nativeBuildInputs = [
    autoPatchelfHook
    dpkg
  ];
  buildInputs = [
    onetbb
    zlib
    zstd
    stdenv.cc.cc
  ];

  sourceRoot = ".";
  unpackCmd = "tar xzf $curSrc";

  installPhase = ''
    runHook preInstall
    dpkg-deb -x intel-driver-compiler-npu_*.deb deb
    install -Dm555 -t $out/lib deb/usr/lib/x86_64-linux-gnu/libopenvino_intel_npu_compiler*.so
    runHook postInstall
  '';

  meta = {
    description = "Intel NPU graph compiler, as Intel releases it with the NPU driver";
    homepage = "https://github.com/intel/linux-npu-driver";
    license = lib.licenses.mit;
    sourceProvenance = [ lib.sourceTypes.binaryNativeCode ];
    platforms = [ "x86_64-linux" ];
  };
})
