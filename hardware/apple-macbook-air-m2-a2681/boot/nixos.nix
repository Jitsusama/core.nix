# m1n1's second stage, which the m1n1 Asahi's installer left in macOS's
# place loads from the EFI system partition at each start: nixpkgs' m1n1,
# this Mac's device tree from core's kernel, and core's U-Boot, joined into
# m1n1/boot.bin and put there by systemd-boot on each switch. It follows
# nixos-apple-silicon's boot-m1n1 module (MIT, Thomas Watson and its
# contributors), whose options it leaves out, since nothing here sets one.
{ config, pkgs, ... }:
let
  inherit (pkgs) m1n1;
  uBoot = pkgs.callPackage ./u-boot.nix { };

  # This Mac's device tree alone, from the kernel it starts, so a change to
  # the kernel's device trees, fairydust's screen among them, reaches the
  # boot chain with the kernel. m1n1 chooses among the trees appended to it
  # by the Mac it runs on, and here there's one.
  bootBin = pkgs.runCommand "boot.bin" { } ''
    cat ${m1n1}/lib/m1n1/m1n1.bin \
        ${config.boot.kernelPackages.kernel}/dtbs/apple/t8112-j413.dtb \
        ${uBoot}/u-boot-nodtb.bin.gz \
        > $out
  '';
in
{
  boot.loader.systemd-boot.extraFiles."m1n1/boot.bin" = bootBin;

  # On a stick, so the guide installs a system whose boot chain is already
  # there, with nothing to fetch.
  system.extraDependencies = [
    m1n1
    uBoot
  ];

  system.build.m1n1 = bootBin;
}
