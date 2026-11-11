# The Machine Daemon

How this machine is doing, held ready for the shell: a Rust program,
`machine`, that reads the kernel's files and asks UDisks for each disk's
health, UPower for each battery's times, logind what keeps it awake
and fwupd for each device's firmware, the screen's backlight and the
room's light sensor among the files, and answers over `$XDG_RUNTIME_DIR/daemons/machine.sock` in
version 1 of the shell's daemon interface (hello, get, list, watch,
unwatch, health), and acts on what it holds when asked (act), so the
launcher spawns and writes nothing itself. [`src/main.rs`][main] says how
it reads and when;
[`src/lib.rs`][lib] how a record names its figures, with its unit in its
key. The launcher's Machine draws only what it holds
([`../quickshell/launcher/Machine.qml`][machine]).

It reads nothing while nobody watches. Watched, it reads once a second,
and once more a fifth of a second after a read whose rates aren't of now.
On a sixteen-core laptop under load, a read of the processor, memory,
disks and boot took 4 to 7 ms, and of every process 25 to 30 ms (7
November 2026), which is why the programs are
watched only while a part saying them is shown, and a process's tree
only by its key, for the pids a client names. The shell watches the
processor, memory and boot only while one of Machine's answers is
shown, and the disks only while a Disk answer or row is, since on an
NVMe disk each temperature read is a command to the drive; a disk's
limits are read once and kept. Rates are timed on the clock that counts
since boot, which the wall clock's steps don't move, and UDisks is asked
once a minute while disks are watched, its refusals said as such.

A tree, `tree:<pid>`, is a process and every process under it, by each
one's parent: a program's drawer watches its own pid's, so what it
started counts as its own. A process that started since the read before
counts all its time, and a pid given again counts from its new start.

Each bus is asked on a thread of its own, only while a kind it gives is
watched: UDisks once a minute, UPower every 5 seconds, logind every 2,
and fwupd once a minute and at once each time it says something
changed, a device's problems among them. fwupd's signals are listened
for only while firmware is watched; logind's `PrepareForSleep` is
listened for from the daemon's start, since a sleep comes while
nothing watches, and a signal wakes it only when a sleep begins or
ends. A watch starting sooner than
that after the last ask doesn't ask again, and each call gives up after
ten seconds. A chip's labels and limits are read once, until the chips
listed change, since some answer every read through slow firmware.

## Ready Rows

Heat's, Power's, Firmware's and Machine's own rows and headlines are made
here, ready for the launcher, in the one form every daemon serves them in: each a
`row:<place>:<id>` record of kind `row`, its row under `row` naming what
it shows as its `thing` (`power:limit`), and the headline a
`head:<place>` record of kind `head`. A watch names its place,
`{"ask": "watch", "kind": ["row", "head"], "place": "heat"}`, and
hears only that place's. While one does, the sensors, zones, cooling
and processor are read together once a second as their own watch would
have them, and each read makes the rows again and tells only those that
changed; a graph is sent in the steps the shell draws, so a reading
that leaves it as drawn changes nothing. The processor's telemetry is
asked only then, and the Last Reads kept only then, so a shut launcher
costs nothing. Power's are made the same way from the supplies, its
sparks of watts and volts kept a read a second while watched, and each
of its Charge Limit's choices asks the `charge-limit` act. Firmware's
are made from what fwupd says, asked once a minute while they're
watched; an act that can't be taken back holds one `sure` row a step
in that asks it, and while an act goes its row says so, then how it
ended until the launcher shuts. Machine's
(`place` `machine`) are made from the memory and the boot: the Memory
and each of its parts, and the Uptime, which travels as a time the
shell words (`lasted`, and Started a `moment`). Using It names the
programs only while something watches `using`, since that reads every
process's files, and holds its place saying it's still reading
otherwise. They're said
in the launcher's words and number settings,
handed on the command line (`--words`, `--numbers`) by home.nix from the
launcher's own; handed none, it makes no rows.
[`src/heat_rows.rs`][heat-rows], [`src/power_rows.rs`][power-rows],
[`src/firmware_rows.rs`][firmware-rows] and
[`src/machine_rows.rs`][machine-rows] make them,
[`src/rows.rs`][rows] places and graphs them, and
[`src/words.rs`][words] words them.

## Acts

