# How a laptop hibernates: its memory written to the swap file as an image,
# and read back at the next start. Where the image went is kept by systemd in
# an EFI variable, so nothing here names a disk (tests/hibernate.nix proves
# the machine comes back).
{
  # The kernel compresses the image on at most three threads, so the
  # compressor sets how long going down and coming back take. lz4 compresses
  # a little faster than lzo, the kernel's default, and decompresses several
  # times faster, for a slightly larger image. The image's header says which wrote it, so a
  # resume reads it with the same one, whatever the starting kernel was told.
  boot.kernelParams = [ "hibernate.compressor=lz4" ];
  # The image is read in the initrd, so lz4 has to be there before the disk
  # opens. Where a kernel builds it in, as the laptops' does, this asks for
  # nothing more.
  boot.initrd.kernelModules = [ "lz4" ];
}
