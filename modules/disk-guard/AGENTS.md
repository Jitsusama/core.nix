# The Disk Guard

A Rust daemon that keeps a machine's disks from filling, taking only what
regenerates, cheapest first, and never what a build or a person is using. It
runs twice: `disk-guard system`, a root service that owns what lies outside
every home, and `disk-guard user` in each account, which owns the home.
[`nixos.nix`][1] and [`darwin.nix`][3] fill in the machine's config, which
[`system.nix`][4] declares and renders for both, and run it under systemd
or launchd, and [`home.nix`][2] the account's.

## How It Decides

- **Bands.** Each filesystem a source or the store lies on is healthy,
  filling, nearly full or critically full by its free space (25%, 10% and 4% by
  default), and a pass reclaims towards the target (35%). A forecast starts a pass
  before the next band arrives. Btrfs pools are one filesystem however many
  subvolumes they hold. A writable subvolume reached through the same mount, as
  the work broker clones each tree, is walked into and reclaimed from as any
  folder is, and one that is itself what goes, as each quest's scratch is
  made, goes whole; a mount, a read-only snapshot and a subvolume inside a
  build or a scratch aren't. A filesystem in an image on another, mounted
  from a loop device to discard, as cargo's build disk is, frees its host as
  it frees itself: a pass reaching for room on the host is offered what the
  image holds too, by the host's band, and syncs the image before it
  measures (`backing.rs`). One that doesn't discard frees only itself.
- **Order.** Below healthy a pass takes, in turn: waste (orphaned targets,
  oversized logs, Nix's leftover build directories, and what a killed git
  left staged in a repository no git works on now), Nix generations and
  garbage, idle build variants and cold scratch, producers' own collection,
  caches cheapest to refill first (from their own band on, and before it
  once nothing has written to one for a week), idle targets and installs, older builds a
  newer one of their crate or unit replaced, and last what is occupied. Waste,
  which costs nothing to lose, goes largest first; the rest of a group
  goes longest idle first. While
  healthy, routine upkeep takes what's due every six hours: on the charger or a
  battery over half, ten minutes after boot or resume, however busy the
  machine, since a workstation that's always busy would otherwise never
  be tidied. Only an overloaded one, its tasks stalled on the disk, memory
  or the processors 75% of the time, holds upkeep and the store's routine
  listing until it recovers; a disk past healthy is answered all the
  same, its listing at idle priority until the disk is nearly full. It comes
  at once, too, when an owner ends: the guard looks each minute, and a
  tree newly spent, a new boot, a guard with new
  rules or a path a producer watches saying something new since it last
  ran makes upkeep due, so what an owner leaves never waits hours for the
  routine. What a path says is a link's target, a file's length and
  start, and a folder's names three levels down, not when it was
  written, since a tool that rewrites its links unchanged each round
  would otherwise set off a walk of every source each time. A producer
  whose watched path said something new runs in that upkeep whatever its
  schedule, as a cache gaining a version ends an older one's use. Upkeep
  that kept something only the system daemon could have vouched for,
  while it couldn't be asked, as when a switch restarts both daemons,
  comes again ten minutes later.