`{"ask": "act", "act": <name>, ...}` is answered `{"done": true}` once
done, or an error: `bad-act` for what this machine hasn't, `refused`
where the kernel or a service said no, in its words, `stopped` for a
firmware act waited for past its `patience_s`, `busy` while another
firmware act goes, and `unknown-act` for a name it doesn't know. What a
charge limit moved is read again before it's answered, so a watch hears
the change first, never a second late. A firmware act can wait minutes
on fwupd or a polkit prompt, so each goes on a thread of its own, the
connection answering the rest meanwhile, and fwupd is asked again as it
ends.

| Act                | Given                      | What it does                                         |
| ------------------ | -------------------------- | ---------------------------------------------------- |
| `charge-limit`     | `supply`, `stop_pct`       | stops charging there, starting 5 below where it can  |
| `firmware-install` | `device`, `patience_s`     | fwupdmgr puts in the update fwupd offers it, alone   |
| `firmware-fix`     | `attribute`, `patience_s`  | fwupd fixes the security attribute fwupd names so    |
| `firmware-undo`    | `attribute`, `patience_s`  | fwupd puts that fix back                             |
| `firmware-setup`   | `patience_s`               | logind restarts into the firmware's own setup        |
| `restart-unit`     | `unit`                     | restarts a user unit the shell may, once queued      |

An install never restarts, which stays Joel's. It runs the fwupdmgr
`MACHINE_FWUPDMGR` names, which the shell's home configuration hands the
service, since fwupdmgr fetches and checks the update as fwupd's own
client does; a machine without fwupd names none and installs nothing.
The rest are asked over the system bus, letting polkit ask Joel where
its rules say to.

A charge limit's files are written in the order the kernel takes them,
which refuses a start at or above the stop.

A restart goes through systemd's user bus, never a spawned `systemctl`,
and is answered once systemd has queued it. Only the units in
`src/units.rs`'s `RESTARTABLE` are restarted, spend-watch's alone now,
for Spend's Start the Watch; any other is refused `not-allowed`.

## Every Figure, and Where It Shows

Joel's bar (#2204): every true figure its source has, live, in its own
units, or named here as left out with the reason.

