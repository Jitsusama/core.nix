# Sets a machine's clock on a network that names a time server of its own
# that never answers, as a hotel's did. The machine's own servers are a second
# machine running chrony, which it reaches; it has to get there, and not keep
# asking the network's. Nothing names the network's server until networkd has
# it, so timesyncd starts once it does: started sooner, it would find no server
# named and fall back to the machine's, which proves nothing. The script is
# clock-vm-test.py.
{ self, pkgs }:
let
  silent = "192.168.1.99";
  # What the script needs from Nix, in one file whose path takes the place
  # of the script's @values@ marker.
  values = pkgs.writeText "clock-values.json" (builtins.toJSON { inherit silent; });
in
pkgs.testers.runNixOSTest {
  name = "clock";

  nodes.server = {
    services.chrony = {
      enable = true;
      servers = [ ];
      # Serves its own clock, to anyone on the test's network.
      extraConfig = ''
        allow
        local stratum 8
      '';
    };
    networking.firewall.allowedUDPPorts = [ 123 ];
  };

  nodes.machine =
    { lib, ... }:
    {
      imports = [ self.nixosModules.timesyncd ];
      networking.useNetworkd = true;
      networking.timeServers = [ "server" ];
      systemd.network.networks."40-eth1".ntp = [ silent ];
      # A virtual machine takes its time from KVM, so NixOS leaves timesyncd
      # off in one.
      services.timesyncd.enable = lib.mkForce true;
      systemd.services.systemd-timesyncd.wantedBy = lib.mkForce [ ];
    };

  testScript = builtins.replaceStrings [ "@values@" ] [ "${values}" ] (
    builtins.readFile ./clock-vm-test.py
  );
  # nix fmt holds the script to mypy --strict against the driver's own types,
  # through driver_names.pyi, which the driver's lighter check can't find.
  skipTypeCheck = true;
}
