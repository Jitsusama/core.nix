# Apple's restore image for its Macs with Apple silicon, the zip whole in a
# read-only erofs of its own, for a stick to carry as a partition beside its
# system (stick.nix), so `guide revive` can put macOS back on a Mac from any
# laptop. idevicerestore reads the image's filesystem in place when the zip
# stores it uncompressed, as Apple's do, so the partition needs no room
# beside it to unpack into.
#
# The image is Apple's own and public, so it's fetched by its hash and
# needs no seal. It's tens of gigabytes, so it's built where it's asked
# for rather than sent to a builder, and a machine that already holds a
# copy checked against Apple's hash adds it with `nix-store --add-fixed
# sha256` rather than fetching it again.
{
  lib,
  runCommand,
  fetchurl,
  erofs-utils,
  gnutar,
}:
{
  # Apple's address for it, which names the file: `UniversalMac_<version>_
  # <build>_Restore.ipsw`.
  url,
  # Its SHA-256, in SRI form.
  hash ? null,
  # The zip itself, which a test gives in place of Apple's.
  zip ? fetchurl {
    inherit url hash;
    preferLocalBuild = true;
  },
}:
let
  file = baseNameOf url;
in
runCommand "${lib.removeSuffix ".ipsw" file}.erofs"
  {
    nativeBuildInputs = [
      erofs-utils
      gnutar
    ];
    # Copying tens of gigabytes to a builder and back costs more than
    # building it here, which only copies.
    preferLocalBuild = true;
    allowSubstitutes = false;
    passthru = { inherit file zip; };
  }
  ''
    # Streamed through tar, so the zip is read once and never copied whole
    # into the build's own folder. The same zip always makes the same
    # image: no times, owners or random UUID of its own.
    tar --create --file=- --dereference \
      --owner=0 --group=0 --numeric-owner --mtime=@0 --mode=0444 \
      --transform=${lib.escapeShellArg "s|.*|${file}|"} \
      --directory=${dirOf "${zip}"} ${baseNameOf "${zip}"} |
      mkfs.erofs --tar=f -b4096 -T0 -U 00000000-0000-0000-0000-000000000000 \
        -L apple-restore "$out"
  ''
