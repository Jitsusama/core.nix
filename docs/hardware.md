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
| Battery  | charges to 80% and again below 75%, in the BIOS; see below             |
| BIOS     | the thermal mode and four other settings held where they were chosen   |
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

With the CVS bridge out of the way, the amplifiers still race each other for
the pin. They probe at once, and each requests it exclusively, reads it and
puts it back, so on one boot in six an amplifier found it taken and failed
with that same -EBUSY, and the card didn't register until the amplifier was
bound by hand. `cs35l56-spkid-lock.patch`, written here, makes them take turns
with a lock around the read; the patch says why.

### The Boot Clock

The firmware hands over to the boot loader at 400 MHz with HWP off, and holds
the CPU there for as long as it likes, ignoring the 1.9 GHz IA32_PERF_CTL
asks for: 130 ms into lanzaboote's stub on one boot, through the whole 3 s of
the loader on another, and through 26.9 s of the firmware's own on a third.
No thermal or power limit was in force. At 400 MHz the stub's SHA-256 of the
kernel and initrd runs at 84 TSC ticks a byte against 7.4 at full speed.

core's stub turns HWP on at entry, which Linux does at every boot anyway, and
records what it found in the volatile `StubHwp` variable: IA32_PM_ENABLE and
IA32_PERF_STATUS as handed over, and IA32_PERF_STATUS a millisecond after.
It can't shorten the firmware's own time.

### Wi-Fi

The BE211's Wi-Fi 7 receive path drops a link to its slowest rate, so the
module turns Wi-Fi 7 off and the card runs as Wi-Fi 6. The workaround comes
from [Omarchy][4], which applies it to this laptop; remove it once a Wi-Fi 7
link holds its rate without it. Turning Wi-Fi 7 back on lets the card offer
multi-link, and the kernel then answers nothing through the old wireless
extensions, which WARP's service still reads its Wi-Fi statistics through.

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

### The Charge Limit

The battery stops charging at 80% and starts again below 75%. A lithium cell
ages fastest held near full, and faster warm: Battery University puts a cell
kept a year at 25 °C at 96% of its capacity at 40% charge, against 80% at
full. With the starting point that close, the laptop leaves the charger near
80% however briefly it was plugged in; a start of 50 would leave one plugged
in at 70% uncharged. On mains the laptop runs from the charger either way, so
the limit costs no speed.

`dell-laptop` offers these as `charge_types` and the charge thresholds, but
with a BIOS password set every write fails with EIO: Dell's token writes need
a key made from the password, and the driver sends none. So the module sets
them through the BIOS's own settings ([firmware-settings][13]), which take
the admin password. The thresholds are read-only until the mode is Custom
(`ReadOnlyIfNot:PrimaryBattChargeCfg=Custom`), so the first pass writes the
mode and the second the thresholds. The battery reports the new limits at
once, though the BIOS also asks for a boot to finish taking them.
power-profiles-daemon leaves a battery in the custom mode alone.

To charge to full once, `run0 firmware-settings set CustomChargeStop 100`,
and `run0 firmware-settings apply` after to bring the limit back. Both ask
for the admin password.

The module holds five more settings that were chosen here, since a BIOS
update or reset can change them without a word, as 1.11.0 turned the
adapter warnings off. The thermal mode stays
Performance, which the power limits were measured in; SAGV stays on, so the
memory slows when idle, for the battery; the keyboard's light stays dim; the
adapter warnings stay off, so a charger the BIOS thinks too weak doesn't
hold the boot; and downgrades stay allowed, so a bad BIOS can be rolled back.
Secure Boot's settings stay out: recovering those is the enrolment's job.

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
  firmware allowed ran no faster: Dell's skin-temperature policy holds
  sustained power near 25 W either way. BIOS 1.11.0 allows 41 W, and the
  30 W limit holds as before.
- **The performance profile on mains.** It sustains 22 to 28% more work than
  balanced, which is why the charger follower asks for it.
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

### The Energy Preference

power-profiles-daemon's performance profile asks Intel's driver for the
energy preference `performance`. The module patches it to ask for
`balance_performance`, as balanced already does on mains; the profiles still
differ in the energy bias (0 against 6) and in the firmware's platform
profile. Measured on a quiet laptop under the performance profile, with
stress-ng's CPU stressor on sixteen threads for a minute:

| Preference          | Idle         | Sustained, ops/s | Load power   | Burst from idle   | Wakeup p99    |
| ------------------- | ------------ | ---------------- | ------------ | ----------------- | ------------- |
| performance         | 7.0 to 9.5 W | 23,720           | 34.4 W       | 18.57 to 18.61 ms | 83 to 118 µs  |
| balance_performance | 3.0 to 4.4 W | 25,859 to 26,219 | 38.3, 39.3 W | 18.55 to 18.70 ms | 77 to 164 µs  |

