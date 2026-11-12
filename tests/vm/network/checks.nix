# The network VM tests' checks, which checks.nix gathers with every other
# folder's; README.md beside this says what each test proves.
{
  self,
  onLinux,
  vmHostPkgs,
  ...
}:
let
  pkgs = vmHostPkgs;
in
onLinux {
  # Joining Wi-Fi is the kernel, wifid, wpa_supplicant and networkd
  # together, and what it leaves of other programs' routing only shows
  # once it has. Each area is a machine of its own, so they run side by
  # side (tests/vm/network/wifi.nix).
  vm-wifi-joins = import ./wifi.nix {
    inherit self pkgs;
    area = "joins";
  };
  vm-wifi-holds-on = import ./wifi.nix {
    inherit self pkgs;
    area = "holds-on";
  };
  vm-wifi-joins-when-asked = import ./wifi.nix {
    inherit self pkgs;
    area = "joins-when-asked";
  };
  # What an account's services make of the network coming late, changing
  # and networkd restarting shows only with both managers running.
  vm-network-revives-units = import ./network-revive.nix { inherit self pkgs; };
  # Whether chrony hears the network's server, and sets the clock past it
  # when it never answers, shows only on a network that names one.
  vm-clock-keeps-time = import ./clock.nix { inherit self pkgs; };
  # Whether mDNS gets through the firewall, and only on a trusted network,
  # shows only with machines on a network asking each other.
  vm-machines-find-each-other-at-home = import ./mdns.nix { inherit self pkgs; };
  # Whether the name service keeps a family's addresses that resolved left
  # out shows only with a neighbour whose other family answers first.
  vm-names-give-ipv4-when-ipv6-answers-first = import ./name-service.nix {
    inherit self pkgs;
  };
  # Which of CAKE's tins a slice's traffic reaches shows only with the
  # account's manager making the slices and packets crossing a real link.
  vm-the-network-keeps-its-order = import ./shaper.nix { inherit self pkgs; };
  # Whether the shaper finds a connection's rate, and keeps a ping near its
  # own delay under a full upload, shows only with a real queue filling.
  vm-the-connection-stays-quick = import ./shaper-rate.nix { inherit self pkgs; };
  # Whether the shaper takes back a queue a router hides by sharing it flow
  # by flow shows only with real TCP filling a real fq_codel.
  vm-a-hidden-queue-is-taken-back = import ./shaper-owned.nix { inherit self pkgs; };
  # Whether the shaper's queues drop a call's packets while the connection
  # is full both ways shows only with real flows filling real CAKEs.
  vm-a-call-keeps-its-packets = import ./shaper-call.nix { inherit self pkgs; };
}