- **Windows.** How long a thing has to sit idle before it goes narrows with
  each band, and counts only time the machine was awake. Even while healthy,
  waste goes, and so does what nobody has used for long: a build after a
  week, an install after two, a replaced build after three days, a core
  after a week. What belongs to something that has ended goes once it
  rests: a concluded or retired quest's scratch ten minutes after its last
  write, and the builds in every tree its README lists, at once, whatever
  the band. So do the builds in a tree the tree broker's records say no
  running process holds and no session a pi host's roster holds was cut
  for (a restarted host resumes its sessions under a new process, while
  the record goes on naming the old one), unless a quest at work lists
  it (a quest parked on the bench or for someday isn't: its builds are a
  cache). A build in
  a snapshot the broker pinned at a commit, its record shareable and its
  path the build's git top, goes an hour after it was last built in,
  unless a session sits in its tree, whoever holds it and whatever the
  band: nobody edits a snapshot, and the session that cut one for a gate
  goes on long after the gate, so its holder ending says nothing. In such
  a snapshot a crate's older build goes once an hour unused, even while
  the next gate builds, and its incremental sessions go while nobody sits
  in the tree, since rustc's locks keep the one a build uses. The guard
  makes upkeep due the moment the soonest of these passes its hour. A
  `cargo` source given a `budget` (a size, or a share of the disk its
  first root is on) keeps its builds newest first while they fit; past
  it, the least recently used go whatever the band. A build a session
  sits in, or one used within the last 15 minutes, counts but stays, a
  build already spent takes no room, and a build a cargo runs in, which
  holds its lock, is never offered. For the budget alone, a build is
  used when a cargo last wrote it or when the watch on opens last heard
  anything right in its profile or `deps` opened, as every cargo opens
  its profile's `.cargo-build-lock`: a cargo that finds every unit fresh
  writes nothing, so a run of checks over an unchanged crate would
  otherwise look idle between them. Any reader of a profile, such as
  rust-analyzer or a backup, holds its build back from the budget the
  same way, which costs room, never a build in use. Other windows read
  writes alone, so such a reader holds none of them back. The 15
  minutes hold only where this guard's own marks have heard a build's
  folders, as they are now, for 15 minutes of awake time without a
  gap; a folder is told by the file handle its mark was placed through,
  which a folder made anew never shares. The handle and the mark are
  read through one descriptor, so a folder renamed between them can't
  lend its handle to another. A build not marked since the guard
  started, one whose profile or `deps` was made anew since its mark,
  one after lost events, and every build where nothing hears, as on
  macOS, in `plan`, or once the watch or its listener fails, keeps the
  hour, and the guard says how many folders it couldn't mark and why.
  A record it can't parse counts as lost events. So does a failed read,
  after which the listener tries again a second later; five failed
  reads in a row, or a panic, end it. A guard whose listener ended, or a daemon with no
  watch, forgets what it watched and marks nothing more, in its saved
  record too, so it and the next guard keep replaced builds until they
  have watched them afresh. `plan` and `explain` never listen, and read
  what the daemon watched. A
  budget that comes to nothing, as a share of a disk that can't be
  measured does, is none. While the first root's
  disk holds more than the budget, the guard makes upkeep due, at most
  once an hour, so a build stays until the first look after its 15
  minutes. Cargo's shared builds folder held 63 GiB on 17 November, 59
  of it written within a day, which no idle window reached, and 56 GiB
  against its 20 on 18 November, 55 of it used within the hour, so the
  floor came down from an hour. Incremental sessions are most of what a
  one-shot build writes and never reads again, so one-shot runs should
  set `CARGO_INCREMENTAL=0`: gates, councils' builds, and clippy and
  test sweeps. Nothing here sets it; each run's owner does. A session
  iterating on a crate keeps incremental builds, since quick iteration
  is the point of them, and the budget takes them once it moves on. A temp entry
  nothing has changed since the last boot goes whatever the band and
  whatever its name, since whatever made it ended with that boot. A browser profile in the `tmp/`
  of a quest still at work goes once nothing has changed in it for an
  hour: a browser run for one job makes a fresh one and leaves it, and a
  browser still running it keeps it in use. Anything else in that `tmp/`
  goes once nothing in it has been written for a week, a day once the disk
  is nearly full and six hours once it's critically full, unless a process
  works in it: a quest at work may come back for what it left, but on one
  laptop those folders held 124 GiB, a quarter of it a week unwritten.
  What such a quest keeps beside its `tmp/`, its labs, clones and runs,
  goes the same way after a week, two days once the disk is nearly full
  and twelve hours once it's critically full, but for its handover and a
  folder of GC roots, whose links keep what running programs read. A
  quest a live session works on keeps all it left beside `tmp/`, and in
  `tmp/` what it wrote within two days awake: on 16 November a laptop's
  guard took 188 entries of a working session's `tmp/`, a script it wrote
  that evening among them. Past two days with nothing changed anywhere in
  it, however deep, a live quest's `tmp/` entry goes by the same windows,
  unless a process works in it or it is or holds a `handover`, a
  repository (git, bare or not, jj, Mercurial, Sapling, Pijul, Subversion
  or Fossil), a link into the store, a folder or entry that can't be read
  or another filesystem mounted, at any depth, since the guard can't ask
  whether a clone's commits are on a remote. It's dated by each entry's
  later of its modification and change times, as an archive unpacked or
  a tree copied keeping its times carries old modification times but new
  inodes. One past 100,000 entries is kept, as it can't be read whole, and
  a survey reads a million entries of all live quests at most, keeping
  the rest for a later one. Each is read whole again just before it goes,
  as a pass can reach it hours after its survey. Two days unchanged isn't
  two days unread: the disk keeps no access times, so a lab only read is
  dated by its last change. When the sessions folder can't be read, every
  quest is held to this rule. On 18 November
  quests' `tmp/` entries two days unwritten held 38 GB on one laptop, kept
  while their sessions lived. Its browser profiles at rest
  go as any quest's do. A session lives while its process runs as the
  one that started then, told by its start ticks on Linux and by the
  second it noted on a Mac, or while a pi host's roster holds it.
  A `store-roots` source names links a tool keeps to hold its builds from
  collection, as a development shell keeps its environment: those under
  its roots that Nix's indirect roots list and its pattern matches go once
  the place its pattern names has gone the source's window unwritten, six
  hours at most once the disk is nearly full, unless a process works in
  their tree. `src/plan.rs` holds the table.