Idle power falls by a third or more, and the load runs 9 to 11% faster for
about the same work per joule. Bursts from idle (a fixed loop of about 19 ms,
after 300 ms asleep) and wakeups (lateness past a 1 ms timer) are the same
within the noise, over three interleaved runs each. An earlier comparison
found no difference in idle power because it read idle while another
session's builds kept the laptop 9% busy.

### The Two Crashes

The laptop has panicked twice, both times on BIOS 1.10.1 and kernel 7.2.9,
and pstore kept both records:

- **At boot, 20 s in.** An RCU callback freeing a dentry read the slab's
  record of which allocation tag the dentry was charged to, and found one in
  the modules' tag area whose counter pointer was garbage, so
  `__alloc_tagging_slab_free_hook()` faulted. It came 60 ms after the
  camera's ISYS bound the CVS bridge.
- **At reboot, after a suspend.** `device_shutdown()` found that the device
  before the one it was removing had been freed: the `kobj.entry` link at
  offset 8 of its 1 KiB slab object held an address in the modules' tag area
  instead. Offset 8 is where an entry in a slab's object vector keeps its
  tag, so the freed device's memory had become some slab's object vector.

Each is a use-after-free: something freed memory that was still in use. In
the second, the allocation profiler's bookkeeping was only the next owner of
that memory, writing where it was entitled to. Neither record names what
freed it. 7.2.10 and the memory list hold no fix for the object vectors that
would match, and there have been no crashes since, over five boots on BIOS
1.11.0. Both kernels carried the staging IPU7 drivers and Intel's
out-of-tree psys module, which the camera needs.

Two settings in [the kernel guide][1] answer what can be answered without a
culprit: the profiler is out of the build, and a panic reboots after ten
seconds instead of freezing the laptop. If it happens again, the record is in
`/var/lib/systemd/pstore`, and the next step is a kernel built with KASAN,
which names both the code that freed the memory and the code that used it.

### What's Left

| Part         | Why it isn't here yet                                                 |
| ------------ | --------------------------------------------------------------------- |
| Panel VRR    | the panel's range comes from a DisplayID block Linux doesn't read yet |
| Presence     | `iio-sensor-proxy` doesn't publish the presence sensor; see below     |

- **Presence.** The sensor hub's attention sensor shows up as an IIO
  proximity device, but `iio-sensor-proxy` has no near level for it and
  publishes nothing, and nothing would read it yet.

### Expected in the Journal

Each of these turns up at every boot or resume, has been traced to its cause,
and is harmless, so nothing silences or patches it:

- **dbus-broker's "Ignoring duplicate name", at error priority.** NixOS
  gives the bus `system-path`'s `share/dbus-1` beside each package's own, so
  every service file is found twice, once through a link. Leaving
  `system-path` out would lose the services only it has.
- **NetworkManager's "error setting IPv4 forwarding to '0'"** for
  `p2p-dev-wlp0s20f3`. The Wi-Fi P2P device has no kernel interface, so
  `nm_device_sysctl_ip_conf_set()` gives up without setting errno, and the
  warning ends with whatever errno was left: "Success" at one boot,
  "Resource temporarily unavailable" at the next. Forwarding stays off, as
  it should. NetworkManager
  1.58's fix is an early return at the top of `_dev_ipforwarding4_set()`:
  `if (nm_device_get_ip_ifindex(self) <= 0) return;`.
- **"Esys_NV_DefineSpace ErrorCode (0x0000014c)", four times.** That's
  `TPM_RC_NV_DEFINED`: systemd-tpm2-setup finds its NvPCR indexes already
  defined from an earlier boot and carries on.
- **"Anchor secret file ... different from current anchor secret,
  updating."** systemd-tpm2-setup leaves a copy of the NvPCR anchor on the
  ESP for the initrd, but runs before `/boot` is mounted, so finds no ESP and
  leaves none. The initrd then makes a new anchor at every boot, and the unit
  replaces `/var`'s copy. Nothing here seals to an NvPCR: the disk's policy
  is PCRs 0, 4 and 7. Ordering the unit after `/boot` would put its 0.7 s on
  the way to `sysinit.target`.
- **"SMBus is busy, can't use it!", seven times.** The i801 driver finds the
  bus busy as it probes. Nothing uses the bus: the LPDDR5's SPD isn't
  registered while its write protection is on.
- **No "RAPL domain uncore" on BIOS 1.11.0.** 1.10.1 reported it. It only
  meters the uncore's energy; the power limits are unaffected.
