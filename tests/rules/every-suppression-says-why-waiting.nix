# The files holding lint suppressions that don't say why before
# every-suppression-says-why, each with how many, waiting while each gains its
# reason. The check fails on a file holding more, and on one holding fewer
# until its count here is lowered, so the list only shrinks.
{
  "modules/ddcutil/tests/only-external-screens-reach-the-seat.sh" = 2;
  "modules/quickshell/Launcher.qml" = 1;
  "tests/agents-on-niri-test.py" = 1;
  "tests/vm/boot/bluetooth-keyboard-stand-in.py" = 6;
  "tests/desktop/desktop-launcher-answers-vm-test.py" = 4;
  "tests/desktop/desktop-launcher-finds-by-name-vm-test.py" = 11;
  "tests/desktop/desktop-launcher-finds-sessions-vm-test.py" = 6;
  "tests/desktop/desktop-launcher-goes-places-vm-test.py" = 2;
  "tests/desktop/desktop-launcher-holds-meetings-vm-test.py" = 2;
  "tests/desktop/desktop-launcher-reads-mail-vm-test.py" = 2;
  "tests/desktop/desktop-looks-machine-vm-test.py" = 2;
  "tests/desktop/desktop-opens-and-sets-vm-test.py" = 1;
  "tests/vm/boot/greeter-vm-test.py" = 2;
  "tests/launcher-on-niri-test.py" = 7;
  "tests/repository/every-vm-test-keeps-to-the-pyramid.py" = 1;
  "tools/run-subtests/run-subtests.py" = 5;
}