- **Use.** Before and after it moves a target aside, the guard judges whether
  any process uses it: a build holding cargo's or rustc's locks, a process
  with a file, map or working directory beneath it, or build variables
  naming it. Busy stays; a target only sat in waits for critically full; a process
  it can't read keeps everything it might use, unless the system daemon
  vouches for it or it has exited since the scan, as most of a build's do
  within a second. An entry of Go's build cache isn't judged: Go trims it
  by age alone, and a process reading one keeps reading it once it's gone.
  Judging each cost two scans of every process.
- **Measuring.** A pass reclaims a batch, lets it settle and measures it. A
  group whose batches free under a tenth of what they predict, three times
  running, is left for the next group; a batch predicting under a gigabyte
  measures whatever is writing, so it neither counts nor teaches. A pass
  that falls short rests before the next, doubling from a minute to fifteen
  while filling, five when nearly full and one when critically full. One
  that kept things only for want of the system guard's word ends its rest
  as soon as the system guard answers again, once in a row. A survey counts the
  blocks its candidates share once, for the first it measures, as trees
  cloned with their builds inside share them: what it predicts is what
  deleting them all frees, unless something it didn't measure shares them
  too. Counted per build, five builds sharing one copy predicted 194 GiB
  and freed 56.
- **Pacing.** A busy machine never stops a pass; deleting slowly is how it
  spares the machine. Btrfs frees what's unlinked only when it commits, and
  saves wait behind a commit carrying a big delete, which no I/O priority
  reaches. So upkeep and a pass on a filling disk rest after every 2,000
  unlinks for 30 seconds, past Btrfs's own commit, and longer while the
  guard's probe, a page beside its state synced once a second, comes back
  slow more than three times in two minutes, until two minutes pass
  without. Each item waits on that strain four minutes in all, then rests
  only its 30 seconds, since strain others make never ends for its
  waiting: one install of 40,000 files held a pass two hours on 16
  November. Before measuring, they wait for Btrfs's own commit rather than
  ask for one, since a commit asked for holds up every save on the disk. A
  disk becoming nearly full ends the resting. Passes on a nearly or critically full disk never
  rest, and ask for their commits. Either keeps the status file's
  heartbeat, resting or not: a step that blocks, such as a commit waited
  for, a command, a Nix deletion or the system daemon's answer, runs on a
  thread of its own while the pass beats, since the desktop calls a guard
  silent for two minutes dead. `src/pace.rs` holds it; `pacing` in the
  config sets the batch and the rest.
