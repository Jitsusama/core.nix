# A YubiKey's touch answers polkit only once the person is in
# (modules/yubikey/administers.nix): QEMU's canokey stands in for the key,
# plugged in and named. The person is signed in to a desktop at the seat,
# cage with a terminal in it standing in for niri, and run0 is asked from
# that terminal with the shell's lock let go, up, or saying nothing, from a
# text console signed in beside it, and from an ssh login. canokey says yes
# to every touch, so a prompt that takes the touch needs nothing typed. The
# tests are yubikey-touch-vm-test.py, which pytest runs inside the driver
# (tests/lib/vm_pytest.py).
{ self, pkgs }:
let
  inherit (import ./vm/guide/canokeys.nix { inherit pkgs; }) qemu;
in
import ./lib/pytest-vm-test.nix { inherit pkgs; } {
  name = "yubikey-touch";
  # The machine's qemu, which checks.nix wraps to wait for its sockets.
  qemu.package = qemu;

  nodes.machine =
    { lib, ... }:
    {
      imports = [ self.nixosModules.yubikey ];

      virtualisation.qemu.options = [
        # cage draws through virtio's GPU, not QEMU's standard VGA.
        "-vga none"
        "-device virtio-gpu-pci"
        # The key's state is made as QEMU starts, in the machine's own
        # temporary folder, since a key plugged in later can't make one.
        "-device qemu-xhci,id=keys"
        "-device canokey,bus=keys.0,file=$TMPDIR/canokey"
      ];

      users.users.someone = {
        isNormalUser = true;
        extraGroups = [ "wheel" ];
        # Typed to polkit's sign-in and the console's login, standing in
        # for the person's own.
        initialPassword = "typed";
      };
      # Signed in to a desktop at the seat, as the person is to niri.
      services.cage = {
        enable = true;
        user = "someone";
        program = lib.getExe pkgs.foot;
      };
      fonts.packages = [ pkgs.dejavu_fonts ];
      services.openssh.enable = true;
      # What asks run0's question, as every desktop has it. A sign-in it
      # would keep a while for the session is asked afresh each time, so
      # every run0 here shows what that one prompt takes.
      security.polkit = {
        enable = true;
        extraConfig = ''
          polkit.addRule(function (action, subject) {
            if (action.id == "org.freedesktop.systemd1.manage-units") {
              return polkit.Result.AUTH_ADMIN;
            }
          });
        '';
      };

      # A key is named so polkit asks for one, but canokey makes its
      # credential only once it's running, so the tests enrol it into a
      # file of their own.
      jitsusama.yubikey.administers.someone = [ "aGFuZGxl,cHVibGlj,es256,+presence" ];
      security.pam.u2f.settings.authfile = lib.mkForce "/run/u2f-mappings";
    };

  tests = ./yubikey-touch-vm-test.py;
}
