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
| Kernel   | the CVS patch, the charger's offers and the memory's SPD below         |
| Keyboard | named as the built-in one: the embedded controller's PS/2 keyboard     |
| Boot     | NVMe, USB and Thunderbolt drivers in the initrd, so a dock can unlock  |
| Graphics | Mesa, with VA-API (iHD), oneVPL, OpenCL and Level Zero for the Arc GPU |
| NPU      | `intel_vpu` with Level Zero                                            |
| Audio    | the firmware Sound Open Firmware runs on the DSP                       |
| Speakers | Omarchy's tuning, as a smart filter; see below                         |
| Camera   | Intel's camera software, with the CVS bridge in its graph; see below   |
| Sensors  | the light sensor's buffer, for the backlight; see below                |
| Panel    | Chrome, Slack and 1Password told it's P3; 60 Hz on a low battery       |
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

The amplifiers then disagreed about what they read. Dell ships two tunings
for each amplifier, and Windows picks between them by that pin, as Linux
does; the files are byte for byte the ones linux-firmware has as `spkid0` and
`spkid1`. At every boot the right woofer, the first to probe, loaded `spkid1`
and the other three `spkid0`. The pin reads low the whole time: sampled every
2 ms for 30 s while the camera opened and closed, it never went high. The
bridge's wake interrupt is an active-low `GpioInt` on that pin, and the ACPI
code that hands it the interrupt marks the pin active-low without requesting
it. The first amplifier to request the pin inherits the flag and reads 1, and
putting the pin back clears the flag for the rest.
`gpio-acpi-irq-leaves-polarity.patch`, written here, keeps the flag off the
pin, since the interrupt's trigger already carries its polarity, so every
amplifier reads the pin's real level.

### The Charger's Offers

The USB-C controller lists a charger's offers with an empty one first. A
100 W charger offering 5, 9, 12, 15 and 20 V and a PPS range comes back as
nothing, 5 V, 9 V and 12 V, then 15 V, 20 V and the PPS range. The contract
asks for the fifth offer, 20 V at 5 A, the full 100 W, but the kernel counted
the empty one as the first, so `voltage_now` read 15 V, and the charger's
offers under `/sys/class/usb_power_delivery` stopped at the empty one and
listed none. Charging itself was never short. The first offer is always 5 V
and never empty, so `ucsi-leading-empty-pdo.patch`, written here, drops an
empty one there.

Asked for the offers a charger would take as a sink, which it has none of, the
controller answers that the request isn't supported, where others answer with
an empty list. The kernel logged "UCSI_GET_PDOS failed (-95)" at each plug-in
and stopped there, so the charger's offers were registered but never linked
to it: `/sys/class/typec/port0-partner` had no `usb_power_delivery`.
`ucsi-partner-without-pdos.patch`, written here, reads that answer as none.

### The Memory's SPD

The memory is LPDDR5X soldered to the board, which SMBIOS lists as eight rows
of chips. A memory module carries an SPD chip on the SMBus that describes it;
soldered memory has none. The kernel probes for one at each of the eight
addresses anyway, since the memory type alone doesn't say, LPCAMM2 modules
being LPDDR5 too. The first probe never completes: the SMBus driver waits out
its 200 ms timeout and as long again trying to stop it, the bus stays busy,
and the seven probes after it fail with "SMBus is busy, can't use it!". That
held the SMBus driver for 450 ms at every boot and left the bus busy.
`spd-skips-soldered-memory.patch`, written here, has DMI keep each memory
device's form factor and the SPD probe skip soldered memory.

### Mainline's Fixes 7.2 Lacks

Mainline's fixes since 7.2, for the drivers this laptop runs, were read
against 7.2.11. Most were in 7.2.11 already under other subjects; two that
matter here weren't, and a third, mainline's 86c8360c97f6, came with 7.2.12:

