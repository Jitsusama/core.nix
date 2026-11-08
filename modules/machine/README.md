# The Machine Daemon

How this machine is doing, held ready for the shell: a Rust program,
`machine`, that reads the kernel's files and asks UDisks for each disk's
health, and answers over `$XDG_RUNTIME_DIR/daemons/machine.sock` in
version 1 of the shell's daemon interface (hello, get, list, watch,
unwatch, health). [`src/main.rs`][main] says how it reads and when;
[`src/lib.rs`][lib] how a record names its figures, with its unit in its
key. The launcher's Machine draws only what it holds
([`../quickshell/launcher/Machine.qml`][machine]).

It reads nothing while nobody watches. Watched, it reads once a second,
and once more a fifth of a second after a read whose rates aren't of now.
On a sixteen-core laptop under load, a read of the processor, memory,
disks and boot took 4 to 7 ms, and of every process 25 to 30 ms (7
November 2026), which is why the programs are
watched only while a part saying them is shown.

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
|                 | kernel release and build                     | Kernel, its Release and Build             |
| `using:cpu:N`   | each program's share, its processes          | Processor's Using It                      |
| `using:memory:N`| each program's resident memory, processes    | Memory's Using It                         |

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

[main]: src/main.rs
[lib]: src/lib.rs
[machine]: ../quickshell/launcher/Machine.qml
