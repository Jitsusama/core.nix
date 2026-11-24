# The keys VM tests' checks, which checks.nix gathers with every other
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
  vm-commits-are-signed = import ./signing.nix { inherit self pkgs; };
  # A passkey is the TPM, uhid, udev's seat rules and the shell's askpass
  # together, and only a running machine has them all.
  vm-passkeys-come-from-the-tpm = import ./linux-id.nix { inherit self pkgs; };
  vm-keyring-opens-without-asking = import ./gnome-keyring.nix { inherit self pkgs; };
  # Only a key plugged into a machine shows which sign-ins take its touch.
  vm-a-yubikey-answers-only-once-in = import ./yubikey-touch.nix { inherit self pkgs; };
  # A password an agent asks for reaches its one command only on Joel's own
  # keys, through the launcher, the shell and the passwords daemon, an area
  # of the desktop's one machine (tests/desktop/desktop.nix).
  vm-desktop-answers-secret-asks = import ../../desktop/desktop.nix {
    inherit self pkgs;
    area = "answers-secret-asks";
  };
}
