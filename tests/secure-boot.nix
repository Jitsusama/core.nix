# Installs core's disk layout and Secure Boot onto a virtual machine with a
# TPM, then walks docs/installing.md the way a laptop does: lanzaboote makes
# and enrols the keys on the first boot, the disk is bound to the TPM's
# measured-boot policy with a PIN, a recovery key is added and the install
# passphrase wiped, and the disk opens with the PIN alone. A changed PCR makes
# the TPM refuse. The TPM keeps no NvPCRs, so no boot or login waits on them.
# Each generation is counted as it boots and blessed once it's up, and one
# whose tries are spent is passed over for the last good one, the TPM still
# opening the disk, with a word to the session.
{ self, pkgs }:
let
  passphrase = "install-passphrase";
  pin = "2468";
  notices = pkgs.writeShellApplication {
    name = "stand-in-notices";
    text = ''
      exec ${
        pkgs.python3.withPackages (python: [ python.dbus-fast ])
      }/bin/python3 ${./stand-in-notices.py} "$@"
    '';
  };
in
pkgs.testers.runNixOSTest {
  name = "secure-boot";
  globalTimeout = 40 * 60;

  nodes.machine =
    {
      config,
      extendModules,
      lib,
      ...
    }:
    {
      imports = [
        self.nixosModules.disko
        self.nixosModules.lanzaboote
      ];

      jitsusama.disk = {
        device = "/dev/vda";
        swapSize = "64M";
      };
      # OVMF runs the option ROM of QEMU's network card, so the keys carry its
      # checksum, as a machine with a ROM of its own sets them to.
      boot.lanzaboote.autoEnrollKeys.includeChecksumsFromTPM = true;
      # The machine runs the check it would get without that, to see it
      # refuse.
      system.extraDependencies = [ config.system.build.optionRomCheck ];
      # Only the image builder reads it, to format the disk unattended.
      disko.devices.disk.main.content.partitions.system.content.passwordFile = toString (
        pkgs.writeText "passphrase" passphrase
      );
      disko.devices.disk.main.imageSize = "4G";
      # qcow2 keeps only what the install wrote, where a raw image would put
      # four sparse gigabytes in the store.
      disko.imageBuilder.imageFormat = "qcow2";
      disko.memSize = 4096;
      # The image builder's VM has no TPM, which measured boot needs to predict
      # its policy, so the image holds the system without it and carries the
      # real one for the test to switch to. A real install runs on the laptop,
      # which has one.
      disko.imageBuilder.extraConfig = {
        boot.lanzaboote.measuredBoot.enable = lib.mkForce false;
        system.extraDependencies = [
          config.system.build.toplevel
          config.system.build.later
        ];
      };
      # A later generation to install, which differs only in what it says.
      system.build.later =
        (extendModules {
          modules = [ { environment.etc.generation.text = "later"; } ];
        }).config.system.build.toplevel;
      # disko's image builder hands vmTools its module tree as the kernel,
      # which current nixpkgs refuses (disko#1298); its new name is
      # kernelModules, beside the real kernel.
      disko.imageBuilder.pkgs = pkgs.extend (
        _final: prev: {
          vmTools = prev.vmTools // {
            override =
              args:
              prev.vmTools.override (
                removeAttrs args [ "kernel" ]
                // {
                  kernelModules = args.kernel;
                  kernel = config.disko.imageBuilder.kernelPackages.kernel;
                }
              );
          };
        }
      );

      environment.systemPackages = [
        notices
        pkgs.tpm2-tools
        pkgs.jq
        pkgs.cryptsetup
      ];

      virtualisation = {
        directBoot.enable = false;
        mountHostNixStore = false;
        useDefaultFilesystems = false;
        fileSystems = lib.mkForce { };
        useEFIBoot = true;
        tpm.enable = true;
        # Typed keys go to the PS/2 keyboard, which the initrd always has.
        qemu.virtioKeyboard = false;
        memorySize = 2048;
      };
      networking.hostName = "machine";
      # A user whose first login logind would measure into the login NvPCR.
      users.users.alice = {
        isNormalUser = true;
        uid = 1000;
      };
      boot.loader.timeout = 0;
      # nixos-install puts the boot loader on through it; tests leave it out.
      system.switch.enable = true;
      system.stateVersion = "26.05";
    };

  testScript =
    { nodes, ... }:
    ''
      import json, os, subprocess, tempfile

      # The machine writes to an overlay, since the image is in the store.
      overlay = tempfile.NamedTemporaryFile(suffix=".qcow2")
      subprocess.run([
        "${nodes.machine.virtualisation.qemu.package}/bin/qemu-img", "create", "-f", "qcow2",
        "-b", "${nodes.machine.system.build.diskoImages}/main.qcow2", "-F", "qcow2", overlay.name,
      ], check=True)
      os.environ["NIX_DISK_IMAGE"] = overlay.name

      # Two password agents print each prompt, so an unlock waits for the disk
      # to open before anything looks for the next one.
      def unlock(secret):
          machine.wait_for_console_text("Please enter")
          # The agent that wrote the prompt to the serial console reads it
          # there, where tty1's may not be reading keys yet.
          machine.send_console(secret + "\n")
          machine.wait_for_console_text("Finished .*Cryptography Setup for system")

      machine.start(allow_reboot=True)

      with subtest("the first boot makes the keys and reboots to enrol them"):
          unlock("${passphrase}")
          # Every boot's shell is connected in turn, or a later connection
          # reads an earlier boot's.
          machine.connect()
          # lanzaboote reboots once its keys are on the ESP; systemd-boot then
          # enrols them and the firmware comes back with Secure Boot on.
          machine.wait_for_console_text("Exporting as auth files")
          machine.wait_for_console_text("Linux version")
          # The driver can't see a reboot the machine started itself.
          machine.connected = False
          unlock("${passphrase}")
          machine.connect()
          machine.wait_for_unit("multi-user.target")
          status = machine.succeed("bootctl status")
          print(status)
          t.assertIn("Secure Boot: enabled (user)", status)
          print(machine.succeed("sbctl verify"))

      with subtest("without checksums, a firmware option ROM stops enrolment"):
          check = "${nodes.machine.system.build.optionRomCheck}/bin/option-rom-check"
          code, output = machine.execute(f"{check} 2>&1")
          print(output)
          t.assertNotEqual(code, 0)
          t.assertIn("Found OptionROM", output)
          t.assertIn("includeChecksumsFromTPM", output)

      with subtest("the real system installs with measured boot"):
          real = "${nodes.machine.system.build.toplevel}"
          machine.succeed(f"nix-env -p /nix/var/nix/profiles/system --set {real}")
          print(machine.succeed(f"{real}/bin/switch-to-configuration boot 2>&1"))
          # Installed with its three tries to count.
          machine.succeed("ls /boot/EFI/Linux/nixos-generation-2-*+3.efi")
          machine.reboot()
          unlock("${passphrase}")
          machine.wait_for_unit("multi-user.target")
          t.assertIn("Secure Boot: enabled (user)", machine.succeed("bootctl status"))
          # The boot counts only once the services are up.
          machine.wait_for_unit("systemd-bless-boot.service")
          up, blessed = (
              int(machine.succeed(f"systemctl show -P {when} {unit}"))
              for when, unit in [
                  ("ActiveEnterTimestampMonotonic", "multi-user.target"),
                  ("ExecMainStartTimestampMonotonic", "systemd-bless-boot.service"),
              ]
          )
          t.assertGreater(blessed, up, "the boot was blessed before its services were up")

      with subtest("the measured-boot policy covers PCRs 0, 4 and 7"):
          machine.wait_for_unit("systemd-pcrlock-make-policy.service")
          policy = json.loads(machine.succeed("cat /var/lib/systemd/pcrlock.json"))
          pcrs = sorted(v["pcr"] for v in policy["pcrValues"])
          t.assertEqual(pcrs, [0, 4, 7])

      disk = "/dev/disk/by-partlabel/system"
      with subtest("the runbook binds the disk to the TPM and a PIN"):
          machine.succeed(
              f"PASSWORD=${passphrase} NEWPIN=${pin} systemd-cryptenroll --tpm2-device=auto"
              f" --tpm2-with-pin=yes --tpm2-pcrlock=/var/lib/systemd/pcrlock.json {disk}"
          )
          recovery = machine.succeed(
              f"PASSWORD=${passphrase} systemd-cryptenroll --recovery-key {disk} 2>/dev/null"
          ).strip().split()[-1]
          machine.succeed(f"PASSWORD=${passphrase} systemd-cryptenroll --wipe-slot=password {disk}")
          tokens = machine.succeed(f"cryptsetup luksDump --dump-json-metadata {disk}")
          kinds = sorted(t["type"] for t in json.loads(tokens)["tokens"].values())
          print(kinds)
          t.assertEqual(kinds, ["systemd-recovery", "systemd-tpm2"])
          machine.fail(f"echo -n ${passphrase} | cryptsetup open --test-passphrase {disk}")
          machine.succeed(f"echo -n {recovery} | cryptsetup open --test-passphrase {disk}")

      with subtest("the disk opens with the TPM and the PIN alone"):
          machine.reboot()
          unlock("${pin}")
          machine.wait_for_unit("multi-user.target")
          print(machine.succeed("journalctl -b -o cat -u systemd-cryptsetup@system.service"))

      with subtest("no NvPCR is set up, and a login measures nothing"):
          # By its name rather than its unit: it exits before journald can
          # tell which unit its lines came from.
          setup = machine.succeed("journalctl -b -o cat -t systemd-tpm2-setup")
          print(setup)
          t.assertIn("SRK already stored in the TPM.", setup)
          t.assertNotIn("NvPCR", setup)
          machine.fail("ls /boot/loader/credentials/nvpcr-anchor.*.cred")
          machine.fail("test -s /run/systemd/nvpcr/nvpcr-anchor.cred")
          # Lingering has logind start the user as a first login does.
          machine.succeed("loginctl enable-linger alice")
          machine.wait_for_unit("user@1000.service")
          state = machine.succeed("systemctl show -P LoadState systemd-pcrlogin@1000.service")
          t.assertEqual(state.strip(), "masked")
          logind = machine.succeed("journalctl -b -o cat -u systemd-logind.service")
          print(logind)
          t.assertNotIn("measurement", logind)

      def entries():
          return json.loads(machine.succeed("bootctl list --json=short"))

      with subtest("a generation counted from its install is blessed once it's up"):
          booted = [e for e in entries() if e.get("isSelected")]
          print(booted)
          t.assertEqual(len(booted), 1)
          t.assertIn("nixos-generation-2-", booted[0]["id"])
          t.assertNotIn("triesLeft", booted[0], "the booted generation is still being counted")
          machine.wait_for_unit("boot-fell-back.service")
          machine.fail("test -e /run/boot-fell-back")

      with subtest("a generation that never boots is passed over, and the session is told"):
          later = "${nodes.machine.system.build.later}"
          machine.succeed(f"nix-env -p /nix/var/nix/profiles/system --set {later}")
          machine.succeed(f"{later}/bin/switch-to-configuration boot")
          # Its three tries spent, as three boots that never came up leave it.
          machine.succeed(
              "cd /boot/EFI/Linux && f=$(ls nixos-generation-3-*+3.efi) && mv $f ''${f%+3.efi}+0-3.efi"
          )
          machine.reboot()
          unlock("${pin}")
          machine.wait_for_unit("multi-user.target")
          t.assertEqual(machine.succeed("readlink -f /run/booted-system").strip(), real)
          machine.wait_for_unit("boot-fell-back.service")
          said = machine.succeed("cat /run/boot-fell-back").strip()
          t.assertEqual(said, "Generation 3 failed to boot 3 times, so this is 2.")
          # The session hears the file and waits for something to show
          # notices, then tells it.
          machine.succeed("loginctl enable-linger alice")
          machine.wait_for_unit("user@1000.service")
          told = lambda state: machine.wait_until_succeeds(
              "test \"$(systemctl --user -M alice@ show -P ActiveState"
              f" boot-fell-back.service)\" = {state}",
              timeout=60,
          )
          told("activating")
          machine.succeed(
              "systemd-run --user -M alice@ --unit notices"
              " /run/current-system/sw/bin/stand-in-notices /home/alice/told"
          )
          told("active")
          t.assertEqual(
              machine.succeed("cat /home/alice/told"),
              f"Boot\nBooted the last good generation\n{said}\n",
          )

      # Last, since the PCR it changes stays changed until the next boot.
      with subtest("the TPM refuses once a measured PCR changes"):
          # A scratch volume bound to the same policy, since the system's own
          # is already open.
          machine.succeed(
              "truncate -s 32M /root/scratch.img && "
              "echo -n scratch | cryptsetup luksFormat --batch-mode /root/scratch.img - && "
              "PASSWORD=scratch NEWPIN=${pin} systemd-cryptenroll --tpm2-device=auto"
              " --tpm2-with-pin=yes --tpm2-pcrlock=/var/lib/systemd/pcrlock.json /root/scratch.img"
          )
          attach = (
              "PIN=${pin} systemd-cryptsetup attach scratch /root/scratch.img -"
              " tpm2-device=auto,headless=yes,password-echo=no"
          )
          machine.succeed(attach)
          machine.succeed("systemd-cryptsetup detach scratch")
          machine.succeed("tpm2_pcrextend 7:sha256=" + "00" * 32)
          machine.fail(attach)
    '';
}
