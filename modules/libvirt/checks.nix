# The checks of libvirt, which checks.nix gathers with every other folder's;
# its opening comment says what each kind of check proves.
{
  bareNixos,
  holds,
  lib,
  self,
  ...
}:
{
  # greetd waits for the GPU the hardware names rather than for the boot to go
  # idle, and the card gets the device unit it waits on. Without the udev tag
  # the unit never appears, and the login waits out the device timeout.
  # Session machines join virbr0 without a password, which needs the bridge
  # helper to allow it and the default network to be up from boot.
  session-machines-have-a-bridge =
    let
      machine = bareNixos [ self.nixosModules.libvirt ];
      inherit (machine.config) systemd virtualisation;
      network = systemd.services.libvirt-default-network;
    in
    holds "session-machines-have-a-bridge"
      "Session machines can't join virbr0: the bridge helper doesn't allow it, or nothing starts libvirt's default network at boot."
      (
        lib.elem "virbr0" virtualisation.libvirtd.allowedBridges
        && lib.elem "multi-user.target" network.wantedBy
        && lib.hasInfix "net-start default" network.script
      );
}
