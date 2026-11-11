# btvirt, BlueZ's emulator of Bluetooth controllers, for the VM tests that
# need a radio: as a server two machines' serial ports reach
# (vm/boot/bluetooth-disk-prompt.nix), or as controllers of the machine's own
# through the kernel's vhci (desktop.nix's looks area).
#
# btvirt is built only with BlueZ's tests, which nixpkgs leaves out. Its
# server reads every client into one buffer and loses the start of a
# header a read ends in, so two machines on serial ports, which pass a
# few bytes at a time, each got replies to the other's commands; the
# patch gives each client its own buffer, with a header's start kept at
# its front.
{ pkgs }:
pkgs.bluez.overrideAttrs (old: {
  patches = (old.patches or [ ]) ++ [ ./btvirt-keeps-each-clients-bytes.patch ];
  configureFlags = old.configureFlags ++ [ "--enable-testing" ];
  doCheck = false;
  postInstall = old.postInstall + ''
    install -Dm755 emulator/btvirt $out/bin/btvirt
  '';
})
