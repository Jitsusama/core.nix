# A USB drive for the guide's VM tests to copy or pack home folders onto,
# as a person's often is: one exFAT partition in a GPT, made on its own and
# put in place at 1 MiB.
{ pkgs }:
pkgs.runCommand "usb-drive"
  {
    nativeBuildInputs = [
      pkgs.exfatprogs
      pkgs.gptfdisk
    ];
  }
  ''
    truncate -s 64M partition.raw
    mkfs.exfat partition.raw
    truncate -s 66M $out
    sgdisk --new=1:2048:133119 --typecode=1:0700 $out
    dd if=partition.raw of=$out bs=1M seek=1 conv=notrunc status=none
  ''
