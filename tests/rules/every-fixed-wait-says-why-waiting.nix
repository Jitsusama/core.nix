# The test files holding fixed waits that neither poll nor say why before
# every-fixed-wait-says-why, each with how many, waiting while each says what
# passes while it waits or becomes a wait on its event. The check fails on a
# file holding more, and on one holding fewer until its count here is lowered,
# so the list only shrinks.
{
  "modules/disk-guard/tests/guard/collecting.rs" = 1;
  "modules/disk-guard/tests/guard/commands.rs" = 2;
  "modules/disk-guard/tests/guard/dedupe.rs" = 1;
  "modules/disk-guard/tests/guard/host.rs" = 1;
  "modules/disk-guard/tests/guard/linux.rs" = 1;
  "modules/disk-guard/tests/guard/pace.rs" = 2;
  "modules/disk-guard/tests/guard/serve.rs" = 1;
  "modules/guide/tests/stick.rs" = 1;
  "modules/machine/tests/daemon.rs" = 1;
  "modules/mpv/keys-test.sh" = 2;
  "modules/nix-offload/offload/tests/there.rs" = 1;
  "modules/people/tests/daemon.rs" = 2;
  "modules/pi-packages/tests/inuse.rs" = 1;
  "modules/swayimg/tests/swayimg-moves-by-its-keys.sh" = 2;
  "modules/tuple/call/tests/daemon.rs" = 1;
  "modules/update-to-main/tests/update/main.rs" = 1;
  "tests/desktop/desktop-keeps-pace-vm-test.py" = 1;
  "tests/desktop/desktop-launcher-waits-on-joel-vm-test.py" = 1;
  "tests/desktop/desktop-shows-then-rests-vm-test.py" = 1;
  "tests/desktop/desktop_session.py" = 1;
  "tests/launcher_on_niri.py" = 1;
  "tests/lib/qemu-after-its-sockets-test.py" = 1;
  "tests/vm/boot/plymouth-vm-test.py" = 1;
  "tests/vm/guide/secure-boot-vm-test.py" = 1;
  "tests/vm/network/wifi/wifi-joins-when-asked-vm-test.py" = 1;
}