| Record          | Figure                                       | Where it shows                            |
| --------------- | -------------------------------------------- | ----------------------------------------- |
| `processor:0`   | busy, and each field of `/proc/stat`'s time  | Processor's headline, Time Spent          |
|                 | `steal`                                      | Time Spent's Stolen, only inside a VM     |
|                 | pressure, some: 10 s, 1 min, 5 min           | Pressure                                  |
|                 | switches, interrupts, deferred, forks a sec  | Switches, Interrupts, Started             |
|                 | each core's busy, kind, clock, temperature   | Cores                                     |
|                 | clock range, base, driver, governor, EPP     | Clock                                     |
|                 | turbo                                        | Clock's Turbo                             |
|                 | package temperature, high, critical          | Temperature                               |
|                 | load, running, blocked, tasks                | Load, Running                             |
|                 | NPU busy, clock, most clock                  | NPU Load                                  |
|                 | model, vendor, family, model, stepping       | Model                                     |
|                 | microcode, caches (size, ways, line, shared) | Model's Microcode and Caches              |
| `memory:main`   | used, available, total                       | Memory's headline, Used                   |
|                 | pressure, some and full                      | Pressure                                  |
|                 | faults, major faults a second                | Faults                                    |
|                 | swapped and compressed in and out a second   | Swapping                                  |
|                 | paged in and out a second                    | Paging                                    |
|                 | swap, swap cached, page cache                | Swap, Cached                              |
|                 | what holds it, zswap's pool and its pages    | Held By                                   |
|                 | committed, commit limit                      | Committed                                 |
|                 | out-of-memory kills since boot               | Out of Memory                             |
|                 | every field of `/proc/meminfo`, huge pages   | Every Count, by the kernel's own names    |
|                 | each module the firmware lists               | Type                                      |
| `disk:<name>`   | read, written, discarded a second, requests  | Speed, its parts                          |
|                 | merged requests a second                     | Reading's and Writing's notes             |
|                 | flushes a second                             | Speed's Flushing                          |
|                 | busy, latency of each kind of request        | Busy, Latency                             |
|                 | in flight, queued on average, queue depth    | Queue                                     |
|                 | io pressure, some and full                   | Pressure                                  |
|                 | Composite temperature, high, critical        | Temperature                               |
|                 | model, firmware, NVMe revision, link, size   | Model                                     |
|                 | block sizes, scheduler and those offered     | Model                                     |
|                 | solid state or spinning, fixed or removable  | Model's Kind                              |
|                 | each file system's room, files, kept back    | File Systems                              |
|                 | SMART: wear, spare, read, written, powered   | Health                                    |
|                 | busy time, errors, error log, shutdowns      | Health                                    |
|                 | time past warning and critical heat          | Health's Too Hot For                      |
|                 | critical warnings, when it was asked         | Health's Warnings, its note               |
| `boot:<id>`     | when it started, time asleep, idle share     | Uptime, Asleep, Idle, Started             |
|                 | kernel release and build                     | Linux, its Release and Build              |
| `using:cpu:N`   | each program's share, its processes          | Processor's Using It                      |
| `using:memory:N`| each program's resident memory, processes    | Memory's Using It                         |
| `tree:<pid>`    | its share, memory, processes, threads, pids  | a program's drawer, watching it           |
| `processor:0`   | throttles: package and each core, count, time, longest | Heat's Throttled                |
| `sensor:<chip>` | each temperature, low, high, critical, emergency, alarm | Heat, each part by what it measures |
|                 | each fan's speed, least, most, aim, power, mode | Heat's fans                            |
|                 | each voltage, current and power, with limits | Heat's part's drawer, each figure         |
|                 | the chip's name                              | Heat's part's drawer, Linux Name          |
| `zone:<type>`   | temperature, policy, mode, each trip point   | Heat, for a part no chip gives; drawer's trips and Zone |
| `cooling:<type>`| how many, each one's state, the most         | Heat's Processor's drawer, Cooling Step   |
| `supply:<name>` | capacity, its level                          | Power's Charge                            |
|                 | charge now, full and designed; energy        | Power's Energy and its drawer             |
|                 | health against design, cycles, health        | Power's Health, Cycles                    |
|                 | volts, amps, watts, design voltage           | Power's Drawing, Voltage                  |
|                 | charge type and those offered, limits, settable | Power's Charge Type, Charge Limit      |
|                 | maker, model, technology, made, temperature  | Power's Model, Made, Temperature          |
|                 | serial                                       | Power's Model's drawer, Serial            |
|                 | a port's volts, amps, those offered, USB types | Power's Charger, each port and its drawer |
|                 | a port's roles, mode, power delivery revision | each port's drawer                       |
|                 | what's plugged in: its revision, accessory, offers | each port's drawer, Partner, Offers |
|                 | UPower's state, times, rate                  | Power's headline, Drawing                 |
| `sleep:main`    | mode, those offered, the ways Linux can sleep | Sleep's side, Other Modes, Ways to Sleep |
|                 | successes, the last failed step and its count | Slept, Sleep's note                      |
|                 | failures by step, last device, errno         | Failed, Blocked By, Last Error, Error Code |
|                 | last and all hardware sleep, and the most it counts | Sleep's drawer                     |
| `awake:main`    | logind: idle, since, idle action and delay   | nothing yet; Keep Awake's drawer watches it next |
|                 | every inhibitor, the last sleep's start, end | nothing yet, as above                     |
| `firmware:<id>` | name, vendor, version                        | Firmware, each device                     |
|                 | lowest and bootloader versions, branch       | each device's drawer                      |
|                 | the hidden, needs-reboot and require-ac flags | which are listed, Install's note         |
|                 | problems, as fwupd keys them                 | the row's note, Install, Blocked By       |
|                 | the update: version, urgency, size, date, time to install, summary, notes | Offered and its drawer |
|                 | fwupd's words for offering none              | the drawer's Nothing Offered              |
|                 | the last update's state, version, error      | at the restart, Last Install              |
|                 | when an update last went in                  | Last Updated                              |
|                 | flashes left, battery, protocols             | each device's drawer                      |
|                 | plugin, serial                               | the drawer's last, in the Name face       |
| `firmware:host` | security level                               | Security Level's side, note and drawer    |
|                 | product, fwupd's version                     | Security Level's drawer                   |
|                 | each attribute's name, level and result      | a step in, Needed for Level, Lowered By   |
|                 | its description, fix flags, plugin and id    | each attribute's drawer                   |
|                 | the setting it reads, and what passing needs | each attribute's drawer                   |
|                 | can fix, can undo                            | Secure It, Undo It, a step in             |
| `backlight:<device>` | its steps, the most it takes            | the screen's Brightness, row and drawer   |
|                 | the steps the hardware reads back            | the drawer's Reads, where they differ     |
|                 | its scale, its type                          | the drawer's Stepping and Driven By       |
|                 | its device                                   | the drawer's Device, in the Name face     |
|                 | the screen it lights                         | which screen holds Brightness, never said |
| `light:<device>`| the lux it hears                             | Room Light, its row and the drawer        |
|                 | its raw count, scale and offset              | the drawer's Raw, Per Count and Offset    |
|                 | its device                                   | the drawer's Sensor, in the Name face     |
| `brightness:built-in` | the level the backlight program settled, whether it follows the room | Brightness's note, Follows |
|                 | the lux it heard, what its curve asks for them | Room Light, on its way to                |
|                 | the curve's points, dimmest first            | the drawer's Curve                        |
| `warmth:built-in` | the warmth, follows the room, sees colour  | Warmth's side and its Follow the Room switch |
|                 | the room's white, the night light, its white | Warmth's figures and Night Light switch   |
|                 | the night's sunset and sunrise               | Warmth's note and figures                 |
| `camera:<name>` | the camera's name, the programs holding it   | Camera's note and hint, and Cameras       |
| `kept:idle`, `kept:lid` | whether Keep Awake holds the lock, its lock | Keep Awake, the margin, `:set awake` |
| `profile:main`  | the profile power-profiles-daemon is on, those it offers | Session's Power Profile, judging a pick |
|                 | each program's hold, why performance is slowed | nothing yet; the profile's drawer next  |
|                 | what the machine chose, follow.sh's `wanted` | Now's Power Profile, set by hand or not   |
| `keyboards:connected` | each keyboard the kernel lists, as it says it | Keyboards, the layout's map, Layer |
| `capslock:keyboard` | whether caps lock is on                  | the margin's caps lock, a prompt's warning |
| `layer:keyboard` | the board's layer, whether it's locked there | Layer, Keyboard's layers, the map        |
| `counts:<board>` | its presses and holds, the keys pressed most, each layer's and modifiers' presses, when counting began | a keyboard's drawer |
| `status:<watcher>` | what its status file says, but its heartbeat; whether it's gone silent | the margin's Lag and Disk, Waits |

