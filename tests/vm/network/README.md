# The Network VM Tests

What a laptop makes of the networks it meets, on machines booted in QEMU:
joining Wi-Fi, setting its clock, finding its neighbours' names, bringing
its services back once the network comes, and keeping its connection quick
under load with the shaper. Each test's opening comment says what stands in
for the router, access point or server a VM lacks.

| File                                   | Holds                                                         |
| -------------------------------------- | ------------------------------------------------------------- |
| `wifi.nix`                             | joining Wi-Fi with wifid, one check per area of `wifi/`       |
| `wifi-check-page.py`                   | the pages the Wi-Fi networks are checked and signed in at     |
| `clock.nix`, `-vm-test.py`             | the clock set past a network's time server that never answers |
| `mdns.nix`, `-vm-test.py`              | machines finding each other by name, only on trusted networks |
| `name-service.nix`, `-vm-test.py`      | an IPv4 address resolved even when IPv6 answers first         |
| `network-revive.nix`, `-vm-test.py`    | an account's services started again once the network comes   |
| `network-revive-stand-in.sh`           | that test's stand-in service that needs the network           |
| `shaper.nix`, `-vm-test.py`            | which of CAKE's tins each kind of traffic reaches             |
| `shaper-rate.nix`, `-vm-test.py`       | a ping kept near its delay under a full upload                |
| `shaper-owned.nix`, `-vm-test.py`      | a queue a router hides, taken back by the shaper              |

- `wifi/`: the Wi-Fi test's areas, joining, joining when asked and holding
  on, and the session they share.
