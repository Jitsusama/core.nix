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

  installPhase = ''
    install -D stand-in-light.ko \
      $out/lib/modules/${kernel.modDirVersion}/extra/stand-in-light.ko
  '';
}
