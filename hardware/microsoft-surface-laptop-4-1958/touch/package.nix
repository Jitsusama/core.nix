# The touchscreen's driver, built against the machine's own kernel: a module
# for the AMDI0060 SPI controller and one for the MSHW0231 panel behind it,
# as HID over SPI. v1.7.1 is the release its author asks owners of this
# model to test.
{
  lib,
  stdenv,
  fetchFromGitHub,
  kernel,
  kernelModuleMakeFlags,
}:
stdenv.mkDerivation {
  pname = "sl4a-touchscreen";
  version = "1.7.1-${kernel.version}";

  src = fetchFromGitHub {
    owner = "avgbob";
    repo = "SL4A_TouchScreen";
    rev = "eccedfb606e579242950052de2022d445dee028c";
    hash = "sha256-sXwDq1v5tpTQO863AS+6IPr9GUewQvrjIePWFO/Jlvo=";
  };
  sourceRoot = "source/driver";

  nativeBuildInputs = kernel.moduleBuildDependencies;
  makeFlags = kernelModuleMakeFlags ++ [
    "KDIR=${kernel.dev}/lib/modules/${kernel.modDirVersion}/build"
  ];

  installPhase = ''
    runHook preInstall
    install -Dm444 -t $out/lib/modules/${kernel.modDirVersion}/extra sl4a-spi-amd.ko sl4a-spi-hid.ko
    runHook postInstall
  '';

  meta = {
    description = "Touchscreen and pen driver for the Surface Laptop 3 and 4 with AMD";
    homepage = "https://github.com/avgbob/SL4A_TouchScreen";
    license = lib.licenses.gpl2Only;
    platforms = [ "x86_64-linux" ];
  };
}
