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
| NPU      | `intel_vpu` with Level Zero, and Intel's graph compiler; see below     |
| Audio    | the firmware Sound Open Firmware runs on the DSP                       |
| Speakers | Omarchy's tuning, as a smart filter; see below                         |
| Camera   | Intel's camera software, with the CVS bridge in its graph; see below   |
| Sensors  | the light sensor's buffer, for the backlight; see below                |
| Panel    | Chrome, Slack and 1Password told it's P3; 60 Hz on a low battery       |
| Docks    | bolt, which authorizes Thunderbolt and USB4 devices                    |
| Firmware | fwupd, for the firmware updates Dell publishes through LVFS            |
| Battery  | charges to 80% and again below 75%, in the firmware; see below         |
| Settings | the firmware's power profile and its other settings, held as chosen    |
| Wi-Fi    | runs as Wi-Fi 6; see below                                             |
| Devices  | expected: every device with a driver, its lights, keys and settings    |

### The CVS Patch

Intel's CVS bridge sits between the camera and the image processor. Its 7.2 driver claims
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
for each amplifier, and the driver picks between them by that pin; the files
are byte for byte the ones linux-firmware has as `spkid0` and `spkid1`. At
every boot the right woofer, the first to probe, loaded `spkid1`
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

### 7.3's Fixes, Taken Early

7.3 adds and fixes a good deal for this laptop's parts, and none of it is
tagged for 7.2's stable releases. Rather than wait for the kernel to move,
`kernel/backports.nix` carries what matters here: 18 patches in
`kernel/backports/`, 87 mainline commits, each patch naming its commits in
order. They were picked from every commit between 7.2 and 7.3 touching a
driver this laptop binds or could bind, and apply to 7.2.12 after the
kernel's own patches with no offset or fuzz. One, the Thunderbolt fix, had
a hunk moved onto 7.2's context, which its header says.

| Patch                           | What it gives                                       |
| ------------------------------- | --------------------------------------------------- |
| `xe-flips-in-the-vrr-window`    | flips on time with variable refresh always on       |
| `xe-real-vtotal`                | the real vertical total, which USB-C monitors need  |
| `xe-refresh-without-modeset`    | refresh rate changes without blanking the panel     |
| `xe-peak-bandwidth`             | Xe3's bandwidth point, so less memory power         |
| `xe-cdclk-for-downscaling`      | no underruns or flicker on a downscaled plane       |
| `xe-small-fixes`                | fbdev, resume hotplug, UHBR DP audio, four GT fixes |
| `sof-dsp-and-dma`               | the DSP's DMA, its period on a high-priority queue  |
| `amp-feedback-channels`         | the amplifiers' feedback on the right channels      |
| `hdmi-jack-after-resume`        | a jack plugged in during sleep is seen              |
| `sdca-hid-lifetime`             | the headset buttons' device leaves with its codec   |
| `iwlwifi-core-107`              | Wi-Fi firmware 107 and per-country puncturing       |
| `iwlmld-resume-fixes`           | Wi-Fi resumes cleanly after being powered off       |
| `btintel-pcie-reset`            | Bluetooth resets without races, and `vendor_reset`  |
| `bluetooth-le-fixes`            | LE Audio broadcast reports and advertising          |
| `ish-sensors`                   | the sensor hub's sensors, ready and scaled right    |
| `haptic-ff-before-register`     | the touchpad's force feedback from the start        |
| `thunderbolt-reset-at-shutdown` | a TB3 dock is told it's unplugged at shutdown       |
| `issei`                         | a driver for the security engine                    |

`sdca-hid-lifetime` brings the RT766 codec's driver, which nothing here uses,
because one of its commits edits it. The headset buttons' input device takes a
new name with it, from `HID sdw:0:0:01fa:4245:01` to one starting `SDCA HID`,
which no rule here matches on. `iwlwifi-core-107` loads
`iwlwifi-sc-a0-wh-b0-c107`, which linux-firmware has shipped since 20260916;
without that file the driver falls back to 106. `issei` binds the security
engine at `00:18.0` (`8086:e45d`), the device Dell's `issei.sys` drives, with
`INTEL_SSEI` and `INTEL_SSEI_HW_HECI` built as modules; it gives
`/dev/issei0` and the engine's firmware version in `/sys/class/issei`, and
nothing in userspace talks to it yet.

An assertion fails once the kernel is 7.3, which has them all, and the way
through is to drop `kernel/backports.nix` and its directory.

### linux-next's Fixes, Taken Early

Some fixes this laptop needs are in no release yet, and two are in no tree
at all. `kernel/next.nix` carries them in `kernel/next/`, applied after the
backports from 7.3 with no offset or fuzz, each patch's header naming where
it comes from: linux-next's commits and their series, a posting to a list,
or a local fix.