`brightness` and `warmth` are the backlight program's (modules/backlight),
which writes them whole in its files, and `camera` is each camera's file
in /run/camera-holders, which modules/v4l2loopback and
modules/usb-camera-holders write whole. `keyboards` and `capslock` are
the keyboard-lights watcher's (modules/quickshell/nixos.nix), the kernel's
list of input devices read again each time the watcher's list of the
keyboards it holds changes, since the kernel's file can't be watched; and
`layer` is the ErgoDox's watcher's (modules/ergodox), in the session's
runtime folder. `counts` is key-counts' file (modules/key-counts), written
beside and renamed over a minute after a key is pressed, summed for each
board here so the shell sums nothing. `status` is the status file each
watcher writes whole and renames into place: lag-watch's
(`status:lag-watch`) and each disk guard's, the account's
(`status:disk-guard`) and the machine's (`status:disk-guard-machine`).
Its heartbeat is left out, so a write saying only that it's still
running tells nobody anything, and a watcher is said silent once no
heartbeat has come for its silence (30 seconds for lag-watch, 120 for a
guard), counted on the clock a suspend stops. All of them are heard as
they're written while watched, through inotify, and read at no other
time, but for a status read again once its watcher's silence runs out.
Keep Awake's locks are the files its services leave in the runtime folder
(modules/quickshell/home.nix), and the machine's choice is follow.sh's
file in `/run/power-profile` (modules/power-profiles-daemon): both are
heard through inotify from the daemon's start, and power-profiles-daemon
as it says a property changed, since the shell watches them while the
launcher's shut. A folder not there, or taken away as systemd does a
service's runtime folder, is looked for again every 10 seconds.

## Left Out, and Why

- **Time stolen on bare metal.** Only a machine this one runs inside
  takes it, so it's always none here; Time Spent lists it only where the
  processor's flags say it runs inside one.
- **Every task waiting for a processor.** The kernel keeps `full` for a
  processor only for a cgroup, and says 0 for the whole machine.