- **Stopping.** A daemon takes its stop signal between steps, and a
  waiting stop ends a rest and starts no new command. A survey only
  reads, and its walks take minutes on a full home, so it asks as it
  walks whether a stop waits and ends there. Its step then takes
  nothing, since a checkout it never reached would look gone. One held a
  switch's stop for 35 s before. A step that doesn't ask is left mid-way
  once a stop has waited 500 ms (`leave_when_held_up` in
  `src/serve.rs`), which is as safe as systemd's kill at 1 s, since
  state is written beside and renamed into place.
- **Removal.** Everything is reached through directories opened beneath the
  source's root, never through a symlink, another filesystem or `..`. A
  target is renamed into a trash beside it, judged again, then emptied; a
  disk too full to rename deletes in place. Each step is in the journal,
  stamped when it was done rather than when its pass began, since a pass
  can run for hours, and a guard starting after a crash empties what was
  begun. `disk-guard
  journal` shows the account's guard and the machine's together, by time,
  each line saying whose: the machine's guard owns the store and what lies
  outside every home, so the account's alone never said where most of a
  disk's room went. A pass's line says what set it going (`reach`,
  `upkeep`, `garbage`, `producers`, `clear` or `collect`), how many things
  it was handed, how many of those its windows made due and it judged,
  each once, taken, kept or failed, and how long the survey that offered
  them took, which its own seconds leave out, or null when none did. Each
  pass a survey served carries its seconds, so passes at one time and the
  same seconds shared one walk. Judged reads beside the line's end: a pass
  that ended `exhausted` having judged none had nothing due, and one that
  judged some and freed nothing kept or failed all it judged, which its
  action lines tell apart. A reach that ended `reached` having judged none
  was at its target already, and a stopped or failed pass may end before
  judging any. In the day to 03:00 on 21 November, half the passes freed
  nothing, and their lines couldn't say which. Offered is what that pass
  was handed, so it differs by why: a clear is handed what its ask holds
  that a clear takes; garbage alone, its batches, but beside another
  disk's survey every offer on the store's disk; and an image's own pass,
  its builds less those a disk reaching for room tried earlier in the
  step. A reach's band is the band it reaches for, so a forecast reach
  on a healthy disk reads `filling`. A survey's seconds take in the system
  daemon's answer, up to ten seconds a try, and the git and
  finished-predicate commands as well as the walk, so a jump of about ten
  seconds is likelier a wait than a slow walk. A garbage pass counts as
  the routine, so it puts off every disk's next six-hourly `upkeep`. The
  table shows what
  set a pass going; `--json` gives the counts and its survey's seconds.
  `upkeep` covers every trigger of the routine, among them its own
  retries: ten minutes after one that kept what only an unheard system
  daemon could vouch for, and at the next spare step after one left
  unfinished. A producers' or clear's survey that offers nothing makes no
  pass, so its walk leaves no seconds. Any account may pass
  through the machine's state folder to its journal but not list it, and
  every other file there is saved as root's alone.
- **Replaced builds.** The account's guard marks every target's `deps` with
  fanotify and keeps when each artifact was last opened in `uses.json`, since
  under noatime nothing on disk says. A replaced build is an older build
  beside a newer one: of its crate, for a library, or of its unit, for an
  executable, which is its package and target as its fingerprint names
  them, so a binary and its tests, or two packages' tests of one name,
  never replace each other. An executable with no fingerprint is never
  offered. Once a disk is filling, one goes once nothing has opened it for
  a day and an hour. In a `cargo` source given a budget, an executable a
  newer build of its own variant replaced goes, whatever the band, once
  this guard has heard its `deps` for an awake hour and nothing opened it
  in that hour: on 18 November a gate's build held 28 GiB of test binaries
  a newer build had replaced, nearly all one to six hours old, and filled
  its disk hours later. The variant is the compiler, features, target,
  profile and mode, kind, flags and config its fingerprint names, so there a
  test built two ways keeps each way's newest. In a budget a library never
  goes by the hour: on 19 November 12 of the 35 units built three or more
  times here held live builds differing only in how a dependency was
  built, which nothing tells from a replacement, and every crate above a
  library builds from it, where a live executable so taken costs only the
  link its next run makes. A pinned snapshot's older builds, of either
  kind, go an hour unused. A build with a file cargo shares with its
  profile, as the binary a person runs is, never goes, since taking it
  frees nothing. Each goes
  with cargo's locks held, and only while no process works in its target
  by its cwd, files, executable or maps, and no builder sits in its
  checkout or names it, since a cargo starting up holds no lock yet; a
  build running there keeps them all until it ends. A
  shell or editor that only sits there keeps nothing, since nothing builds
  from an older build. A `deps` never watched dates nothing. Each pass that
  took any journals a `replaced` line per target, saying how many went and
  their bytes.
