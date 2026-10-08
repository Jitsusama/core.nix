# Hardware

A piece of hardware is one machine model: what that model needs to run well,
and nothing a person chose. It lives in `hardware/<model>/nixos.nix`, named
after the model as its maker names it, and `flake.nix` exports it under the
same name. A machine imports its hardware beside its roles:

```nix
nixosConfigurations.optimus = nixpkgs.lib.nixosSystem {
  modules = [
    core.nixosModules.workstation
    core.nixosModules.graphical
    core.nixosModules.dell-xps-14-da14260
    ./machines/optimus
  ];
};
```

The hardware decides the kernel: it imports the kernel module and sets
`jitsusama.kernel.cpu`, so each model gets a kernel compiled for its own CPU.
[The kernel guide][1] explains that build. A patch only one model needs goes
in that model's `boot.kernelPatches`, under the same bar as the kernel's own
patches: a clear purpose, from the people who maintain the driver, and
accepted or on its way upstream.

What stays with the machine: its disks and file systems, its name, and
anything that's a preference rather than a need, such as a charge limit.

## Dell XPS 14 (DA14260)

[`hardware/dell-xps-14-da14260/nixos.nix`][2], for optimus. An Intel Core
Ultra X7 358H (Panther Lake H), Arc B390 graphics, an NPU, a 2880x1800 120 Hz
OLED touchscreen, four CS35L57 SoundWire amplifiers, an OV08X40 camera on the
IPU7 behind Intel's CVS bridge, and Intel's BE211 Wi-Fi 7.

| Part     | What the module does                                                   |
| -------- | ---------------------------------------------------------------------- |
| CPU      | the kernel compiled for `pantherlake`, microcode, thermald             |
| Kernel   | the CVS patch below                                                    |
| Boot     | NVMe, USB and Thunderbolt drivers in the initrd, so a dock can unlock  |
| Graphics | Mesa, with VA-API (iHD), oneVPL, OpenCL and Level Zero for the Arc GPU |
| NPU      | `intel_vpu` with Level Zero                                            |
| Audio    | the firmware Sound Open Firmware runs on the DSP                       |
| Sensors  | ambient light and presence through `iio-sensor-proxy`                  |
| Docks    | bolt, which authorizes Thunderbolt and USB4 devices                    |
| Firmware | fwupd, for the BIOS and firmware Dell publishes through LVFS           |
| Wi-Fi    | runs as Wi-Fi 6; see below                                             |

### The CVS Patch

Intel's CVS bridge sits between the camera and the IPU. Its 7.2 driver claims
a GPIO it only reads an interrupt from, and on this laptop that pin is the one
all four amplifiers read their speaker ID from. With the driver loaded, none
of the amplifiers bind and there's no sound card:

```text
cs35l56 sdw:0:2:01fa:3557:01:2: error -EBUSY: Failed to get spk-id-gpios
```

[Junjie Cao's fix][3], from Intel, takes the interrupt without claiming the
pin. The media maintainer accepted it in September 2026 and it's marked for
the stable series; Omarchy carries the same fix for this laptop. Drop the
patch once nixpkgs's kernel includes it: the build fails when a patch no
longer applies, which says when.

### Wi-Fi

The BE211's Wi-Fi 7 receive path drops a link to its slowest rate, so the
module turns Wi-Fi 7 off and the card runs as Wi-Fi 6. The workaround comes
from [Omarchy][4], which applies it to this laptop; remove it once a Wi-Fi 7
link holds its rate without it.

With the lid closed, Wi-Fi slows to a crawl even on a dock. Testing in
[Omarchy's report][5] points to the antennas in the closed lid rather than to
software, though nobody has proven it, and nothing here fixes it. On a dock,
Ethernet avoids it.

### What's Left

| Part             | Why it isn't here yet                                                 |
| ---------------- | --------------------------------------------------------------------- |
| Camera           | nixpkgs's IPU7 camera software predates its CVS support; see below    |
| Touchpad haptics | needs a package for Dell's haptics daemon                             |
| Speaker tuning   | Omarchy's equalizer for these speakers, through PipeWire              |
| Panel VRR        | the panel's range comes from a DisplayID block Linux doesn't read yet |

Each needs the laptop itself to prove, so each lands after the first boot.

The camera will build on two pieces of work. nixpkgs's `hardware.ipu7` runs
the camera through Intel's image processor, but its release of Intel's camera
software predates [the change][6] that routes the sensor through the CVS
bridge 7.2 puts in front of it. [nixos-hardware's profile for this laptop][7],
not yet merged, carries that change and the camera's tuning, and has the
speaker tuning too.

### Left Out on Purpose

- **`fred=on`.** Omarchy sets it, but 7.2 turns FRED on by default wherever
  the CPU has it.
- **`rtc_cmos.use_acpi_alarm=1`.** Omarchy sets it for suspend-then-hibernate,
  but the kernel already uses the ACPI alarm on any Intel machine with a BIOS
  from 2015 or later.
- **Omarchy's Panel Replay patches.** Upstream turns Panel Replay off on this
  panel because it lags; Omarchy's patches turn it back on with a workaround
  nobody upstream has taken.

[1]: kernel.md
[2]: ../hardware/dell-xps-14-da14260/nixos.nix
[3]: https://patchwork.linuxtv.org/project/linux-media/patch/20260913133017.624919-1-junjie.cao@intel.com/
[4]: https://github.com/omacom/omarchy/blob/75cb4f7195cfc064d05cd30623b3254c9541ba3c/install/hardware/intel/fix-wifi7-eht.sh
[5]: https://github.com/omacom/omarchy/issues/9922
[6]: https://github.com/intel/ipu7-camera-hal/commit/f167239b3ecf
[7]: https://github.com/NixOS/nixos-hardware/pull/1912