- `dm-crypt-status-after-last-reference.patch`, mainline's 148845aa1921.
  `crypt_dec_pending` read a request's error before dropping its reference,
  so a completion that set the error in between was missed, and a failed
  read or write on the encrypted disk completed as a success.
- `cs35l56-suspend-before-attach.patch`, mainline's fbf2c660bac8 backported
  to 7.2's names. Suspending an amplifier before it first attached returned
  early, skipping `pm_runtime_force_suspend`, while resume always runs
  `pm_runtime_force_resume`, so its runtime power count went out of balance.
- 7.2.12's OT25 DAI. 7.2.11 took 4b7f393a01f8, which moves each
  amplifier's feedback capture to the OT25 DAI on SoundWire's DP4, without
  the commit that adds the DAI. The machine driver then named a DAI no
  amplifier registered, and the sound card never bound; core carried the
  commit as a patch until 7.2.12.

Once a stable release takes one, its patch no longer applies and the kernel
build stops, which is the sign to drop it.

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

### thermald's Start

thermald's adaptive mode runs the laptop's own thermal tables, as Intel's
Dynamic Tuning does on Windows. Their ITMT3 tables steer the CPU's zone,
`TCPU`, through the power limits behind `intel-rapl-mmio`, and thermald
looks for both once, as it starts. udev used to load their drivers, and on
one boot of 24 they came a second after thermald: it logged that adaptive
policy couldn't create any zones, left itself `/run/thermald/ignore_adaptive`
and stopped, for its unit to start it again without adaptive mode. NixOS's
unit doesn't restart it, and this CPU runs only in adaptive mode, so even a
restart refused it ("Unsupported cpu model") until the next boot.

The module loads `int3400_thermal`, `int3403_thermal` and
`processor_thermal_device_pci` with the early modules, which finish before
thermald may start, and none of them probes in the background, so the zones
and limits are there when it looks. `coretemp` loads early too, which stops
thermald warning there's no core temperature; on its own that wasn't the
failure, since another boot without it still ran adaptive mode. Each start
also clears the note, and thermald is started again 2 s after a failure, so
should adaptive mode fail again it tries adaptive mode again once its drivers
are there. Three starts a minute at most, so one that never can stops trying.

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

Dell fits the laptop with one of several camera modules, and its Windows
driver tunes the picture for the one fitted; the HAL has only Intel's tuning
for its own reference module, KAFE799. The firmware names the module in the
sensor's `_DDN`, which Linux doesn't show: `acpi_call` reads it, at the path
in `/sys/bus/acpi/devices/OVTI08F4:00/path` with `._DDN` added. Naming it in
`jitsusama.dellXps14.cameraModule` has the HAL read Dell's tuning for it,
taken from Dell's driver pack as the system builds, since it can't be shared.
On a sheet of white paper, Dell's tuning for CJFOE90 lit the corners to 86%
of the centre and Intel's to 76%, at the same signal to noise.

The IR camera beside it, an HM1092 that Windows signs in with, has no driver in
any kernel release yet. Three patches from the media list bring it up: its
`ipu-bridge` entry from mainline, the sensor's driver from the [v7
series][15] with Jake Steinman's patch for Intel laptops, and the IPU7's
greyscale formats, which the IPU7 has to have before any mono sensor can
stream. Jake found the cause that kept it dark for months: the link frequency
had been given as the lane's bit rate, twice the clock the receiver should
lock to. He tested the driver on the XPS 16 DA16260, with the same bridge and
sensors. The sensor streams 648x368 greyscale, raw and unprocessed, on a
CSI-2 port the firmware names only at boot, which differs between models.
Its flood light, the `HIMX1092_00::ir_flood_led` LED, belongs to the sensor's
power controller, which offers it to the sensor's driver, but no driver took
it, so every frame was dark. A patch written here has the driver light it
while the sensor streams, and only then, as Windows does.

