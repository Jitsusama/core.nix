# Installs core's disk layout and Secure Boot onto a virtual machine with a
# TPM, then walks docs/installing.md the way a laptop does: lanzaboote makes
# and enrols the keys on the first boot, the disk is bound to the TPM's
# measured-boot policy with a PIN, a recovery key is added and the install
# passphrase wiped, and the disk opens with the PIN alone. A changed PCR makes
# the TPM refuse. The TPM keeps no NvPCRs, so no boot or login waits on them.
# Each generation is counted as it boots and blessed once it's up, and one
# whose tries are spent is passed over for the last good one, the TPM still
# opening the disk, with a word to the session. The machine is
# secure-boot-machine.nix's, and the tests are secure-boot-vm-test.py, run
# through pytest.
{ self, pkgs }:
let
  passphrase = "install-passphrase";
  pin = "2468";
  notices = pkgs.writeShellApplication {
    name = "stand-in-notices";
    text = ''
      exec ${
        pkgs.python3.withPackages (python: [ python.dbus-fast ])
      }/bin/python3 ${../../stand-in-notices.py} "$@"
    '';
  };
in
import ../../lib/pytest-vm-test.nix { inherit pkgs; } {
  name = "secure-boot";
  globalTimeout = 40 * 60;

  nodes.machine = import ./secure-boot-machine.nix {
    inherit self pkgs passphrase;
    packages = [ notices ];
  };

  # The driver runs on the host, so what the tests need from Nix, the image
  # to boot, the tool to put an overlay on it, the systems to install and the
  # secrets to type, reaches them as their values fixture rather than on the
  # machine's path.
  tests = ./secure-boot-vm-test.py;
  values =
    { nodes, ... }:
    let
      inherit (nodes.machine.system.build)
        diskoImages
        later
        optionRomCheck
        toplevel
        ;
      values = pkgs.writeText "secure-boot-values.json" (
        builtins.toJSON {
          qemuImg = "${nodes.machine.virtualisation.qemu.package}/bin/qemu-img";
          image = "${diskoImages}/main.qcow2";
          optionRomCheck = "${optionRomCheck}/bin/option-rom-check";
          real = "${toplevel}";
          later = "${later}";
          inherit passphrase pin;
        }
      );
    in
    values;
}
