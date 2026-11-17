# The Wi-Fi VM Test's Areas

The Wi-Fi test joins networks in a virtual machine the way a laptop meets
them, with wifid, networkd and wpa_supplicant, as [`../wifi.nix`][wifi] says
in its opening comment. Each area is a check of its own on a machine of its
own, `vm-wifi-<area>`, so what one area sets up the others never see.

- `wifi-joins-vm-test.py`: the card joining the networks it's given, wifid
  telling the shell what it finds, and a network the launcher saved chosen
  first.
- `wifi-joins-when-asked-vm-test.py`: the shell having wifid join, leave,
  sign in and save, the radios going off and on, and a card stuck or
  unanswered moving on.
- `wifi-holds-on-vm-test.py`: nothing that should leave the card on its
  network taking it off, and what's forgotten going from everywhere.
- `wifi_session.py`: wifid on the test's machine, as the shell and the test
  reach it, which every area imports.

[wifi]: ../wifi.nix
