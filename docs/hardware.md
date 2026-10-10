# Hardware

A piece of hardware is one machine model: what that model needs to run well,
and nothing a person chose. It lives in `hardware/<model>/nixos.nix`, named
after the model as its maker names it, and `flake.nix` exports it under the
same name. A machine imports its hardware beside its roles:

```nix
nixosConfigurations.laptop = nixpkgs.lib.nixosSystem {
  modules = [
    core.nixosModules.workstation
    core.nixosModules.graphical
    core.nixosModules.laptop
    core.nixosModules.dell-xps-14-da14260
    ./machines/laptop
  ];
};
```

The hardware decides the kernel: it imports the kernel module and sets
`jitsusama.kernel.cpu`, so each model gets a kernel compiled for its own CPU.
[The kernel guide][1] explains that build. A patch only one model needs goes
in that model's `boot.kernelPatches`, under the same bar as the kernel's own
patches: a clear purpose, from the people who maintain the driver, and
accepted or on its way upstream.

Hardware also says which keyboard is the machine's own, in
`jitsusama.keyboard.builtIn`, so the `laptop` role can give that keyboard
Colemak Mod-DH and leave every other keyboard as it is. The layout is a
choice, so it lives in the role; which keyboard is built in is a fact, so it
lives here.

What stays with the machine: which disk it installs onto, its name, and
anything that's a preference rather than a need, such as a charge limit. The
disk's layout is the same on every laptop, so it's a module, `disko`, and
[the install guide][8] says how a machine gets onto it.

## Dell XPS 14 (DA14260)

[`hardware/dell-xps-14-da14260/nixos.nix`][2]. An Intel Core Ultra X7 358H
(Panther Lake H), Arc B390 graphics, an NPU, a 2880x1800 120 Hz OLED
touchscreen, four CS35L57 SoundWire amplifiers, an OV08X40 camera on the IPU7
behind Intel's CVS bridge, and Intel's BE211 Wi-Fi 7.

| Part     | What the module does                                                   |
| -------- | ---------------------------------------------------------------------- |
| CPU      | the kernel compiled for `pantherlake`, microcode, thermald             |
| Kernel   | the CVS patch below                                                    |
| Keyboard | named as the built-in one: the embedded controller's PS/2 keyboard     |
| Boot     | NVMe, USB and Thunderbolt drivers in the initrd, so a dock can unlock  |
| Graphics | Mesa, with VA-API (iHD), oneVPL, OpenCL and Level Zero for the Arc GPU |
| NPU      | `intel_vpu` with Level Zero                                            |
| Audio    | the firmware Sound Open Firmware runs on the DSP                       |
| Speakers | Omarchy's tuning, as a smart filter; see below                         |
| Camera   | Intel's camera software, with the CVS bridge in its graph; see below   |
| Sensors  | ambient light through `iio-sensor-proxy`; see below                    |
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

### The Speaker Tuning

