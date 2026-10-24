# The checks of disko, which checks.nix gathers with every other folder's;
# its opening comment says what each kind of check proves.
{
  bareNixos,
  holds,
  self,
  ...
}:
{
  # A disk's partitions and its open volume go by its name, so a machine's
  # drive and a stick attached beside it are never taken for each other.
  disks-go-by-their-name =
    let
      machine = bareNixos [
        self.nixosModules.disko
        {
          jitsusama.disk = {
            name = "stick";
            device = "/dev/disk/by-id/usb-example";
            swapSize = "4G";
          };
        }
      ];
      inherit (machine.config.disko.devices.disk.main.content.partitions) boot system;
    in
    holds "disks-go-by-their-name" "A disk's partitions or volume don't carry its name." (
      boot.label == "stick-boot"
      && system.label == "stick"
      && machine.config.boot.initrd.luks.devices ? stick
    );

  # Btrfs finds a damaged block only when something reads it, so the disk is
  # scrubbed on a schedule, whole, from its root.
  disks-are-scrubbed =
    let
      machine = bareNixos [
        self.nixosModules.disko
        {
          jitsusama.disk = {
            device = "/dev/disk/by-id/nvme-example";
            swapSize = "4G";
          };
        }
      ];
      inherit (machine.config.services.btrfs) autoScrub;
    in
    holds "disks-are-scrubbed" "The disk's Btrfs is never scrubbed, so damage stays hidden." (
      autoScrub.enable && autoScrub.fileSystems == [ "/" ]
    );
}
