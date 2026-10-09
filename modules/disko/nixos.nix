# The disk of a machine Joel carries: a boot partition, then one encrypted
# partition holding Btrfs. The systemd initrd unlocks it with the TPM and a
# PIN, a YubiKey when the TPM refuses, or the recovery key. disko writes the
# layout when the machine is installed and declares its mounts from then on.
# The partitions and the open volume are found by the disk's name, so two
# disks attached at once, such as a machine's drive and a stick it was
# installed from, need names of their own.
{ disko }:
{ config, lib, ... }:
let
  cfg = config.jitsusama.disk;

  # Light compression, no access-time writes, and discards in the background
  # rather than on every delete.
  btrfs = [
    "compress=zstd:1"
    "noatime"
    "discard=async"
  ];
in
{
  imports = [ disko.nixosModules.disko ];

  options.jitsusama.disk = {
    name = lib.mkOption {
      # Short enough that "<name>-boot" fits a GPT partition's 36 characters.
      type = lib.types.strMatching "[a-z][a-z0-9-]{0,30}";
      default = "system";
      example = "stick";
      description = ''
        What the disk's partitions and encrypted volume are called: the labels
        <name>-boot and <name>, and /dev/mapper/<name> once it's open.
      '';
    };
    device = lib.mkOption {
      type = lib.types.str;
      example = "/dev/disk/by-id/nvme-SK_hynix_PVC10_SK_hynix_1TB_AJE7N523610207C3V";
      description = "The disk to install onto, by a name that doesn't change between boots.";
    };
    swapSize = lib.mkOption {
      type = lib.types.str;
      example = "64G";
      description = "The size of the swap file, at least the memory's, so the machine can hibernate.";
    };
  };

  config.disko.devices.disk.main = {
    type = "disk";
    inherit (cfg) device;
    content = {
      type = "gpt";
      partitions = {
        boot = {
          label = "${cfg.name}-boot";
          size = "1G";
          type = "EF00";
          content = {
            type = "filesystem";
            format = "vfat";
            mountpoint = "/boot";
            mountOptions = [ "umask=0077" ];
          };
        };
        system = {
          label = cfg.name;
          size = "100%";
          content = {
            type = "luks";
            inherit (cfg) name;
            settings = {
              allowDiscards = true;
              # NVMe is fast enough that dm-crypt's queues only add latency.
              bypassWorkqueues = true;
              crypttabExtraOpts = [
                "tpm2-device=auto"
                "fido2-device=auto"
              ];
            };
            content = {
              type = "btrfs";
              extraArgs = [ "-f" ];
              subvolumes = {
                "@" = {
                  mountpoint = "/";
                  mountOptions = btrfs;
                };
                "@home" = {
                  mountpoint = "/home";
                  mountOptions = btrfs;
                };
                "@nix" = {
                  mountpoint = "/nix";
                  mountOptions = btrfs;
                };
                "@log" = {
                  mountpoint = "/var/log";
                  mountOptions = btrfs;
                };
                # A swap file can't be compressed or snapshotted, so it gets a
                # subvolume of its own.
                "@swap" = {
                  mountpoint = "/swap";
                  swap.swapfile.size = cfg.swapSize;
                };
              };
            };
          };
        };
      };
    };
  };
}
