# The kernel module in stand-in-light.c, built for whichever kernel the test
# machine boots. Call it from that machine's kernel packages:
# config.boot.kernelPackages.callPackage ./stand-in-light { }
{
  stdenv,
  kernel,
  kernelModuleMakeFlags,
}:
stdenv.mkDerivation {
  pname = "stand-in-light";
  inherit (kernel) version;
  src = ./.;

  nativeBuildInputs = kernel.moduleBuildDependencies;

  # The kernel's own build, pointed at this directory, whose Kbuild file names
  # the module.
  makeFlags = kernelModuleMakeFlags ++ [
    "-C"
    "${kernel.dev}/lib/modules/${kernel.modDirVersion}/build"
    "M=$(PWD)"
  ];
  buildFlags = [ "modules" ];

  # The kernel's own install, which signs the module and leaves its
  # certificate beside it, where NixOS gathers it into the kernel's trusted
  # set. depmod is NixOS's to run, over every module at once.
  installFlags = [
    "INSTALL_MOD_PATH=${placeholder "out"}"
    "INSTALL_MOD_DIR=extra"
    "DEPMOD=true"
  ];
  installTargets = [ "modules_install" ];
}