- **Freezing.** Once critically full with nothing left to take, the account's guard
  freezes the process that wrote more than half of the last minute's writes,
  at a mebibyte a second or more, with a file open on that disk: it and its
  descendants move into a scope of their own, which is frozen. Never the guard,
  nothing in a session, nothing of another account's. It thaws once the disk
  is no longer nearly full, or by `disk-guard thaw`, after which it's never frozen
  again.
- **Nix.** The system daemon lists the dead paths every six hours in a spare
  moment, and on a filling disk also half a minute after a root leaves and
  two minutes after a process holding paths ends, since a listing of a
  large store takes a core for minutes. A listing that expired generations
  or found its batches stale brings another, though on a healthy disk not
  until the last has rested nine times its own length, a rest a restart
  keeps, so listing takes at most a tenth of its time. It lists with no
  experimental features, since content-addressed derivations made Nix parse
  and hash every derivation it met, five times the CPU; one that only a
  realisation keeps is listed dead, and Nix refuses it when it's deleted.
  The garbage's own pass, which comes once something let go of some, rests
  after one that left every path it was offered, two minutes doubling to
  half an hour until one takes some, since an evaluation registering the
  same paths again brought 16 passes freeing nothing in eight minutes.
  What's left is judged by the paths still there, not by the room
  measured, which other writers move, and garbage that pass wasn't
  offered, as a root's leaving lets go, still goes at once.
  It deletes them oldest first,
  in a pass of their own once something has let some go. While the disk is healthy it keeps what
  was built in the last hour, up to 5% of the disk, unless a machine gives
  its own `garbage.keep` and `garbage.budget`. A path a running process
  holds, as a temporary root, a file it has open, or a name in its command
  line or environment, and every dead
  path it reaches, is left for a later pass: Nix refuses a whole delete for
  one live path, deleting none, and each refusal costs it a whole look for
  roots. A built output goes no later than the dead derivation that built
  it, since Nix keeps a derivation while its output is there. Temporary
  roots are read again just before each delete, since an
  evaluation begun after the batch was made holds what it writes for
  minutes. A path written since the listing began, as its ctime says,
  is in no batch until the next listing dates it: the listing keeps what
  was deleted, and an evaluation that wrote it again would lose it by its
  old date. Once a batch is refused anyway, the store is listed again and
  none of the old listing's batches goes to Nix meanwhile, not even the
  rest of that pass's, since builds root again what a listing called
  dead, within minutes of it. It expires
  generations, never the current, running or booted one, the
  newest five, or one with a boot entry. It links the store's identical
  files weekly in a scope of its own at idle priority, only while nothing
  stalls on the disk or memory, since it can wait without cost, stopping
  when the moment ends. It runs only in the night's hours, 01:30 to 07:00
  unless `dedupe.hours` says otherwise, and stops when they end: a run
  reads every file not yet linked, and on a disk with no I/O scheduler its
  idle weight holds back nothing of that. An account's guard on the same filesystem asks it to
  collect rather than collecting itself; with no system daemon answering, it
  collects with `nix store gc --max` once nearly full.