- **"UCSI_GET_PDOS failed (-95)".** The firmware's USB-C interface doesn't
  support the command.
- **xe's "Selective fetch area calculation failed in pipe A"**, once. When
  the panel's self refresh can't work out which part of a frame changed, the
  driver sends the whole frame instead, and says so the first time. The
  driver marks the case as one still to optimise.
- **"no PCM in topology for HDMI converter 3"** from the sound card. The
  HDMI codec has four converters and the DSP's topology three outputs. The
  GPU drives four displays, the panel being one, so the three cover every
  external display.
- **Bluetooth's "Received gp1 mailbox interrupt" and "Failed to send
  Intel_Write_DDC (-95)".** The controller doesn't take the DDC parameters,
  and works without them.
- **"Returned feature report did not match the request"** from the
  touchpad, which answers one feature request with another report and works
  as it should.
- **"Defer on undeferrable control"**, three times, from the headset
  codec's SDCA function. The codec answers busy while its firmware loads, and
  the driver polls until it's ready; Dell's tables don't mark those controls
  as ones that may answer busy, which is all the message says. The jack's
  controls all appear.
- **"ASoC: Parent card not yet available, widget card binding deferred"**,
  four times. The DSP's topology loads before the card registers, and its
  widgets bind once it does.
- **igen6_edac's "resource sanity check ... spans more than PNP0C02:03" and
  "mapping multiple BARs".** The memory controller's error driver maps
  128 KiB of its registers, where ACPI reserves 32 KiB. It reads only its
  own registers.
- **The camera bridge's "GPIO 11 is not in FW pins bitmap" and "Invalid
  speed 400000 adjusting to bus max 100000".** ACPI names a pin and a bus
  speed the USB-IO bridge's firmware doesn't offer, and the driver uses what
  it does. The camera works.
- **"HPET dysfunctional in PC10. Force disabled."** and "no address or irqs
  in _CRS". The kernel turns HPET off on any CPU whose deepest idle state
  stops it, and the TSC keeps time.
- **i8042's "Warning: Keylock active".** The keyboard controller's keylock
  status bit is set, which the kernel only reports.
- **"No UUID available providing old NGUID".** The drive reports no UUID, so
  the kernel names it by its NGUID.
- **"WQBC data block query control method not found".** One data block in
  Dell's WMI tables has no method to read it, so the WMI bus leaves that block
  out.
- **"`tokio-rt-worker' uses wireless extensions"**, once a boot. WARP's
  service reads Wi-Fi statistics through the old interface. It still gets
  them because Wi-Fi 7 is off: on a card with multi-link on, the kernel
  answers nothing. stress-ng run as root repeats the warning, since it resets
  the kernel's once-only warnings.
- **Dummy regulators** for the camera sensor, the SPI flash and the four
  amplifiers. ACPI names no supplies for them, and all three work as they
  should.

### Fixed Here Until nixpkgs Fixes It

Packages nixpkgs builds wrong for this laptop are overridden in [the hardware
file][2]. Each fix is the change nixpkgs's package needs, so it can go
upstream as it is and the override can then come out.

- **Level Zero on the GPU.** The compute runtime's `postFixup` gives only the
  OpenCL driver, `libigdrcl.so`, the graphics compiler's libraries on its
  RUNPATH. The Level Zero driver beside it gets nothing, so it can't
  `dlopen` `libigc.so.2`, and `zeInit` aborts the whole program in
  `command_stream_receiver.cpp`. OpenCL worked, but Level Zero didn't, for
  the NPU either, since `zeInit` loads every driver. With the fix, `zeInit`
  finds both the Arc B390 and the NPU. The fix in
  `pkgs/by-name/in/intel-compute-runtime/package.nix`:

  ```diff
  -      $out/lib/intel-opencl/libigdrcl.so
  +      $out/lib/intel-opencl/libigdrcl.so \
  +      $out/lib/libze_intel_gpu.so.1
  ```

- **turbostat.** Its Makefile, `tools/power/x86/turbostat/Makefile`, sets
  `CC = $(CROSS_COMPILE)gcc`, which wins over the `CC` the build environment
  sets. The kernel's package set builds with clang here, so the build stops
  at `gcc: command not found`. Passing the compiler on the command line
  fixes it, and gives a cross build its own compiler too. The fix in
  `pkgs/os-specific/linux/turbostat/default.nix`:

  ```diff
  -  makeFlags = [ "PREFIX=${placeholder "out"}" ];
  +  makeFlags = [
  +    "PREFIX=${placeholder "out"}"
  +    "CC=${stdenv.cc.targetPrefix}cc"
  +  ];
  ```

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
[13]: ../modules/firmware-settings/nixos.nix