The IPU7's node for it offers colour formats too, and pads each line to 64
bytes, so a program opening it as it would a webcam picks a format the sensor
can't send, and OpenCV, which Howdy reads through, shears its frames. The
[IR camera module][16] relays the stream to `/dev/ir-camera` as 8-bit grey,
which every camera program reads, and only while one reads, so the sensor and
its light stay off otherwise. Before each stream it wires the pipeline, from
the CSI-2 port the sensor's own link names. Browsers and calls aren't offered
it, as Windows keeps its IR camera to sign-in. Nothing signs in with it unless
a machine turns Howdy on and names the services it may unlock.

Both cameras are v4l2loopback devices, and both relays stop writing while
nothing reads. The module hands a new reader the newest frame it holds,
however old, so a call opened on the last frame of the one before, its
timestamps running backwards, which recorders reject. A patch written here,
in the [loopback module][18], starts each reader at the next frame written.

Unbinding the IPU and binding it again, which brings a hung camera back
without a reboot as disabling it in Windows' Device Manager does, freed the
capture side's nodes while WirePlumber still held them open, and the kernel
oopsed as they closed. A patch written here keeps them until the last one
closes. A sensor's node held open across the unbind still isn't made again
at the bind, so the IR camera needs WirePlumber stopped first.

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

### The Power-Saver Profile

Two drivers offer the firmware's platform profiles here: Dell's, with the
quiet, cool, balanced and performance modes Dell's own app offers on
Windows, and Intel's SoC slider, with low-power, balanced and performance.
power-profiles-daemon writes only the legacy file, which offers what every
driver has, so neither quiet nor low-power was there, and its power-saver
profile fell back to balanced: Dell's balanced table, the slider's middle
value. `soc-slider-takes-quiet.patch`, written here, has the slider take
quiet and cool as hidden choices, as AMD's driver does for the same reason,
and set them as its most efficient value. Power-saver now sets Dell's quiet
mode and the slider's low power together, as Windows' best power efficiency
does. `platform-profiles-reach-the-legacy-file` reads both drivers' choices
from the kernel's patched source.

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

### The Light Sensor

The sensor hub shows its ambient light sensor twice, as two `als` devices
reading the same light. `iio-sensor-proxy` gives a buffer half a second to fill
after turning it off and on again, which the hub never manages, so the proxy
read the first device on a timer. The backlight module turns both buffers on
through udev, keeps the proxy off them, and reads the first itself: the hub
sends a sample only when the light changes by 1%, checking ten times a second.

### The Panel's Colours

The panel, LG Display's 0x07C6, shows more than sRGB: its EDID gives red
0.689/0.310, green 0.240/0.721 and blue 0.136/0.045, a little wider than
Display P3. Dell's Windows profile for it has the same primaries and no
calibration curves, so Windows changes nothing for the whole screen and only
the programs that convert colours use it. niri has no colour management, so
every program here draws as if the screen were sRGB, which stretches sRGB's
colours over the wider gamut.

Chromium converts to the screen's colours when told what they are, so the
module names the panel's as P3 and the panel module adds
`--force-color-profile=display-p3-d65` to Chrome, Slack and 1Password. Against
true sRGB, by CIE76 from the panel's primaries, a skin tone was 10.8 off and
is 3.8 with the flag, Slack's aubergine 8.1 and 2.9, pure green 53.2 and 9.7:
pure primaries stay a little off, because P3 is narrower than this panel.
Other programs still draw sRGB stretched, as on Windows; only colour
management in niri fixes them.

### The Panel's Rate

Below 30% on battery the panel drops from 120 Hz to 60 Hz, and it comes back
when the charger is plugged in. Measured unplugged in light use, in five
alternating pairs of 24 s, 60 Hz drew a median 1.4 W less than 120 Hz, about
a tenth of the laptop's draw. It waits for a low battery because scrolling is
less smooth at 60 Hz. Once the battery is under 30% it stays at 60 Hz until
the charger is back, even if the reading climbs over 30% again, so the screen
can't flick between the two. A rate picked by hand with niri holds until the
next change.

