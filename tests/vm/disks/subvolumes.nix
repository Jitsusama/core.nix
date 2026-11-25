# Boots a machine whose /home is Btrfs, as a workstation's is, and checks an
# account's folders are moved into subvolumes of their own as its manager
# starts: one moves, its files all there and the folder as it was kept;
# one a process holds open stays, and one it only maps; one whose writer runs, holding nothing
# open, stays, as at a switch; and one that can't be copied is left
# whole, its unit failing, with the writer it comes before started all the
# same. Each turns on Btrfs's own answers, a subvolume's root, a reflink
# across subvolumes and an account removing one it emptied, so only a
# machine with Btrfs shows them. The test is subvolumes-vm-test.py, which
# pytest runs inside the driver (tests/lib/vm_pytest.py).
{ self, pkgs }:
import ../../lib/pytest-vm-test.nix { inherit pkgs; } {
  name = "subvolumes";

  nodes.machine = {
    imports = [ self.nixosModules.home-manager ];
    users.users.joel = {
      isNormalUser = true;
      uid = 1000;
    };
    # Activation makes the home before the disk mounts over /home, so it's
    # made again on the disk, ahead of home-manager.
    systemd.tmpfiles.rules = [ "d /home/joel 0700 joel users -" ];
    home-manager.users.joel = {
      imports = [ self.homeModules.subvolumes ];
      home.stateVersion = "26.05";
      jitsusama.subvolumes.folders = {
        moved.seconds = 30;
        held = { };
        mapped = { };
        unreadable.before = [ "writer.service" ];
        missing = { };
        busy.before = [ "busy.service" ];
      };
      # Stands in for a program that writes in its folder, as the node
      # does in its state, started by the account's manager.
      systemd.user.services.writer = {
        Service.ExecStart = "${pkgs.coreutils}/bin/sleep infinity";
        Install.WantedBy = [ "default.target" ];
      };
      # Stands in for a writer running between writes, as a switch finds
      # the node, with nothing in its folder open.
      systemd.user.services.busy = {
        Service.ExecStart = "${pkgs.coreutils}/bin/sleep infinity";
        Install.WantedBy = [ "default.target" ];
      };
    };
    virtualisation.emptyDiskImages = [ 1024 ];
    virtualisation.fileSystems."/home" = {
      device = "/dev/vdb";
      fsType = "btrfs";
      autoFormat = true;
    };
  };

  tests = ./subvolumes-vm-test.py;
}
