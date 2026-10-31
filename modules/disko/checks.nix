# The checks of disko, which checks.nix gathers with every other folder's;
# its opening comment says what each kind of check proves.
{
  bareNixos,
  holds,
  pkgs,
  self,
  ...
}:
{
  # The cryptsetup a person types reads a disk's YubiKey tokens through
  # systemd's plugin: its dump of one says what the key asks for, which
  # cryptsetup alone can't. The plugin writes its part to stderr.
  cryptsetup-finds-systemds-plugins =
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
      token = builtins.toJSON {
        type = "systemd-fido2";
        keyslots = [ "0" ];
        fido2-credential = "AAAA";
        fido2-salt = "AAAA";
        fido2-rp = "io.systemd.cryptsetup";
        fido2-clientPin-required = true;
        fido2-up-required = true;
        fido2-uv-required = false;
      };
    in
    pkgs.runCommandLocal "cryptsetup-finds-systemds-plugins" { } ''
      cryptsetup=${machine.config.system.path}/bin/cryptsetup
      truncate -s 20M disk.img
      printf pw | $cryptsetup luksFormat -q --disable-locks --type luks2 --pbkdf pbkdf2 \
        --pbkdf-force-iterations 1000 --key-file - disk.img
      printf '%s' '${token}' | $cryptsetup token import --disable-locks --json-file - disk.img
      $cryptsetup luksDump --disable-locks disk.img > dump 2>&1
      grep -q 'fido2-clientPin-required' dump || {
        echo "cryptsetup on the machine's path read the YubiKey token without systemd's plugin:"
        sed -n '/^Tokens:/,/^Digests:/p' dump
        exit 1
      }
      touch $out
    '';

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