- **Footprint.** A file's size and blocks are what it holds once read; on a
  compressed disk it can take a third of that or less, as Go's build cache
  does. Only root can read what each run of a file takes, from the extent
  items in the filesystem's tree, so `disk-guard footprint PATH...` asks the
  system daemon. It answers only for paths the asker owns or lies beneath,
  resolved first and then opened refusing any link, and walks them without
  following a link or leaving their subvolume, one walk at a time and at
  most two million entries. Each run counts once, but a run shared with a
  file outside the paths counts too, so what clearing frees can be less.
- **Status.** Each daemon keeps a status file, the account's in its
  runtime directory and the system's in `/run/disk-guard`, which the
  desktop and `disk-guard status` read. A thread of its own writes it
  again every 30 seconds for as long as the daemon runs, from its first
  moment, on what the last daemon left until it has its own, each stamped
  as it's written: so the heartbeat says what the unit's state does, and
  neither a restart nor a long step reads as a guard that stopped. The
  desktop asks the unit before it says one has. Its second version says
  things in sentences the desktop shows as they are: one for the machine,
  one for each disk beside its size, use, fill an hour and when it fills,
  and one for each of the last 20 things cleared, which the journal
  remembers across restarts. `src/words.rs` writes them. The fill and when it fills
  are the trend over about two hours, told once it's watched ten minutes
  and while the disk is filling now too, since builds writing and freeing
  gigabytes every few minutes swing the two-minute rate passes plan by
  either way by tens of gigabytes an hour. A disk whose last pass, in its
  band or a worse one, found nothing more to clear is `exhausted`, and its
  sentence says so rather than that the guard is clearing. While a disk
  is below healthy, or holds more than the config's `budget` (a size or a
  share used, which takes nothing more), `holders` says what holds the
  space: the ten biggest folders the sources name, each straight under a
  root or a cache whole, and each folder in the config's `holders.roots`,
  with whose it is (a quest, a worktree, a cache, containers or other),
  whether the guard may clear any of it and, when not, why. Sizing them
  reads millions of entries, so a thread at idle priority does it at most
  every half hour, and while the disks' use has moved less than a
  gibibyte since the last, only every six hours; `holders_at` says when.
  A clear measures again at once. `src/holders.rs` holds it.
- **Wear.** The machine's guard on Linux also says how hard each disk
  is written and how worn it is: what it wrote since boot, from the
  kernel's count at every step, and over the last hour awake, once ten
  minutes are watched; and for an NVMe disk its health log, which only
  root may ask for, once an hour awake, with the share of its rated wear
  used and what it wrote in its life. From those, once a percent is worn
  and the boot is an hour old, it says when the disk reaches its rating
  at the boot's pace, a quarter either way at 2%, since the disk says
  its wear in whole percents, and past its rating says so instead. Each
  ask replaces the last answer, so a disk that stops answering says no
  wear rather than an old one. Only disks that can be written are read,
  which leaves out an empty slot, an optical disc and an eMMC's boot
  areas; under native multipath the health is asked of the first
  controller the subsystem holds. The unit lets the guard open the NVMe
  controllers to read, beside the clock's devices ProtectClock allows,
  and the VM test proves the ask answers through it. A laptop's disk wrote 4.2 TB in 16 hours
  of one boot on 18 November, nearly all of it builds, about a year from
  its rating at that pace, and nothing said so. The counts move every
  step, so they're written with the status each half minute rather than
  being news; a disk coming or going, or its health read, is.
  `src/disks.rs` holds it.
- **The ledger.** Each measurement of the holders goes in the state
  folder's `ledger.json`, kept two days. A holder's `grew` is how much it
  grew since the earliest measurement within the last day that's at least
  an hour old, said in its sentence from a gibibyte, and one that grew
  5 GiB or more is listed beside the ten biggest, so a leak shows by name
  before it's big. A folder that measurement didn't size isn't said to
  grow. `src/ledger.rs` holds it.