- **Each core's split of time.** Time Spent says the whole processor's;
  a core says how busy it was.
- **The runnable count from `/proc/loadavg`.** It's Running's count, read
  a moment apart.
- **Offline cores' figures.** The kernel keeps none while a core is off.
- **The NPU's busy time since boot.** NPU Load is its share of the last
  second, which is what moves.
- **The disk's other sensors.** Heat shows every sensor and its history.
- **SMART's own temperature and its sensors.** They're the Composite
  sensor's reading, which Temperature shows.
- **Unallocated capacity and the controller's state.** NVMe namespaces
  are whole on a laptop's disk, so it's always none, and the state says
  only that the controller is live.
- **Host read and write commands.** UDisks doesn't give them.
- **Bytes each kind of request moved since boot.** Health's Read and
  Written give the disk's whole life, which is what lasts; since boot,
  a note had no room for them beside the rates.
- **The boot's id.** Nothing is done with it.
- **Each process of a tree on its own.** A drawer says what its program
  costs as a whole; Using It names programs apart. A tree lists its
  pids, so a drawer counts once a process two watched trees hold, but
  not their figures.
- **The processor package's power.** Its energy counter is root's alone,
  so the processor's own telemetry daemon gives it, and Heat reads it
  there.
- **Which cooling device each thermal zone drives.** They're the
  firmware's wiring, and nothing Joel acts on; each device's state shows.
- **A chip's electric limits.** The only chips here that say them are
  the USB-C ports', whose figures Power shows from their supplies, so
  Heat's drawer gives each reading alone.
- **A battery's alarm and UPower's warning level.** The alarm is a
  figure in the battery's own unit for when the firmware warns, and the
  warning says the charge is low, which Charge's bar shows from 20%.
- **A supply's present and online flags, scope and status for a
  battery.** They say which supply is the machine's battery and whether
  something's plugged in, which Power's rows say in their words.
- **A port's USB-C revision.** It says which revision of USB-C the port
  was built to, the same for every port of a machine, where the power
  delivery revisions beside it say how each charger talks to it.
- **A thermal zone's policy and mode.** They say how Linux is set to
  manage the zone, the same for every zone, and nothing Joel acts on.
- **fwupd's devices with no name.** They're pins it keeps for its own
  checks, with no version and nothing to update.
- **A firmware device's other flags, parent, version format, summary
  and times.** They say how fwupd handles the device and when it first
  saw it, nothing Joel acts on; the flags that change what Install does
  show as what it does. fwupd's own update error is its words for a
  device it hides.
- **A security attribute's GUIDs, link, time and other flags.** The
  GUIDs name the device it was read from, the link is fwupd's page for
  the attribute, which its description already says, and the time is
  when fwupd read it, on each start; the flags that say what fixes it,
  or that it couldn't read what it needed, show as what they mean.
- **A security attribute's fallback result.** The daemon serves it, but
  fwupd says it may be the state before a fix or one counted the same,
  without saying which, so the launcher can't say what it means.
- **An update's vendor, protocol and download.** The device names its
  vendor and protocols, and fwupd fetches the download itself.
- **The last sleep before the daemon started.** logind says a sleep as
  it happens and keeps none, so it's kept from the daemon's start, its
  signal listened for whether or not `awake` is watched; time
  asleep since boot is `boot:<id>`'s, whole.
- **A sensor's reading the chip can't give.** A sensor saying nothing, or
  a temperature no part could reach (an unused sensor's 65,261 degrees),
  is left out rather than shown.
- **The backlight's power state.** `bl_power` is on whenever the
  launcher can be seen, since a blanked panel shows nothing.
- **The light sensor's sampling frequency.** It says how often the
  sensor hub samples, nothing Joel acts on; the lux it hears shows.
- **The light sensor's name.** It's always `als`, since only a sensor
  of that name is read, so it names nothing; its device does.
- **The screen a backlight lights, as a name.** Its connector only
  chooses which screen's drawer holds Brightness, whose eyebrow already
  names it.

[main]: src/main.rs
[lib]: src/lib.rs
[machine]: ../quickshell/launcher/Machine.qml
[firmware-rows]: src/firmware_rows.rs
[heat-rows]: src/heat_rows.rs
[power-rows]: src/power_rows.rs
[machine-rows]: src/machine_rows.rs
[rows]: src/rows.rs
[words]: src/words.rs
