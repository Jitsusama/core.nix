# The Wi-Fi VM Test's Areas

The Wi-Fi test joins networks in a virtual machine the way a laptop meets them,
with wifid, networkd and wpa_supplicant, as [`../wifi.nix`][wifi] says in its
opening comment. Each area is a check of its own on a machine of its own,
`vm-wifi-<area>`, so what one area sets up the others never see. Within an area
the tests run in order, each on the machine as the one before left it, so one
run alone may fail. Each file's opening docstring says its area in full.

- [`wifi-joins-vm-test.py`][joins]: the card joining the networks it's given,
  wifid telling the shell what it finds, and a network the launcher saved chosen
  first.
- [`wifi-joins-when-asked-vm-test.py`][asked]: the shell having wifid join,
  leave, sign in and save, the radios going off and on, and a card stuck or
  unanswered moving on to another network.
- [`wifi-moves-vm-test.py`][moves]: the card moving between the access points
  of one network, keeping its neighbours, giving way at once when a call's
  echoes are lost, and back to the one it left when the one it was asked to
  has gone.
- [`wifi-holds-on-vm-test.py`][holds]: nothing that should leave the card on its
  network taking it off, and what's forgotten going from everywhere.
- [`wifi_session.py`][session]: wifid on the test's machine, as the shell and
  the test reach it, which every area imports.

[wifi]: ../wifi.nix
[joins]: wifi-joins-vm-test.py
[asked]: wifi-joins-when-asked-vm-test.py
[holds]: wifi-holds-on-vm-test.py
[moves]: wifi-moves-vm-test.py
[session]: wifi_session.py