| Patch                                | What it gives                                          |
| ------------------------------------ | ------------------------------------------------------ |
| `xe-shrinker-spares-buffers`         | window buffers reclaimed last, so keys don't repeat    |
| `dp-byte-reads-only-when-longer`     | a failed 1-byte DPCD read isn't sent again             |
| `dp-mst-dsc-skips-empty-ports`       | an MST hub's empty port isn't asked for its DSC caps   |
| `edp-wakes-the-panel`                | the panel is woken before its eDP revision is read     |
| `xe-shared-buffers-keep-their-place` | buffers shared between programs aren't reclaimed first |
| `btintel-pcie-pm`                    | Bluetooth lets a hibernate power off                   |

`xe-shrinker-spares-buffers` is linux-next's split of xe's buffer shrinker
into purgeable and non-purgeable halves. Buffers that cost a backup and a
restore report seeks 8 against the purgeable ones' 2, so reclaim takes them
last, and `shrinker_non_purgeable_seeks` under each device tunes it from 1
to 64. Without it, a build's memory pressure backed up the buffers programs
draw into, and the compositor waited on their locks while they were read
back, so a key let go of read late and repeated.

The default of 8 wasn't enough here. Under a kernel build, traced for 5
minutes at each: at 8, kswapd backed up 141 window buffers, and niri waited
over 10 ms on a buffer lock 25 times; at 64, with kswapd scanning as much,
it backed up none, and no wait passed 4 ms. A udev rule in `kernel/next.nix` sets
64 as the GPU binds.

`dp-byte-reads-only-when-longer` is Chia-Lin Kao's posting to intel-gfx of
2026-06-16. The fallback for USB-C hubs that fail multi-byte reads retried a
failed read byte by byte even when it was one byte, the same read again,
which through an MST hub costs a second 4 s sideband timeout.
`dp-mst-dsc-skips-empty-ports` is a local fix. Adding a connector for each
port of a hub read that port's DSC caps even with nothing on it, where the
answer is no DSC whatever comes back. An HP Z27k G3's hub never answers for
its empty port, and the monitor's picture waited behind those reads, 8 s on
every plug and boot.

`edp-wakes-the-panel` is linux-next's 908f1be9c343, posted to intel-gfx on
2026-09-15. The driver reads the panel's DPCD once as it sets up its
connector, and a panel left asleep at power on can fail those reads however
often they're retried. It now sets the panel to D0 first; The Panel's
Brightness says what a failed read cost here.

`xe-shared-buffers-keep-their-place` is linux-next's d2ce8fcd778e, marked
for stable from 6.8. Each draw moved a program's private buffers to the
fresh end of the list reclaim works from, but never the buffers it shares
with another program, so the windows a compositor imports from its clients
looked unused however often they were drawn, and went first. Bringing
one back happens with its lock held. A trace of niri overnight caught its
draws waiting on a client's buffer lock for 122 to 250 ms, three times in
five minutes, and the section below, The Write-Combined Buffers Under
Reclaim, says what held them.

`btintel-pcie-pm` is four of mainline's commits from 2026-10-14:
12997a385f71, 2926eb396711, 4f6ead67b93b and f2cee6e2aa35. At a
hibernate's thaw, the driver reset the controller and probed it again,
though freeze never cut its power. The probe waited on the controller's
firmware while user space was frozen. So the power-off asked a controller
still in its bootloader for D3, timed out on its interrupt three times
and returned -EBUSY. The hibernate rolled back, leaving the laptop on
until its battery ran out. Thaw is now a plain wake, and only a restore
or a resume from S3 resets. A missed interrupt is checked against the
controller's boot stage register rather than a cache only the interrupt
updates. This kernel lacks the mailbox handshake the third commit
finishes, so that step is left out.

An assertion fails once the kernel is 7.4, which may have them, and the way
through is to drop each patch that kernel has, then `kernel/next.nix`.

### The Write-Combined Buffers Under Reclaim

Even with linux-next's two buffer fixes and seeks at 64, niri still
missed frames under a heavy load. Its tracer caught why: niri waited on a
buffer lock for 57 to 304 ms, held by Quickshell's render thread or
Chrome, each in its own submission reading back buffers the shrinker had
backed up, and spending most of it in `cpa_flush`, setting a
write-combined page's caching again, which flushes every CPU's TLB. The
submission holds its windows' locks the while, and niri's commit needs
one.

Counting what was backed up and read back, over nine 30 s rounds, found
not one buffer shared with the compositor: shared windows already keep
their place. Nearly all were the programs' own write-combined buffers. In
one round Quickshell read back 105 of them in 223 ms and 33 cached ones in
under 1 ms. The programs' own buffers, niri's aside, came to at most
632 MiB, and moved 100 to 250 MiB between resident and backed up every
10 s.