Linux loads Dell's firmware for the amplifiers but has nothing in place of the
Waves tuning Windows puts on top, so the speakers sound thin. The module runs
[Omarchy's tuning][9] for this model unchanged: thirteen biquads per channel
and a limiter, fitted to a measured EasyEffects profile, with no binary blob.
It lives in [`speakers.conf`][10], beside the module, and runs as a PipeWire
client of its own, so changing it restarts only the filter and never cuts off
the programs playing.

Omarchy attaches it as a sink of its own, which programs then have to play
to. Here it's a WirePlumber smart filter on the speakers instead: they stay
the device programs see, sound goes through the filter only on its way to
them, and headphones and displays get it untouched. Omarchy gave up on smart
filters because the filter seemed to pass sound through unchanged, but that's
how it measures, not what it does: WirePlumber hands a recording of the
speakers the filter's input instead. `vm-speakers-are-tuned` links its recorder
by hand and hears the tuning.

### The Camera

The OV08X40 sits behind the CVS bridge, which Linux 7.2 puts between the sensor
and the IPU. [`camera.nix`][11] turns on nixpkgs's `hardware.ipu7`, which brings
the image processor's module and firmware, and swaps in Intel's camera software
from its main branch, which knows the bridge. A patch routes this sensor's graph
through the bridge, as [Intel's own change][6] does for the next IPU. Programs
open a loopback camera, "Built-in Camera", which a relay fills from the image
processor at 1080p, turned upright: the sensor is mounted upside down.
PipeWire hides the raw capture nodes and the bare sensor libcamera offers,
which it would otherwise rank above the loopback as the default camera.

[nixos-hardware's profile for this laptop][7], not yet merged, worked out every
piece: the patch, the order the bridge's drivers load in, keeping USB from
suspending the bridge, hiding the raw capture nodes from PipeWire, and the
relay's buffers and queue, without which it measured a few frames a second.
[Omarchy's camera package][12] ships the same graph. The relay starts only when
the image processor appears, so a machine without the camera never runs it.

Omarchy also sharpens the picture and lowers its exposure. Whether that looks
better is for the laptop to show, as is the camera itself.

### The Mic-Mute Light

`dell-laptop` gives the light `audio-micmute` as its trigger, and `snd-ctl-led`
lights it for any switch attached to it. alsa-ucm-conf's cs42l45 profile
attaches `cs42l45 FU 113 Channel Switch`, the switch PipeWire's mute flips, but
only through a sysfs file root alone can write, so it happens only when
`alsactl` opens the profile as root at boot, as Arch's udev rule does. NixOS's
rule passes `-U`, which skips the profile, and runs only with ALSA persistence
on. The [module][2] has udev write that one attachment itself when the card
appears, with no program run. Omarchy's mute script sets the light instead,
which detaches the trigger after the first unmute.

### Nothing to Add

- **The touchpad.** It's a `2C2F` haptic pad on `hid-multitouch`. Omarchy
  installs Dell's haptics daemon only for Synaptics pads (`VEN_06CB`), and
  nothing for this one, and `hid-multitouch` registers no force feedback on
  it, so it behaves here as it does on Omarchy.
- **The Synaptics USB device** (`06cb:0701`) is the camera's USB-IO bridge,
  bound to `usbio-bridge`, not a fingerprint reader. This laptop has none.
- **Option ROMs for Secure Boot.** The firmware runs none, so its TPM event
  log holds no checksums and the keys go in alone, as lanzaboote enrols them
  unless a machine says otherwise.

### Measured, and Left as It Is

Each of these was measured on the laptop and gained nothing, so nothing
changes it:

- **The long-term power limit.** Raising it from 30 W to the 38 W the
  firmware allows ran no faster: Dell's skin-temperature policy holds
  sustained power near 25 W either way.
- **The performance profile on mains.** It sustains 22 to 28% more work than
  balanced, which is why the charger follower asks for it. Easing the energy
  preference under it to balance_performance kept the same speed, sustained
  and in bursts.
- **Wi-Fi power saving.** It stays on: off, the router's latency was the same
  within the noise.
- **Runtime power management of PCI devices.** Only the management engine's
  two idle interfaces and the bridge to the embedded controller would
  suspend, and the power saved was lost in the noise.
- **The disk's 4 KiB format.** The drive offers one, but changing it erases
  the drive. The `disko` module encrypts in 4 KiB sectors whatever the drive
  reports, which on a RAM disk doubled dm-crypt's write speed. The installed
  laptop keeps its 512-byte sectors: the drive writes slower than dm-crypt
  does at either size, so re-encrypting it in place would gain nothing.

### What's Left

| Part         | Why it isn't here yet                                                 |
| ------------ | --------------------------------------------------------------------- |
| Panel VRR    | the panel's range comes from a DisplayID block Linux doesn't read yet |
| Presence     | `iio-sensor-proxy` doesn't publish the presence sensor; see below     |
| Level Zero   | nixpkgs's GPU driver can't find its compiler and aborts; see below    |

- **Presence.** The sensor hub's attention sensor shows up as an IIO
  proximity device, but `iio-sensor-proxy` has no near level for it and
  publishes nothing, and nothing would read it yet.
- **Level Zero on the GPU.** nixpkgs gives only the OpenCL driver its
  compiler on its library path, so `libze_intel_gpu` can't load
  `libigc.so.2` and aborts in `zeInit`. OpenCL works, and so does the NPU
  through Level Zero. The fix is a line in nixpkgs's package.

### Left Out on Purpose

- **`fred=on`.** Omarchy sets it, but 7.2 turns FRED on by default wherever
  the CPU has it.
- **`rtc_cmos.use_acpi_alarm=1`.** Omarchy sets it for suspend-then-hibernate,
  but the kernel already uses the ACPI alarm on any Intel machine with a BIOS
  from 2015 or later.
- **Omarchy's Panel Replay patches.** Upstream turns Panel Replay off on this
  panel because it lags; Omarchy's patches turn it back on with a workaround
  nobody upstream has taken.
- **intel-lpmd.** Omarchy runs Intel's low-power mode daemon here. When the
  machine goes quiet, its profile for this CPU confines every task to the four
  low-power cores until a burst of work arrives, so the first keystroke after
  a pause lands on the slowest cores. Intel's Thread Director already steers
  light work to efficient cores without fencing anything in.
- **Dell's privacy driver (`DELL_WMI_PRIVACY`).** Omarchy's kernel has it, but
  on this laptop it reports the microphone, camera shutter and privacy screen
  all as unsupported. The mute key's light comes from `dell-laptop`, which is
  in.

[1]: kernel.md
[2]: ../hardware/dell-xps-14-da14260/nixos.nix
[3]: https://patchwork.linuxtv.org/project/linux-media/patch/20260913133017.624919-1-junjie.cao@intel.com/
[4]: https://github.com/omacom/omarchy/blob/75cb4f7195cfc064d05cd30623b3254c9541ba3c/install/hardware/intel/fix-wifi7-eht.sh
[5]: https://github.com/omacom/omarchy/issues/9922
[6]: https://github.com/intel/ipu7-camera-hal/commit/f167239b3ecf
[7]: https://github.com/NixOS/nixos-hardware/pull/1912
[8]: installing.md
[9]: https://github.com/omacom/omarchy/blob/a466dcc04f937a41c820aaa990a31f36ecaed543/default/audio/tunings/dell-xps-2026/filter-chain.conf
[10]: ../hardware/dell-xps-14-da14260/speakers.conf
[11]: ../hardware/dell-xps-14-da14260/camera.nix
[12]: https://github.com/TsaiGaggery/hurrican_omarchy_enabling