- **Keeps.** `disk-guard keeps FOLDER add REASON...` tells the account's
  guard never to clear anything in a folder, or holding one, for Joel's
  reason; `disk-guard keeps FOLDER remove` lets go. `disk-guard keeps` lists
  every keep, and `disk-guard keeps FOLDER` names the one covering it. The
  words before, `keep FOLDER REASON...` and `unkeep FOLDER`, still answer,
  offered nowhere. They live in the state folder's
  `keeps.json`, read afresh at each survey, so a keep holds from the next
  step. A holder a keep covers says his reason, and carries it as `kept`,
  while `clearable` and `clears` stay what the guard judged of it. The
  command marks the status file's holders itself before it ends, and
  every writer of the file marks what it writes the same way, so the
  holder has changed by the time the answer is heard. A machine declares
  its own keeps in the guard's config, `keep`, a reason by path: the
  daemon writes them into `keeps.json` marked `declared` as it starts,
  takes back those the config stops naming, and leaves a hand keep on
  the same folder as it is. A survey counts them even before a daemon
  wrote them, so a plan does too, and `remove` refuses one, since only
  the config lets it go. `src/keeps.rs` holds it.
- **Clears.** `disk-guard clear FOLDER` asks the account's guard to clear
  what it may of a folder now, without waiting for its band or for it to
  sit idle. Each ask is a file of its own in the state folder's `clears/`,
  so two never race, and the next step takes it: a pass of the survey's
  offers inside the folder that `plan::clears` allows, which is what a
  critically full disk would take, less anything occupied and the store's
  garbage, each judged for use again as it goes and paced by the disk's
  own band. That survey reads only the sources lying inside an asked
  folder or holding one (`survey::within`), so a clear starts in seconds
  rather than after a walk of every source, and it leaves what the
  holders say of the rest as the last whole survey found it. An ask ends
  with its pass, whatever the pass did, and until
  then the holders it touches say `clearing`. A holder's `clears` counts
  the same set, so what a clear offers to free is what it takes, and a
  pass drops what it reclaimed from it at once. The command refuses a
  kept folder. `src/clears.rs` holds the asks.
- **Notices.** The account's guard shows one desktop notice, and only once
  a disk is nearly full: a filling disk is the launcher's to show. The
  exception is the guard itself failing: a pass on a filling disk whose
  tries failed and freed nothing, or where one timed out, is told once,
  with the failure's error line, and written to the system journal as an
  error, since a guard that can't clear is otherwise silent. The status
  keeps that pass as `failed` until a pass on that disk frees some, or
  finds it no longer filling, whatever passed between; that pass takes
  the notice away, a frozen writer's or a nearly full disk's notice going
  back in its place, so it says how the guard stands now. A folder its owner wrote again
  while the guard emptied it is kept, not failed. A notice the
  desktop didn't take, as when the notification server is still starting,
  is told again each step until it is. What stands to be told now, once
  a disk is filling, is the status file's `notice` too, for whoever looks after
  the machine. `src/notices.rs` holds both.
- **A Mac.** The same daemons run under launchd, asking macOS through
  `crate::sys`, which is `linux.rs` or `macos.rs`. Nothing hears who opens a
  file there without Apple's entitlement, so no older build is dated or
  offered; nothing freezes a writer yet; there's no scope for a linking run;
  and Time Machine's local snapshots are thinned once nearly full.

## Where Things Live