Seeks only order what reclaim takes, and slices' memory ceilings can't
reach xe's shrinkers, which aren't memory-cgroup aware.
`kernel/local/xe-shrinker-keeps-write-combined-buffers.patch`, written
here, has the shrinker skip a non-purgeable buffer that isn't cached, on
an integrated GPU. Purgeable buffers are still purged. It relies on
linux-next's 846f333b3e0b, without which a busy buffer ends the
shrinker's whole walk. Kept buffers are out of reclaim's reach, so a
program leaking them would push the machine towards the OOM killer rather
than swap. niri-off-cpu counts the buffers the shrinker keeps, so a trace
says how much that is; past 1 GiB, making a read back cheaper is the way
instead.

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

thermald's adaptive mode runs the laptop's own thermal tables, the ones Intel's
Dynamic Tuning was written to. Their ITMT3 tables steer the CPU's zone,
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

With the lid closed, Wi-Fi slows to a crawl even docked. Testing in
[Omarchy's report][5] points to the antennas in the closed lid rather than to
software, though nobody has proven it, and nothing here fixes it. Docked,
Ethernet avoids it.

### Bluetooth's Power Limits

The firmware gives Bluetooth two settings, as UEFI variables: a SAR table,
`UefiCnvBtBRDS`, with transmit limits for
each of the chip's two chains on 2.4, 5.2 and 5.8 GHz, and the antenna gain
setting, `IntelUefiCnvBtPpagSupport`, on for the EU and China. Linux's
`btintel` reads both only from ACPI methods, `BRDS` and `PPAG`, which this
firmware doesn't have, and 7.2 rejects a revision 2 SAR table wherever it
comes from. So Bluetooth ran on the controller's own defaults, with neither.

`bluetooth-sar-rev2.patch` is mainline's ad28b52441bc from 7.3, which reads
revision 2. `bluetooth-uefi-power-tables.patch`, written here, reads the two
variables when ACPI has no table and sends them as the ACPI path does. Only a
revision 2 SAR table is read that way, the one layout known to be given
there. This chip, Scorpius Peak with Whale Peak 2, is one the revision 2 code
accepts, and the driver logs "Applying Bluetooth SAR Rev2" and "PPAG-BT:
Enabled (Mode 3)" as it sends them. What the limits change in range wasn't
measured: nothing here can measure transmit power.

### The Speaker Tuning

Dell voices the speakers with a Waves tuning of its own, on top of the
amplifiers' firmware, and Linux has nothing in its place, so untuned they
sound thin. The module runs
[Omarchy's tuning][9] for this model: thirteen biquads per channel and a
limiter, fitted to a measured EasyEffects profile, with no binary blob. Its
limiter takes 9 dB more in than Omarchy's, chosen by ear, so the tuned
speakers aren't far quieter than plain ones, under the same ceiling.
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
and the image processor. [`camera.nix`][11] turns on nixpkgs's `hardware.ipu7`, which brings
the image processor's module and firmware, and swaps in Intel's camera software
from its main branch, which knows the bridge. A patch routes this sensor's graph
through the bridge, as [Intel's own change][6] does for the next image processor. Programs
open a loopback camera, "Built-in Camera", which a relay fills from the image
processor at 1080p, turned upright: the sensor is mounted upside down.
The firmware says it isn't, so a patch adds the laptop to the bridge's
upside-down table, as linux-next does for the Pro 14 Premium, and the sensor's
rotation control reads 180. Nothing reads that control yet: Intel's camera
software ignores it and libcamera has no IPU7 support, so the relay still
turns the picture itself.
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

Dell fits the laptop with one of several camera boards, and its own driver
tunes the picture for the one fitted; the HAL has only Intel's tuning
for its own reference board, KAFE799. The firmware names the board in the
sensor's `_DDN`, which Linux doesn't show: `acpi_call` reads it, at the path
in `/sys/bus/acpi/devices/OVTI08F4:00/path` with `._DDN` added. Naming it in
`jitsusama.dellXps14.cameraBoard` has the HAL read Dell's tuning for it,
taken from Dell's driver pack as the system builds, since it can't be shared.
On a sheet of white paper, Dell's tuning for CJFOE90 lit the corners to 86%
of the centre and Intel's to 76%, at the same signal to noise.

That tuning has the image processor send full-range video, brightness from 0
to 255, but Chrome and the apps built on it read a camera's frames as
limited range, 16 to 235, whatever the camera says they are: their V4L2
capture passes no colour space on. So they clipped about a tenth of the
picture to black or white. The relay squeezes each frame into limited range
and labels it so, which every program then reads the same way. It used to
convert every frame from one colour standard to another as well, from a
label nobody had set, which it no longer does. Over 150 frames it took 18% of
a core before and 14% after, the squeeze's 4% included.

The IR camera beside it, an HM1092 made for signing in by face, has no driver in
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
while the sensor streams, and only then.

The IPU7's node for it offers colour formats too, and pads each line to 64
bytes, so a program opening it as it would a webcam picks a format the sensor
can't send, and OpenCV, which Howdy reads through, shears its frames. The
[IR camera module][16] relays the stream to `/dev/ir-camera` as 8-bit grey,
which every camera program reads, and only while one reads, so the sensor and
its light stay off otherwise. Before each stream it wires the pipeline, from
the CSI-2 port the sensor's own link names. Browsers and calls aren't offered
it, since its grey picture is for signing in. Nothing signs in with it unless
a machine turns Howdy on and names the services it may unlock.

Both cameras are v4l2loopback devices, and both relays stop writing while
nothing reads. The module hands a new reader the newest frame it holds,
however old, so a call opened on the last frame of the one before, its
timestamps running backwards, which recorders reject. A patch written here,
in the [loopback module][18], starts each reader at the next frame written.

Unbinding the image processor and binding it again, which brings a hung camera back
without a reboot, freed the
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
in at 70% uncharged. Plugged in, the laptop runs from the charger either way, so
the limit costs no speed.

`dell-laptop` offers these as `charge_types` and the charge thresholds, but
with a firmware password set every write fails with EIO: Dell's token writes need
a key made from the password, and the driver sends none. So the module sets
them through the firmware's own settings ([firmware-settings][13]), which take
the admin password. The thresholds are read-only until the mode is Custom
(`ReadOnlyIfNot:PrimaryBattChargeCfg=Custom`), so the first pass writes the
mode and the second the thresholds. The battery reports the new limits at
once, though the firmware also asks for a boot to finish taking them.
power-profiles-daemon leaves a battery in the custom mode alone.

To charge to full once, `run0 firmware-settings set CustomChargeStop 100`,
and `run0 firmware-settings apply` after to bring the limit back. Both ask
for the admin password.

The module holds five more settings that were chosen here, since a firmware
update or reset can change them without a word, as 1.11.0 turned the
adapter warnings off. The firmware's power profile stays
Performance, which the power limits were measured in; SAGV stays on, so the
memory slows when idle, for the battery; the keyboard's light stays dim; the
adapter warnings stay off, so a charger the firmware thinks too weak doesn't
hold the boot; and downgrades stay allowed, so a bad firmware release can be
rolled back.
Secure Boot's settings stay out: recovering those is the enrolment's job.

### Dell's Hourly Event

About once an hour while the laptop is in use, the EC sends `dell-wmi` an
event of type 0x0012 with code 0x0000 and five words of extended data, seen
as 003c 0008 000d 004b 0001. The driver's keymap has no entry for it, so each
one logged "Unknown key with type 0x0012 and code 0x0000 pressed". It comes
from the EC's own buffer, which the firmware's WMI method passes on, and Dell
hasn't said what it means, so nothing on Linux acts on it.
`dell-wmi-ignore-0012-0000.patch`, written here, gives the keymap an entry
that ignores it. The firmware also sends empty events many times an hour,
from a branch with nothing behind it; the driver already drops those.

### Nothing to Add

- **The Synaptics USB device** (`06cb:0701`) is the camera's USB-IO bridge,
  bound to `usbio-bridge`, not a fingerprint reader. This laptop has none.
- **Option ROMs for Secure Boot.** The firmware runs none, so its TPM event
  log holds no checksums and the keys go in alone, as lanzaboote enrols them
  unless a machine says otherwise.
- **The IPSE HECI** (`00:13.0`, `8086:e462`) has no driver. Dell's chipset
  pack names it "Intel(R) IPSE HECI #1" and gives it a null driver, so
  nothing on any system drives it.
- **The overclocking watchdog** (`INTC109D`, `\_SB.PC00.LPCB.CWDT`).
  `intel_oc_wdt` knows its siblings, not this ID, and made to bind it fails
  with -13: the firmware locks it off at power-on. The TCO watchdog stays
  the watchdog.
- **The sensor hub's own sensors.** Beside the light sensors, the hinge and
  the presence sensor it offers in the standard way, the hub offers fourteen
  of its own through `hid-sensor-custom`, with no driver for each. Switched
  on, the colour light sensor and both accelerometers, the lid's and the
  base's, give live readings; ten more hold steady values or zeros, as
  sensors that report only on an event do, and one offers nothing to read.
  Their readings are in sysfs for anything that wants them; the hinge
  already gives the lid's angle, and making the accelerometers IIO devices
  would need a driver change nothing here needs.

### Measured, and Left as It Is

Each of these was measured on the laptop and gained nothing, so nothing
changes it:

- **The long-term power limit.** Raising it from 30 W to the 38 W the
  firmware allowed ran no faster: Dell's skin-temperature policy holds
  sustained power near 25 W either way. Firmware 1.11.0 allows 41 W, and the
  30 W limit holds as before.
- **The performance profile on the charger.** It sustains 22 to 28% more work than
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
`balance_performance`, as balanced already does on the charger; the profiles still
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

Two drivers offer the firmware's power profiles here: Dell's, with the
quiet, cool, balanced and performance modes Dell's own app offers, and
Intel's SoC slider, with low-power, balanced and performance.
power-profiles-daemon writes only the legacy file, which offers what every
driver has, so neither quiet nor low-power was there, and its power-saver
profile fell back to balanced: Dell's balanced table, the slider's middle
value. `soc-slider-takes-quiet.patch`, written here, has the slider take
quiet and cool as hidden choices, as AMD's driver does for the same reason,
and set them as its most efficient value. Power-saver now sets Dell's quiet
mode and the slider's low power together, the most efficient pair the
firmware has. `platform-profiles-reach-the-legacy-file` reads both drivers' choices
from the kernel's patched source.

### The Two Crashes

The laptop has panicked twice, both times on firmware 1.10.1 and kernel 7.2.9,
and pstore kept both records:

- **At boot, 20 s in.** An RCU callback freeing a dentry read the slab's
  record of which allocation tag the dentry was charged to, and found one in
  the modules' tag area whose counter pointer was garbage, so
  `__alloc_tagging_slab_free_hook()` faulted. It came 60 ms after the
  camera's ISYS bound the CVS bridge.
- **At reboot, after a sleep.** `device_shutdown()` found that the device
  before the one it was removing had been freed: the `kobj.entry` link at
  offset 8 of its 1 KiB slab object held an address in the modules' tag area
  instead. Offset 8 is where an entry in a slab's object vector keeps its
  tag, so the freed device's memory had become some slab's object vector.

Each is a use-after-free: something freed memory that was still in use. In
the second, the allocation profiler's bookkeeping was only the next owner of
that memory, writing where it was entitled to. Neither record names what
freed it. 7.2.10 and the memory list hold no fix for the object vectors that
would match, and there have been no crashes since, over five boots on firmware
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
Display P3. Dell's colour profile for it has the same primaries and no
calibration curves, so the panel needs no correction of its own, only each
surface converted to its gamut. Every program here draws as if the screen
were sRGB, which would stretch sRGB's colours over the wider gamut.

niri maps sRGB onto the primaries the EDID declares, in the display engine,
for the whole panel at once (modules/niri's `srgb-gamut`), so every program
shows sRGB as it is, as on the monitor beside it. No program is told the
panel's colours: Chromium told they're P3 would draw in P3, and niri would
map that as if it were sRGB, converting twice. Modelled by CIE76 against
true sRGB, that left pure green 24.0 off and a skin tone 4.6, worse than
with either alone.

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

The panel varies its rate from 20 to 120 Hz, but gives
that range only in its EDID's DisplayID Adaptive Sync block, which the
kernel doesn't read: it looks only for a range limits descriptor in the
base block, so the Intel driver never offers variable refresh.
`panel-variable-refresh.patch`, written here, reads the panel's native range
from that block when the base block has none. On its own it changes nothing
a person sees: the compositor still decides. In
7.2 the Intel driver turns the panel's self refresh off while variable
refresh is on, though the hardware can keep both, so it's worth turning on
only while
a video or game is showing, as niri's on-demand setting does, rather than
everywhere.

### The Panel's Self Refresh

The panel can refresh itself from its own memory while the screen is still,
and update only the part that changed, which eDP calls PSR2; this one adds
Early Transport, sending the changed part as it's drawn. On 2026-10-26, on
7.2.12, the whole screen began flashing after a monitor on the dock was
unplugged and plugged back in, and it stopped the moment self refresh was
turned off through `i915_edp_psr_debug`. Forced to PSR1, which refreshes
the whole panel rather than part of it, it flashed again within seconds,
and with self refresh off once more it flashed on. xe keeps frame buffer
compression off the panel while either self refresh is on, so turning
self refresh off let compression take the panel, which the kernel logged
as a reduced compressed buffer. The boot had logged "Selective fetch area
calculation failed in pipe A", and the panel's own pipe had no modeset all
the while; only the monitor's pipe changed, joining and splitting the
display buffer between the pipes. Each of the three shows the panel from a
buffer other than the frame just drawn.

It came back the same day on a boot made with the lid shut, once the lid
was opened and the panel came on: on PSR1, with no niri change, and it
stopped again the moment self refresh was turned off. That boot logged no
selective fetch failure, and it logged a reduced compressed buffer at
boot, which the steady boots didn't, so a lid-shut boot sets the panel up
differently. `xe.enable_psr=0` and `xe.enable_fbc=0` leave both off, at
the cost of what they save while the screen is still, which is yet to be
measured on battery. Which of them flashes is still to be told apart: self
refresh comes back alone once a lid-shut boot and a replug with both off
hold steady.

### The Panel's Brightness

The panel takes its brightness in nits over the display's AUX channel, as
eDP 1.5 allows. The kernel sets that range from the panel's luminance, which
it reads only from CTA HDR metadata, and this panel gives its luminance only
in its EDID's DisplayID Display Parameters block: 400 nits over the whole
screen. With no range found, the Intel driver guesses 0 to 512 nits, so the
top 112 of the backlight's 512 steps ask for more light than the panel is
rated for. Dell's own graphics driver gives it 25 to 400 nits.
`panel-luminance-range.patch`, written here, reads the range from that block
when no CTA block gives one, so the backlight runs from 0 to 400 in steps of
one nit, and a level keeps the same nits it had before. The panel answers 0
when asked what it was last set to, so `actual_brightness` always reads 0;
`brightness` holds the level set.

The driver picks how to set brightness from the panel's eDP revision, read
once at boot. Every boot with a monitor already on USB-C read it wrong, and
no boot without one did, though the panel answers 0x06, eDP 1.5, when asked
later. The driver then
took a PWM signal, which this OLED ignores, so the keys did nothing until 0
turned it off, and `max_brightness` read 192000. The panel also offers
Intel's interface (TCON capability 1, brightness in nits), which the driver
tries only for a panel older than eDP 1.5. With `xe.enable_dpcd_backlight=3`
a bad read took it, and the keys still did nothing: written by hand over the
AUX channel, Intel's nits at 0x354 left the screen as it was, VESA's
brightness at 0x722 dimmed it with flicker, and VESA's luminance at 0x734,
with 0x721 at 0x82, dimmed it cleanly. So `xe.enable_dpcd_backlight=2`
holds the driver to VESA's controls, which take luminance when the panel
offers it. `edp-wakes-the-panel`, from linux-next, wakes the panel before
the read, but a boot with a monitor on USB-C still read it wrong.

### The Touchpad's Click

The touchpad is a `2C2F` haptic pad on `hid-multitouch`: it has no button,
and its firmware buzzes when it's pressed. Its report descriptor gives the
strength as a feature, Intensity on the Haptics page (report 11, 0 to 100).
The firmware
starts at 50 and nothing on Linux changes it: the kernel's `hid-haptic` takes
only pads that name a trigger as well, which this one doesn't, and Omarchy
installs Dell's haptics daemon only for Synaptics pads (`VEN_06CB`).
`jitsusama.touchpad.clickStrength` sets it as the pad appears and after each
sleep, and a machine that leaves it unset keeps the firmware's 50.

Palm rejection needs nothing: the pad flags a palm in each contact's tool
type, which libinput reads.

### What's Left

| Part     | Why it isn't here yet                                                 |
| -------- | --------------------------------------------------------------------- |
| Presence | the camera bridge never senses anyone with the camera idle; see below |

- **Presence.** The sensor hub's presence reading comes from the Synaptics
  camera bridge, which the hub asks for results every 2 s once face
  detection is on. The bridge answers with nothing unless an app has the
  camera open, so the reading never changes. Dell's own hub firmware, the
  camera sensor powered, the bridge's vision-sensing bit and the hub's
  suspend state changed nothing, and Dell's own drivers on the same bridge
  get nothing from it while the camera is idle. Dell's specifications and driver pack
  offer no presence feature on this model. The firmware's other presence
  setting, Intel's, gives no reading either, idle or with the camera
  streaming. The one boot with it, which also first bound the IR sensor,
  was the first where the bridge never answered the driver handing the
  camera's link back as the camera closed, so the setting stays
  Microsoft's, the one the camera was proven on.

### Expected in the Journal

Each of these turns up at every boot or resume, or when its entry says, has
been traced to its cause, and is harmless, so nothing silences or patches it:

- **dbus-broker's "Ignoring duplicate name", at error priority.** NixOS
  gives the bus `system-path`'s `share/dbus-1` beside each package's own, so
  every service file is found twice, once through a link. Leaving
  `system-path` out would lose the services only it has.
- **No "RAPL domain uncore" on firmware 1.11.0.** 1.10.1 reported it. It only
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
- **bluetoothd's "Unable to register device interface" and "Unable to
  create object for found device"**, a pair now and then while something
  scans. BlueZ 5.87 looks a device up by its address and address type, but
  names its D-Bus object by the address alone, so an advert from an address
  it already holds under another type finds no device and can't take the
  name (src/adapter.c, src/device.c). That advert is dropped and the device
  already known is untouched. It has named the address of zeros as well as
  nearby devices' own.
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
- **"ENERGY_PERF_BIAS: Set to 'normal', was 'performance'".** The firmware
  leaves each CPU's energy bias at performance, and the kernel sets its own
  default at boot. power-profiles-daemon sets it again with each profile.
- **issei_heci's "can't derive routing for PCI INT A" and "PCI INT A: not
  connected".** ACPI gives the security engine's legacy interrupt pin no
  route. The driver takes MSI interrupts instead, and the engine answers.
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
- **Dummy regulators** for the two camera sensors, the SPI flash and the four
  amplifiers. ACPI names no supplies for them, and all of them work as they
  should.
- **intel_ipu7's and intel_ipu7_isys's "module is from the staging
  directory".** The camera's image unit driver is still in staging, so the
  kernel says so as each loads and marks itself with that taint.
- **v4l2loopback's "loading out-of-tree module taints kernel".** The camera
  relay's loopback device is a module built outside the kernel's tree, so the
  kernel marks itself with that taint as it loads.
- **"Automounting of tracing to debugfs is deprecated"**, once, when a tracer
  first reaches the tracing files through `/sys/kernel/debug`, as seen when a
  bpftrace probe started. tracefs is mounted at `/sys/kernel/tracing` too;
  only the old path is going.
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

- **The NPU's compiler.** A model runs on the NPU only once a compiler has
  turned it into the NPU's program, and nixpkgs's `intel-npu-driver` is
  built without one, so every model failed with
  `ZE_RESULT_ERROR_UNSUPPORTED_FEATURE`, from OpenVINO or anything else.
  Intel releases the compiler prebuilt beside each driver release, of the
  same version; the driver loads `libopenvino_intel_npu_compiler_loader.so`
  from its own directory, so `npu-compiler.nix` puts both libraries in the
  graphics driver directory beside it. With it, OpenVINO 2026.4 compiled a
  test network of eight convolutions for the NPU in half a second, and one
  pass ran in 3.7 ms there, against 6.8 ms on the GPU. nixpkgs could carry
  the same package, or build the driver with its compiler from source.
  Dictation (`modules/dictation`) runs Whisper small's encoder there: 75 ms
  for 30 s of audio, against about 130 ms on the GPU and 2.4 s on the CPU,
  so 11 s of speech was typed in under a second while a kernel built.
  Its decoder stays on the CPU, since its shapes grow a token at a time and
  the NPU's compiler takes only fixed ones.

### Left Out on Purpose

- **`fred=on`.** Omarchy sets it, but 7.2 turns FRED on by default wherever
  the CPU has it.
- **`rtc_cmos.use_acpi_alarm=1`.** Omarchy sets it for suspend-then-hibernate,
  but the kernel already uses the ACPI alarm on any Intel machine with firmware
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
| Graphics | named as the GPU the desktop waits for, its driver in the initrd          |
| Devices  | Wi-Fi, Bluetooth, the battery, the charger and the camera, as expected    |
| Clock    | HPET as the clocksource, since the TSC can't be trusted at boot           |
| RTC      | the CMOS clock as rtc0, the firmware's unreadable one kept from loading   |

### The Keyboard at the Disk Prompt

The keyboard isn't on USB or PS/2: it's a HID device the Surface Aggregator
presents over a UART. So an initrd with only the usual keyboard drivers
asks for the disk's PIN with no keyboard to type it on. The module puts the
whole path in the initrd, as the [linux-surface wiki][20] lists it: the
UART's driver (`8250_dw`), the Aggregator's driver, its registry and hub, and
the HID drivers. The UART's GPIO driver, `pinctrl_amd`, is built into the
kernel. A USB stick whose initrd held that set typed on this keyboard,
under Secure Boot with the owner's keys, before any disk was open.

### The Clock

Once the laptop is up, its TSC is sound: all 12 threads read within 22
cycles of each other, and against HPET it counts 2195.875 MHz, steady to
0.01 MHz second by second. Early in the boot it isn't. On kernel 7.2.3
without `tsc=unstable`, the clocksource watchdog threw the TSC out in 5 of 5
boots, about half a second after switching to it, with the TSC 1.7 ms ahead
of HPET over 0.5 s. With HPET disabled it did the same against the ACPI PM
timer in 3 of 3, by between 0.8 ms behind and 1.7 ms ahead. Kernels 6.17 and
6.18 threw it out in 4 of 5. The [ThinkPad T14 AMD Gen 1][21], on the same
Renoir platform, does the same.

The graphics driver isn't the cause: it loads seven seconds after the TSC is
thrown out, and with it kept from loading the TSC went in 3 of 3 boots all
the same. In those boots the disagreement came earlier. The refined
calibration, at about 1.5 s, measured the TSC at 2199.42 MHz, and half a
second later the watchdog found it 0.16% slow against HPET: exactly that
calibration's error. So for a stretch before about two seconds the TSC and
the chipset's timers disagree, by up to 0.35% either way, then agree.

Turning the watchdog off with `tsc=nowatchdog` keeps the TSC, but the
kernel's refined calibration measures it over that same stretch of the boot,
and came out at 2199.456 MHz in one boot and 2199.648 in another. That's
0.16% fast, past the 500 ppm NTP can steer, so the clock would lose about two
minutes a day. A frequency given with `tsc_early_khz` doesn't help: the
refined calibration replaces it whenever HPET or the PM timer is there, and
only a frequency the CPU reports skips it, which AMD's don't. So the module
marks the TSC unstable and the clock runs on HPET, which costs every read of
the time a system call.

### The Real-Time Clock

The firmware describes one clock, an ACPI Time and Alarm Device (`acpi_tad`,
`ACPI000E`), and the kernel adds the CMOS clock (`rtc_cmos`) as a fallback.
The first registered as rtc0, and every read of its date, time and seconds
gave an I/O error, so the kernel set no time at boot ("hctosys: unable to
read the hardware clock") and `timedatectl` failed. Each boot then began at
systemd's fallback date, seven months back, until NTP answered: files made
before that carried the old date, and `uptime` counted from it, at 222 days.

The CMOS clock read 2025-05-12 early in that boot, and the true time to the
second once NTP had answered, since the kernel writes it to keep it in step.
The module keeps `acpi_tad` from loading, so the CMOS clock is rtc0 and the
kernel sets the time from it at boot.

It doesn't hold the time across a power-off. At a cold start on 2026-10-26
it read 2025-05-13, though it had the right time while on. Loaded late by
udev, it then set the system's time back to that date, after systemd had
moved it forward to the last time it saved. The module loads it in the
initrd, so its date comes first, systemd's advance overrides it, and NTP
puts the time right from there.

### The Graphics Driver

Without `amdgpu` in the initrd, the driver loaded after the disk opened and
took the console from the firmware's framebuffer. udev started the console's
setup again for the new console, and `loadkeys` ran as the driver took it,
failing with an I/O error, so `systemd-vconsole-setup` was left failed. The
module loads it in the initrd, as the XPS 14's does with its GPU.

### Sleep

The laptop sleeps only in s2idle, and it sleeps soundly: one night on the
charger, `/sys/power/suspend_stats/last_hw_sleep` counted 12,327 s of
hardware sleep in the 12,328 s between going to sleep and waking, so the SoC
sat in its deepest state throughout. The graphics and Wi-Fi came back within
two seconds.

No timer can wake it from s2idle, though. A 15-second `rtcwake` alarm on the
CMOS clock never fired: its interrupt counted nothing, and the laptop slept
for three and a half hours until the GPIO controller that carries the lid,
the power button and the Aggregator woke it. The CMOS clock says it can wake
the machine only from S4. The ACPI time-and-alarm device reports
capabilities 0x1F4, which set only its S4 and S5 wake bits, so the kernel
offers no alarm on it, and reading its clock fails with an I/O error. A
sleep test on this laptop needs someone to open the lid or press the
power button.

### What's Left

| Part       | Why it isn't here yet                                                          |
| ---------- | ------------------------------------------------------------------------------ |
| Clock      | what parts the TSC from the chipset's timers early on isn't found              |
| Sleep      | the drain on battery overnight isn't measured yet                              |
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
[10]: ../hardware/dell-xps-14-da14260/sound/speakers.conf
[11]: ../hardware/dell-xps-14-da14260/camera/camera.nix
[12]: https://github.com/TsaiGaggery/hurrican_omarchy_enabling
[13]: ../modules/firmware-settings/nixos.nix
[14]: https://git.kernel.org/pub/scm/linux/kernel/git/firmware/linux-firmware.git/commit/?id=a98a6daa9f4dee2f662cc8a48610287189b23499
[15]: https://lore.kernel.org/all/20261013-hm1092-driver-v7-0-d81a9b152ffd@gmail.com/
[16]: ../modules/ir-camera/nixos.nix
[17]: https://git.kernel.org/pub/scm/linux/kernel/git/firmware/linux-firmware.git/commit/?id=8a2941fae0d02364d8ae6542ba890fd496681b55
[18]: ../modules/v4l2loopback/nixos.nix
[19]: ../hardware/microsoft-surface-laptop-4-1958/nixos.nix
[20]: https://github.com/linux-surface/linux-surface/wiki/Disk-Encryption
[21]: https://wiki.archlinux.org/title/Lenovo_ThinkPad_T14_(AMD)_Gen_1#Time_stamp_counter