### The Panel's Variable Refresh

The panel varies its rate from 20 to 120 Hz, as Windows uses it, but gives
that range only in its EDID's DisplayID Adaptive Sync block, which the
kernel doesn't read: it looks only for a range limits descriptor in the
base block, so the Intel driver never offers variable refresh.
`panel-variable-refresh.patch`, written here, reads the panel's native range
from that block when the base block has none. On its own it changes nothing
a person sees: the compositor still decides. In
7.2 the Intel driver turns the panel's self refresh off while variable
refresh is on, which Windows doesn't, so it's worth turning on only while
a video or game is showing, as niri's on-demand setting does, rather than
everywhere.

### The Touchpad's Click

The touchpad is a `2C2F` haptic pad on `hid-multitouch`: it has no button,
and its firmware buzzes when it's pressed. Its report descriptor gives the
strength as a feature, Intensity on the Haptics page (report 11, 0 to 100),
which is what Windows' haptic feedback intensity slider writes. The firmware
starts at 50 and nothing on Linux changes it: the kernel's `hid-haptic` takes
only pads that name a trigger as well, which this one doesn't, and Omarchy
installs Dell's haptics daemon only for Synaptics pads (`VEN_06CB`).
`jitsusama.touchpad.clickStrength` sets it as the pad appears and after each
sleep, and a machine that leaves it unset keeps the firmware's 50.

Palm rejection needs nothing: the pad flags a palm in each contact's tool
type, which libinput reads as Windows reads the same flag.

### What's Left

| Part         | Why it isn't here yet                                                 |
| ------------ | --------------------------------------------------------------------- |
| Presence     | the camera bridge never senses anyone with the camera idle; see below |

- **Presence.** The sensor hub's presence reading comes from the Synaptics
  camera bridge, which the hub asks for results every 2 s once face
  detection is on. The bridge answers with nothing unless an app has the
  camera open, so the reading never changes. Dell's own hub firmware, the
  camera sensor powered, the bridge's vision-sensing bit and the hub's
  suspend state changed nothing, and Windows on the same bridge sends it
  nothing while the camera is idle. Dell's specifications and driver pack
  offer no presence feature on this model. The firmware's other presence
  setting, Intel's, gives no reading either, idle or with the camera
  streaming. The one boot with it, which also first bound the IR sensor,
  was the first where the bridge never answered the driver handing the
  camera's link back as the camera closed, so the setting stays
  Microsoft's, the one the camera was proven on.

### Expected in the Journal

Each of these turns up at every boot or resume, has been traced to its cause,
and is harmless, so nothing silences or patches it:

- **dbus-broker's "Ignoring duplicate name", at error priority.** NixOS
  gives the bus `system-path`'s `share/dbus-1` beside each package's own, so
  every service file is found twice, once through a link. Leaving
  `system-path` out would lose the services only it has.
- **No "RAPL domain uncore" on BIOS 1.11.0.** 1.10.1 reported it. It only
  meters the uncore's energy; the power limits are unaffected.
- **xe's "Selective fetch area calculation failed in pipe A"**, once. When
  the panel's self refresh can't work out which part of a frame changed, the
  driver sends the whole frame instead, and says so the first time. The
  driver marks the case as one still to optimise.
- **"no PCM in topology for HDMI converter 3"** from the sound card. The
  HDMI codec has four converters and the DSP's topology three outputs. The
  GPU drives four displays, the panel being one, so the three cover every
  external display.
- **Bluetooth's "Received gp1 mailbox interrupt"**, four times as the
  controller loads its firmware. The driver reports every interrupt from that
  mailbox as an error and dumps its registers for debugging; the firmware
  loads and the controller works. On some boots, and on resuming from them,
  that dump also logs "Controller device warning": a bit the controller's
  own firmware sets in its boot stage register, which the driver reports and
  does nothing with.
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
- **intel_ipu7's and intel_ipu7_isys's "module is from the staging
  directory".** The camera's image unit driver is still in staging, so the
  kernel says so as each loads and marks itself with that taint.
