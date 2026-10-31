# The disk of a machine Joel carries: a boot partition, then one encrypted
# partition holding Btrfs. The systemd initrd unlocks it with the TPM and a
# PIN, a YubiKey when the TPM refuses, or the recovery key. disko writes the
# layout when the machine is installed and declares its mounts from then on.
# The partitions and the open volume are found by the disk's name, so two
# disks attached at once, such as a machine's drive and a stick it was
# installed from, need names of their own. The cryptsetup typed by hand
# reads the disk's YubiKey and TPM tokens, as the boot does.
{ disko }:
{
  config,
  lib,
  pkgs,
  ...
}:
let
  cfg = config.jitsusama.disk;
  initrdSystemd = config.boot.initrd.systemd.package;

  # Where the initrd says, for each volume in its crypttab that a YubiKey
  # opens, that systemd-cryptsetup should leave its token plugins alone.
  keyed = "/run/cryptsetup-keyed";
  findKeyed = pkgs.writeShellScript "cryptsetup-find-keyed" ''
    while read -r name device _; do
      case "$name" in "" | "#"*) continue ;; esac
      slots=$(${initrdSystemd}/bin/systemd-cryptenroll "$device") || continue
      if [[ $slots =~ (^|[[:space:]])fido2([[:space:]]|$) ]]; then
        ${pkgs.coreutils}/bin/mkdir -p ${keyed}
        echo SYSTEMD_CRYPTSETUP_USE_TOKEN_MODULE=0 > "${keyed}/$name"
      fi
    done < /etc/crypttab
  '';

  # cryptsetup as typed by hand, given systemd's token plugins. Its own
  # build looks for them in a folder of its own that holds none, and
  # --external-tokens-path doesn't move it, so without them it passes over
  # every YubiKey and TPM token on the disk.
  cryptsetup = lib.hiPrio (
    pkgs.runCommand "cryptsetup-with-systemds-tokens"
      { nativeBuildInputs = [ pkgs.makeBinaryWrapper ]; }
      ''
        mkdir -p $out/bin
        makeBinaryWrapper ${lib.getExe' pkgs.cryptsetup "cryptsetup"} $out/bin/cryptsetup \
          --prefix LD_LIBRARY_PATH : ${config.systemd.package}/lib/cryptsetup
      ''
  );

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
    installKey = lib.mkOption {
      type = lib.types.nullOr lib.types.str;
      default = null;
      example = "/run/guide/install-key";
      description = ''
        A file the layout reads the encrypted volume's first key from when
        it formats the disk, rather than asking for a passphrase. The guide
        makes it in memory and takes its slot off the disk once the
        YubiKeys and the recovery key are enrolled. Only formatting reads
        it; the installed machine never does.
      '';
    };
  };

  # Btrfs checks a block's checksum only when something reads it, so damage
  # to a file nobody opens stays hidden until the day it's needed. A monthly
  # scrub reads every block, mends metadata from the second copy Btrfs keeps
  # of it, and names damaged data in the journal. It reads at idle priority,
  # so other work goes first, and a month missed while the machine was off
  # runs at the next boot.
  config.services.btrfs.autoScrub = {
    enable = true;
    interval = "monthly";
    fileSystems = [ "/" ];
  };

  # The TPM and the YubiKeys each have a PIN of their own, so each PIN must
  # reach only its own device. systemd's first pass through libcryptsetup's
  # token plugins asks once ("LUKS2 token PIN") and tries what's typed on
  # every token. When the TPM refuses a start with a YubiKey plugged in,
  # the TPM's PIN goes to the key as well, is asked for again as wrong, and
  # three tries block the key. Without the plugins, systemd asks "TPM2 PIN"
  # for the TPM alone, once, then "security token PIN" for the key alone,
  # through its own TPM2 and FIDO2 support, as it does when it's built
  # without them.
  #
  # That support takes fido2-device=auto to mean a YubiKey is enrolled, and
  # on a disk with none it stops rather than asking for the passphrase
  # (systemd 261's cryptsetup.c treats acquire_fido2_key_auto's ENXIO as
  # fatal, where the plugin falls back). So the plugins go only for a disk
  # whose header lists a YubiKey, read as the volume is opened. A disk with
  # none has no key for the TPM's PIN to reach, and keeps the plugins and
  # their passphrase prompt. Should the reading fail, nothing is written and
  # the disk opens as it would with the plugins.
  config.environment.systemPackages = [ cryptsetup ];

  config.boot.initrd.systemd.storePaths = [ findKeyed ];
  config.boot.initrd.systemd.services."systemd-cryptsetup@" = {
    overrideStrategy = "asDropin";
    serviceConfig = {
      ExecStartPre = [ "-${findKeyed}" ];
      EnvironmentFile = [ "-${keyed}/%I" ];
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
            # Encrypt in 4 KiB sectors, the size Btrfs writes, rather than the
            # 512 bytes a disk formatted for old systems reports, so each block
            # is one encryption instead of eight.
            extraFormatArgs = [
              "--sector-size"
              "4096"
            ];
            settings = {
              allowDiscards = true;
              # NVMe is fast enough that dm-crypt's queues only add latency.
              bypassWorkqueues = true;
              crypttabExtraOpts = [
                "tpm2-device=auto"
                "fido2-device=auto"
              ];
            };
            passwordFile = lib.mkIf (cfg.installKey != null) cfg.installKey;
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
