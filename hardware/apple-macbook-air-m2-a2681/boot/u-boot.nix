# Asahi's U-Boot, which m1n1 starts and which gives the Mac a plain UEFI for
# systemd-boot. The recipe follows nixos-apple-silicon's uboot-asahi (MIT,
# Thomas Watson and its contributors), at the same tag.
{ buildUBoot, fetchFromGitHub }:
buildUBoot rec {
  src = fetchFromGitHub {
    owner = "AsahiLinux";
    repo = "u-boot";
    rev = "asahi-v2026.07-2";
    hash = "sha256-9E79VceZcPw55zj6BBwgIBP/2OrHRS/w3ypGyOWfE6U=";
  };
  version = "2026.07-2-asahi";

  defconfig = "apple_m1_defconfig";
  extraMeta.platforms = [ "aarch64-linux" ];

  # Only the image m1n1 starts. nixos-apple-silicon also joins m1n1, every
  # model's device trees and U-Boot into one file for Asahi's installer,
  # which would rebuild U-Boot whenever the kernel changed, and which
  # nothing here uses.
  filesToInstall = [ "u-boot-nodtb.bin.gz" ];

  # The larger font, for the Mac's panel, and the menu the stick is picked
  # from.
  extraConfig = ''
    CONFIG_IDENT_STRING=" ${version}"
    CONFIG_VIDEO_FONT_4X6=n
    CONFIG_VIDEO_FONT_8X16=n
    CONFIG_VIDEO_FONT_SUN12X22=n
    CONFIG_VIDEO_FONT_16X32=y
    CONFIG_CMD_BOOTMENU=y
  '';

  # Compressed, so m1n1 knows where U-Boot ends and what follows it.
  postBuild = ''
    gzip -n u-boot-nodtb.bin
  '';
}