- **iio-sensor-proxy's "Found proximity sensor but no PROXIMITY_NEAR_LEVEL
  udev property" and "No sensors", twice.** The sensor hub offers two light
  sensors, a presence sensor and the hinge's angle. The light sensors belong
  to the backlight daemon (modules/backlight), which tells the proxy to leave
  them alone. The presence sensor reports no reading on this firmware (see
  "Presence" above), so no udev rule gives it a near level, and the proxy
  finds nothing it can serve and exits, which it does twice each boot.
- **gnome-keyring's "discover_other_daemon: 1" and "0".** gnome-keyring
  50.0 prints whether it found a daemon already running, with g_printerr
  and no newline, so the line runs into the next one. Upstream's main still
  prints it.

### Fixed Here Until nixpkgs Fixes It

Packages nixpkgs builds wrong for this laptop are overridden in [the hardware
file][2]. Each fix is the change nixpkgs's package needs, so it can go
upstream as it is and the override can then come out.

- **Level Zero on the GPU.** The compute runtime's `postFixup` gives only the
  OpenCL driver, `libigdrcl.so`, the graphics compiler's libraries on its
  RUNPATH. The Level Zero driver beside it gets nothing, so it can't
  `dlopen` `libigc.so.2`, and `zeInit` aborts the whole program in
  `command_stream_receiver.cpp`. OpenCL worked, but Level Zero didn't, for
  the NPU either, since `zeInit` loads every driver. With this fix and the
  loader's below, `zeInit` finds both the Arc B390 and the NPU. The fix in
  `pkgs/by-name/in/intel-compute-runtime/package.nix`:

  ```diff
  -      $out/lib/intel-opencl/libigdrcl.so
  +      $out/lib/intel-opencl/libigdrcl.so \
  +      $out/lib/libze_intel_gpu.so.1
  ```

