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

  patches = [
    # The panel sends a 4,302-byte heat map about 100 times a second while
    # touched, and on some starts untouched too, and its controller's FIFO
    # was read a byte at a time: 7.6 ms of a core for each, about 70% of
    # one. Read four bytes at a time, each takes 4.4 ms.
    ./fifo-reads-four-bytes.patch
    # After a second with no contact the panel rests, looking for three
    # frames after each 100 ms, about a quarter of the heat maps it sent.
    # touch_stats says without root how many heat maps held a contact,
    # how often it rested, where the last contacts were and what else the
    # panel sent; heat_scan stops or starts the heat maps by hand.
    ./heat-rests-when-untouched.patch
    # Shut, the lid presses on the glass, and the panel reads two or three
    # contacts along its top edge about 100 times a second. With the
    # laptop's own screen off at a desk, those touches land on the monitor
    # and drag its windows about. While the lid's switch reads shut, the
    # heat maps stop and any touch still down is lifted; they start again
    # when it opens. touch_stats says lid_shut and how many times it was.
    ./heat-stops-while-the-lid-is-shut.patch
  ];

  nativeBuildInputs = kernel.moduleBuildDependencies;
  makeFlags = kernelModuleMakeFlags ++ [
    "KDIR=${kernel.dev}/lib/modules/${kernel.modDirVersion}/build"
    "INSTALL_MOD_PATH=${placeholder "out"}"
    "INSTALL_MOD_DIR=extra"
    "DEPMOD=true"
  ];

  # The driver's Makefile has no install target, so kbuild's own installs the
  # modules: it signs each one and writes the certificate beside it, which the
  # system's module tree gathers into its own and fails without.
  installFlags = [
    "-C"
    "${kernel.dev}/lib/modules/${kernel.modDirVersion}/build"
    "M=$(PWD)"
  ];
  installTargets = [ "modules_install" ];

  meta = {
    description = "Touchscreen and pen driver for the Surface Laptop 3 and 4 with AMD";
    homepage = "https://github.com/avgbob/SL4A_TouchScreen";
    license = lib.licenses.gpl2Only;
    platforms = [ "x86_64-linux" ];
  };
}