| Path                      | Holds                                                    |
| ------------------------- | -------------------------------------------------------- |
| `src/config.rs`           | the config each daemon reads, and what it refuses        |
| `src/band.rs`, `decide.rs` | bands, the forecast and the moment                      |
| `src/plan.rs`, `pass.rs`  | the order, the windows, and a pass that measures as it goes |
| `src/pace.rs`             | resting between batches, by the disk's own feel          |
| `src/survey.rs`           | each source's candidates                                 |
| `src/store_roots.rs`      | the links a tool keeps its builds by, found through Nix  |
| `src/surveyed.rs`         | the last pass's survey, kept for `disk-guard plan` to read |
| `src/holders.rs`          | what holds each disk, and whose it is                    |
| `src/disks.rs`            | how hard each disk is written, and how worn              |
| `src/keeps.rs`            | the folders Joel keeps, and why                          |
| `src/ledger.rs`           | what held each disk over two days, and what grew         |
| `src/clears.rs`           | the folders Joel asked cleared now                       |
| `src/anchor.rs`           | removal that never leaves its root                       |
| `src/footprint.rs`        | what files take on a compressed disk, read as root       |
| `src/activity.rs`, `locks.rs` | whether something is in use                          |
| `src/host.rs`, `linux.rs`, `macos.rs` | the machine the guard runs on, through `crate::sys` |
| `src/procargs.rs`, `pmset.rs` | what macOS says, parsed where Linux's tests reach it |
| `src/daemon.rs`, `serve.rs` | a daemon's step and its loop                           |
| `src/socket.rs`, `vouch.rs` | what the two daemons say to each other                 |
| `src/garbage.rs`, `generations.rs`, `dedupe.rs` | Nix's garbage, generations and linking |
| `src/replaced.rs`, `opens.rs`, `unwatched.rs` | older builds, and the watch that dates them, or a Mac's lack of one |
| `src/writers.rs`, `freeze.rs`, `freezer.rs` | who writes most, and freezing them through systemd |
| `src/desktop.rs`, `notifier.rs` | notices and power saving over D-Bus, or on a Mac |
| `src/words.rs`            | the sentences the status file carries for the desktop     |
| `src/cli.rs`, `cli/`      | the command line: reads, acts and the two daemons        |
| `tests/guard/`            | the scenarios, one file per module                       |
| `tests/*.sh`              | the check of `merged-on-github.sh`, over a stand-in gh   |

## The Command Line

It takes the shape every command of ours has from the terminal kit
(docs/commands.md): `--help`, `--format text|json|jsonl` (`--json` for short)
and `--color` anywhere before `--`, and a failure in three parts on stderr, as
`error/1` under `--json`. A bare `disk-guard` reads the status. Each answer
under `--json` names its schema, `disk-guard.<command>/1`, and holds facts in
units its keys name, never a sentence; `journal --format jsonl` streams one
`entry` line each. The machine daemon runs `keeps FOLDER add`, `keeps FOLDER
remove`, `clear` and `thaw` for the launcher's disk piece with `--json` and words Joel's sentence
itself, so the text for a person may change without changing the shell.

Its exits are the kit's: 1 when it couldn't be done or a footprint couldn't be
read, 64 asked wrongly, 65 a file `check-config` refuses, 66 a file or folder
that isn't there, 69 no guard to ask, 75 a daemon already holding that state,
and 78 a config that can't be read or is the other daemon's.

## Working on It

`direnv` enters the flake's development shell in this directory, with cargo,
clippy and rustfmt, and points `TMPDIR` at the account's runtime tmpfs.
`cargo test` runs every scenario in about ten seconds there, and the package
runs them again as it builds, on `/dev/shm`. On Btrfs instead, every pass a
scenario steps would commit the machine's own disk, or wait for its commit.
`nix fmt` from the repository root formats the Rust as it formats everything
else.

- Write a scenario's test first, against the crate's public surface, and see
  it fail on its assertion before making it pass.
- Before trusting a test of a branch already written, break the branch and
  watch the test fail.
- A scenario never reaches the machine's own Nix, systemd or desktop. Any
  config a test steps a daemon with names a fake or missing `nix.bin` and
  `nix.store`, and a system config a fake `dedupe.systemd_run`: the defaults
  name the real ones, and once ran a real `nix store gc` from a test. A
  stub never falls back to a program by bare name either, since `PATH`
  finds the real one: a stub of an unconfined run once ran the real
  `systemd-run`. After
  adding anything that runs a program by default, point the default at a
  script that logs its calls and run every scenario: nothing may call it.
- A test that waits on a thread to show nothing happened gives it time to,
  and fakes are scripts that refuse an argument they don't expect before
  doing anything.
- The package builds in Nix's sandbox, at the idle scheduling policy and on
  a disk other builds are filling, so a scenario expects the policy it runs
  under and never the real disk's trend.

[1]: nixos.nix
[2]: home.nix
[3]: darwin.nix
[4]: system.nix