- **Level Zero's loader.** The loader 1.32.0 checks a driver's file exists
  before it opens it, in `LD_LIBRARY_PATH`, the usual distribution directories
  and `/etc/ld.so.conf`; upstream's 1.34.0 still does. NixOS links the drivers
  into `/run/opengl-driver/lib`, which is on the loader's RUNPATH but in none of
  those, so it logged "0 Drivers Discovered" and `zeInit` failed with
  `ZE_RESULT_ERROR_UNINITIALIZED` for every program, unless something set
  `LD_LIBRARY_PATH`. nixpkgs's setup hook sets it only inside builds. The NPU
  driver's `npu-umd-test --config none` stopped at its setup that way, and with
  the fix, which adds the driver directory to the list the loader checks, the
  loader opens both drivers and the 13 device and driver tests pass. It goes in
  as an overlay, since programs link the loader. The fix in
  `pkgs/by-name/le/level-zero/package.nix`, with the patch here:

  ```diff
  +  patches = [ ./driver-link.patch ];
  +  postPatch = ''
  +    substituteInPlace source/loader/linux/driver_discovery_lin.cpp \
  +      --replace-fail @driverLink@ ${addDriverRunpath.driverLink}
  +  '';
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

- **Bluetooth's DDC file.** linux-firmware 20260916's
  `intel/ibt-00a0-01a1-pci.ddc` ends in a stray newline. The kernel reads it
  as a record 11 bytes long with 1 byte left: 7.2.9 sent it past the end of
  the file, and the controller refused it with "Failed to send
  Intel_Write_DDC (-95)"; 7.2.10 checks the length first and says "Malformed
  DDC record (plen=11, remaining=1)". The two real records go first either
  way, so the controller has its parameters. linux-firmware's
  [a98a6daa9f4d][14] drops the byte two days after that release.

- **Bluetooth's firmware.** The same release carries Intel's build from
  July for this chip, where linux-firmware's [8a2941fae0d0][17] has its
  release 24.80.0.2 from 1 October, under the same file names. That commit
  also holds the fixed DDC file, so all three files come from it and go
  ahead of the package's until nixpkgs has a newer one.

- **bolt's USB4 controllers.** bolt 0.9.8 decides whether a USB4 domain
  keeps its UUID across reboots from a table of controller PCI IDs, and
  Panther Lake's 0xe433 and 0xe434 aren't in it, so every boot logs "failed
  to determine if uid is stable: unknown NHI PCI id" before taking them as
  unstable, which they are. Upstream bolt's 3a38c237, after its 0.9.11,
  adds them; the patch adds the same four lines to
  `boltd/bolt-sysfs.c`'s `nhi_table`.

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

## Microsoft Surface Laptop 4 (1958)

[`hardware/microsoft-surface-laptop-4-1958/nixos.nix`][19]. The 13.5" model
with AMD: a Ryzen 5 Microsoft Surface Edition (six Zen 2 cores, Renoir) with
Radeon graphics, Intel's AX200 Wi-Fi 6, a Nuvoton TPM 2.0, and Surface's own
embedded controller, the Surface Aggregator Module, behind which sit the
keyboard, the battery and the charger. The stock kernel drives all of what's
below, so the module adds no patches.

| Part     | What the module does                                                      |
| -------- | ------------------------------------------------------------------------- |
| CPU      | the kernel compiled for `znver2`, microcode, KVM                          |
| Keyboard | named as the built-in one, `045e:09ae`, and its drivers in the initrd     |
| Boot     | NVMe and USB drivers in the initrd                                        |
| Graphics | named as the GPU the desktop waits for                                    |
| Devices  | Wi-Fi, Bluetooth, the battery, the charger and the camera, as expected    |

### The Keyboard at the Disk Prompt

The keyboard isn't on USB or PS/2: it's a HID device the Surface Aggregator
presents over a UART. So an initrd with only the usual keyboard drivers
asks for the disk's PIN with no keyboard to type it on. The module puts the
whole path in the initrd, as the [linux-surface wiki][20] lists it: the
UART's driver (`8250_dw`), the Aggregator's driver, its registry and hub, and
the HID drivers. The UART's GPIO driver, `pinctrl_amd`, is built into the
kernel. A USB stick whose initrd held that set typed on this keyboard,
under Secure Boot with the owner's keys, before any disk was open.

### What's Left

| Part       | Why it isn't here yet                                                          |
| ---------- | ------------------------------------------------------------------------------ |
| Clock      | its NixOS ran with `tsc=unstable` on HPET, unexplained; it needs a measurement |
| Sleep      | suspend and resume aren't measured yet                                         |
| Audio      | the speakers and microphones aren't checked yet                                |
| Camera     | the front camera is listed; its picture isn't checked yet                      |
| Firmware   | Microsoft ships firmware updates only for Windows                              |
| Touch, pen | linux-surface lists the touchscreen and pen as unsupported on this model      |

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
[14]: https://git.kernel.org/pub/scm/linux/kernel/git/firmware/linux-firmware.git/commit/?id=a98a6daa9f4dee2f662cc8a48610287189b23499
[15]: https://lore.kernel.org/all/20261013-hm1092-driver-v7-0-d81a9b152ffd@gmail.com/
[16]: ../modules/ir-camera/nixos.nix
[17]: https://git.kernel.org/pub/scm/linux/kernel/git/firmware/linux-firmware.git/commit/?id=8a2941fae0d02364d8ae6542ba890fd496681b55
[18]: ../modules/v4l2loopback/nixos.nix
[19]: ../hardware/microsoft-surface-laptop-4-1958/nixos.nix
[20]: https://github.com/linux-surface/linux-surface/wiki/Disk-Encryption
