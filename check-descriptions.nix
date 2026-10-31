# What each check proves, by its name, as a sentence's predicate: checks.nix puts each in its
# check's meta.description, and every-check-says-what-it-proves fails on a check with none or
# one with no check. A check added names what it proves here, in its place by name.
{
  a-bluetooth-keyboard-may-wake-it = ''
    a machine with Bluetooth lets a USB or PCI controller of Bluetooth's class wake it, so a paired
    keyboard's key does, in a rule udev reads without a complaint
  '';
  a-claimed-chord-is-refused = ''
    a chord two layers claim, or one claims twice, is refused
  '';
  a-failed-boot-says-what-stopped-it = ''
    a failed boot's shell names up to four failed units and their last lines inside 80 columns, and
    the way out, and nothing when none failed
  '';
  a-faint-theme-is-refused = ''
    a theme with text under WCAG's 4.5:1 doesn't build
  '';
  a-firmware-update-frees-pcr-0-only-for-fwupds-entry = ''
    the shutdown step takes PCR 0 out of the TPM's policy only when BootNext names fwupd's
    firmware updater, and leaves the policy alone for any other BootNext or none
  '';
  a-long-name-reads-at-one-pace = ''
    a selected name too long for its row slides at one reading pace, gathering speed from rest and
    settling back to it in at most a fifth of a second at each end, never stepping back
  '';
  a-low-charge-is-said-once = ''
    a battery that reports only its charge runs low at its mark and critical at its own, only ever
    deeper until it charges or rises five points past the low mark, so a charge wavering about a
    mark is never said twice
  '';
  a-low-device-is-its-own-notice = ''
    each connected Bluetooth device reporting a battery stands by its own charge, and each running
    low is a thing of its own, keyed by its address, signed by its name and drawn by its kind's
    glyph, wanting Joel now only at the critical mark
  '';
  a-mac-sudo-takes-touch-id = ''
    every Mac's sudo lets Touch ID answer it, in the file macOS updates keep
  '';
  a-notice-comes-forward = ''
    once a notice is opened, its sender's window comes forward, the one used last, unless one of
    its own is in front already or it has none open
  '';
  a-notice-is-one-thing = ''
    a notice's card and its row among the notices list the same acts in one order, copying last,
    and a notice gone from the screen keeps none of its sender's
  '';
  a-prompt-waits-for-a-person = ''
    a D-Bus call from the account's shells, as run0 asks for polkit's prompt in, outlasts systemd's
    25 seconds
  '';
  a-quiet-file-says-nothing = ''
    Quickshell's FileView, told not to print errors, says nothing of a file that opens and refuses
    every read, as one not told still warns
  '';
  a-realms-link-key-stays-apart = ''
    a realm's link key is served on its own socket by an agent holding only it, kept outside the
    folder the account's own agent loads, made with no PIN, and an account naming no realm has none
  '';
  a-piece-may-stand-for-several = ''
    a piece standing for several things, as the Bluetooth devices' batteries do, is a fact for
    each in the overview's order, each known by its piece's mark and its own key, and none while it
    stands for none
  '';
  a-refused-profile-is-said-once = ''
    a power profile the daemon refuses is said once as a warning, tried again at each change of the
    battery without repeating it, and said once it takes
  '';
  a-sender-keeps-to-its-level = ''
    a notice reaches no further than its sender is kept to, however critical, and each level shows,
    counts and stays as it says
  '';
  a-session-is-known-by-its-own-record = ''
    pi's sessions are those whose records name a process running on this boot since when they
    say, never found by a window's title, each with its window found through the kitty it runs
    in, and gone once its record or its process is; and those to reopen are the last boot's until
    put off, then those another machine carried here, newest first, never put off
  '';
  a-theme-changes-everything = ''
    another theme leaves none of the first's colours behind
  '';
  a-trusted-cable-finds-machines = ''
    a trusted cable's network is a stranger's cable with mDNS on, matched by its card and picked
    first, and a machine naming none gets no such network
  '';
  a-watched-file-comes-back = ''
    the shell's status file hears a status once its folder is made, after the shell starts and again
    after it's removed, and says nothing while it's gone
  '';
  agents-work-runs-without-real-time = ''
    the jobs sorter takes real time from work under jobs.service, jobs-work.slice and
    jobs-timed.slice, and from nothing that holds Joel's own terminal work
  '';
  an-arrival-is-said-once-each-time = ''
    what comes true by itself is said once, and nothing of what stood as the shell started; a
    piece that tells something new is said again, its own earlier notice taken back first and no
    other's; and one that goes takes back whichever of its notices is up
  '';
  an-ipc-reply-arrives-whole = ''
    a Quickshell IPC reply of a million characters reaches `qs ipc call` whole, and a call to a
    function that isn't there ends qs with a failure
  '';
  backlight-passes-clippy = ''
    clippy finds nothing in the backlight daemon or its tests
  '';
  bat-reads-its-theme = ''
    bat colours code in the account's theme from a cache built once, saying nothing, delta finds
    the theme through it too, and a switch builds no cache of its own
  '';
  batteries-leave-time-to-power-off = ''
    UPower powers a laptop off at 5%, before it goes flat
  '';
  bluetooth-firmware-reaches-the-disk-prompt-dell-xps-14-da14260 = ''
    the XPS 14's Bluetooth firmware for the disk prompt is in the machine's firmware
  '';
  bluetooth-firmware-reaches-the-disk-prompt-microsoft-surface-laptop-4-1958 = ''
    the Surface Laptop 4's Bluetooth firmware for the disk prompt is in the machine's firmware
  '';
  bluetooth-keeps-its-power-limits = ''
    the XPS 14's kernel reads Bluetooth's revision 2 SAR table and antenna gain setting from UEFI
  '';
  bluetooth-lets-a-hibernation-power-off = ''
    the XPS 14's kernel thaws Bluetooth's chip after a hibernation's image is made with a plain
    wake rather than a reset, so the power-off after finds it answering and the hibernation goes
    through
  '';
  bluetooth-links-passes-clippy = ''
    clippy finds nothing in bluetooth-links or its tests
  '';
  bluetooth-links-reads-the-replies = ''
    bluetooth-links reads the kernel's lists of adapters and of the connections that are up, with
    who leads each, who started it and how it's secured, and each adapter's address and byte
    counts, writes each HCI command as the controller takes it and picks its answer out of the
    events, reads the signal, link quality, power, channel map and the adapter's version and maker
    from reply bytes, leaves a failed one null, keeps each link's last twenty reads and each
    adapter's traffic, asks the version once, writes nothing new while nothing is connected, and
    replaces its file whole, readable by everyone
  '';
  bluetooth-pairs-before-it-connects = ''
    the launcher trusts a Bluetooth device before pairing it, follows it past its link coming up
    until it's paired, untrusts one that fails, and connects one paired before; lists a device's
    parts after its verbs, named first: its battery warning low, its signal and link quality as
    sparks of the reads bluetooth-links would write, its channels, connection, an input device's
    facts held by how it's carried, whether it reconnects by itself and wakes the machine as
    settings whose choices set them, and its address to copy; lists the adapter's radio,
    visibility and pairing as settings, its traffic as a spark and what it is; answers how the
    links are doing; and says a device running low at its row's side
  '';
  bluetooth-seal-keeps-only-what-a-keyboard-needs = ''
    the Bluetooth seal puts a paired keyboard's keys and service records on the ESP only sealed to
    the TPM under PCR 7, leaves out devices only seen in a scan, seals unchanged pairings once and
    changed ones again, and takes the keys off once unpaired
  '';
  boot-lock-is-whole = ''
    the splash's lock, at each scale, holds all the ink its glyph has drawn with room to spare, and
    is a line of text tall
  '';
  booted-generation-stays-installed = ''
    a switch never removes the generation the machine booted
  '';
  btop-backs-out-on-escape = ''
    btop's Escape backs out a step, off a process chosen and then out of btop, rather than opening its
    menu
  '';
  btop-draws-in-the-theme = ''
    btop draws in the theme's colours, not its own
  '';
  btop-names-its-window = ''
    btop names its terminal's window btop while it runs, and gives the title back as it quits
  '';
  btrfs-reclaims-its-chunks = ''
    tmpfiles turns Btrfs's reclaim on for data and metadata
  '';
  call-audio-runs-in-real-time = ''
    an app's audio thread can take SCHED_RR 8, below PipeWire's, for at most 200 ms at a time
  '';
  cargo-links-with-mold = ''
    cargo links an account's Rust builds with mold
  '';
  changed-checks-names-what-moved = ''
    changed-checks names a check added, changed, removed or broken by its derivation, fails on a
    broken one, builds those added and changed unless asked only to name them, the VM tests only
    with --build and without --no-vm, compares the system with the most checks unless told one,
    sends a commit origin has to a lent builder to build by itself and anything else, or what no
    builder took, builds here, and evaluates a commit's checks once; where nix-offload takes it, the whole run goes to
    `there --changed` naming this machine's system and a base as its merge base's commit, and it
    runs here with --here, a base origin's main doesn't hold, edits not yet committed, or
    nowhere taking it
  '';
  changed-checks-passes-clippy = ''
    clippy finds nothing in changed-checks or its tests
  '';
  chrome-open-tabs-passes-clippy = ''
    clippy finds nothing in the host that tells the launcher Chrome's open tabs, or its tests
  '';
  chrome-pack-extension-passes-clippy = ''
    clippy finds nothing in what packs the launcher's Chrome extension, or its tests
  '';
  chrome-recent-sites-passes-clippy = ''
    clippy finds nothing in what reads Chrome's sites for the launcher, or its tests
  '';
  chrome-recent-sites-reads-a-history-mid-write = ''
    Chrome's sites are read whole from a history caught halfway through a write, its last whole
    write listed, Chrome's files left alone and no copy left behind
  '';
  chromium-leaves-the-panel-to-niri = ''
    the XPS 14's Chrome, Slack and 1Password leave its panel's colours to niri, which maps sRGB onto them
  '';
  claude-shows-its-status-line = ''
    Claude Code is told of its status line on Linux and macOS, in settings it can still write
  '';
  claude-status-line-says-how-it-stands = ''
    Claude Code's status line shows the folder, the model and the context's size, cached tokens
    counted, in the theme's colours, and the command a workstation gives Claude Code draws the
    folder in the theme's strong colour
  '';
  claude-statusline-passes-clippy = ''
    clippy finds nothing in Claude Code's status line or its tests
  '';
  claude-wears-the-theme = ''
    Claude Code's theme has the account's text colour and only hex colours, and the mix its tints
    come from is exact
  '';
  clipboard-history-passes-clippy = ''
    clippy finds nothing in the clipboard history or its tests
  '';
  clock-follows-passes-clippy = ''
    clippy finds nothing in clock-follows, which moves the clock's zone to where the laptop is,
    or its tests
  '';
  console-font-passes-clippy = ''
    clippy finds nothing in the console's font tool or its tests
  '';
  darwin-role-twice-is-once = ''
    importing a nix-darwin role twice leaves the machine unchanged
  '';
  dell-events-are-known = ''
    the XPS 14's dell-wmi ignores the EC's hourly type 0x0012 event rather than log an unknown key
  '';
  desk-facts-read-the-desk = ''
    desk-facts reads a monitor's EDID and USB-C cable, a USB sound card's modes, a built-in
    card's jacks and chips, a camera's formats, a loopback camera's mode and sensor, a
    Bluetooth input device's link and inputs, and a wired keyboard's or mouse's polling rate and
    inputs as the devices' own report them, and writes the launcher's JSON in order
  '';
  desktop-entries-are-read-only-when-theirs-change = ''
    the shell reads the desktop entries again when an entry is added or a link on the way to them
    moves, and not for a file written beside them, as the home folder's hidden files are
  '';
  device-check-passes-clippy = ''
    clippy finds nothing in device-check, device-check-disk or their tests
  '';
  devices-that-stop-working-are-named = ''
    `device-check` names a failed or missing device, or a disk's fault, once, and again when it's
    fixed
  '';
  dictation-hears-the-clip = ''
    Whisper on the CPU types a public clip back word for word
  '';
  dictation-passes-clippy = ''
    clippy finds nothing in dictation or its tests
  '';
  dictation-says-what-its-doing = ''
    dictation's surface says listening with the time, counts down from 25 s, says typing once it
    lasts, still writing, and why it heard nothing; its wave keeps the loudest peak of each step
  '';
  dictation-stops-by-its-chord = ''
    a chord runs `dictation hold`, so dictation can be held to talk
  '';
  dictation-switches-to-the-new-one = ''
    a switch restarts dictation after reloading its units
  '';
  dictation-works-it-out = ''
    dictation's spectrogram, transform and vocabulary match their references, words turn into
    Whisper's tokens as GPT-2's merges join them; a held chord's release stops only what it
    started, and a tap's none
  '';
  dictation-writes-the-words-it-knows = ''
    Whisper writes a name it misheard as the words file spells it, given the file
  '';
  disabling-a-module-removes-it = ''
    `disabledModules` removes a module a role brought in
  '';
  disk-guard-accepts-its-configuration = ''
    the disk guard passes its scenarios and accepts the configs a workstation and its account get
  '';
  disk-guard-asks-github-what-merged = ''
    the disk guard counts a GitHub checkout finished only once its own head was merged
  '';
  disk-guard-compiles-for-macos = ''
    the disk guard and its scenarios compile for macOS, clean
  '';
  disk-guard-finds-a-machines-programs-first = ''
    a machine's programs for the disk guard come first on its PATH under systemd and launchd, with
    core's git after
  '';
  disk-guard-passes-clippy = ''
    clippy finds nothing in the disk guard or its scenarios
  '';
  disk-guard-stays-in-the-background = ''
    the disk guard yields to the account, and a command overrunning its memory never stops it
  '';
  disks-are-scrubbed = ''
    the disk's Btrfs is scrubbed monthly, from its root
  '';
  disks-go-by-their-name = ''
    a disk's partitions and open volume carry its name
  '';
  disks-say-when-they-fail = ''
    a workstation runs smartd on its disks and hands their faults to device-check, not to every
    terminal
  '';
  do-not-disturb-holds-while-sharing = ''
    do not disturb turns on while the screen is shared, not recorded, and off after; ended mid-share
    it stays off until the next, and one kept by hand outlasts it
  '';
  each-machine-signs-with-its-hardware = ''
    a Linux workstation signs through its TPM and a Mac through its Secure Enclave, both with one
    program and Joel's key by default, and signing alone, with no hardware, signs nothing
  '';
  eero-passes-clippy = ''
    clippy finds nothing in eero or its tests
  '';
  eero-reads-what-eero-says = ''
    eero reads a network's line, speed test, conferencing setting and connected devices from what
    eero's cloud answers, a missing field as unknown, and tells an aged session from other refusals;
    it sends the conferencing setting as a bare boolean, and counts it taken only once eero reads
    it back
  '';
  ergodox-on-every-workstation = ''
    every workstation can flash the ErgoDox, a Mac as much as Linux
  '';
  every-check-says-what-it-proves = ''
    every check says here what it proves, and every description here names a check
  '';
  docs-name-what-exists = ''
    every doc's links and paths name what's there, but the exemptions named
  '';
  every-folder-says-what-it-holds = ''
    every folder says what goes in it, but those still waiting for their note
  '';
  every-machine-answers-local-names-in-time = ''
    every NixOS machine's name service is nsncd answering a .local name as not found once it has
    taken 4 s, inside the 5 s glibc's client waits
  '';
  every-machine-asks-names-in-each-family = ''
    every NixOS machine's name service is nsncd asking for each address family on its own
  '';
  every-program-wears-its-own-mark = ''
    every program the launcher lists wears a mark of its own, its logo or the theme's mark for it
  '';
  example-darwin = ''
    a Mac workstation evaluates
  '';
  example-laptop = ''
    a laptop on the XPS 14, its disk and Secure Boot, evaluates
  '';
  example-nixos = ''
    a NixOS workstation with a screen evaluates
  '';
  files-are-chosen-in-the-launcher-alone = ''
    open-with chooses a file in the shell when it answers, and chooses none when the shell can't be
    asked, never asking GTK's portal
  '';
  firmware-leaves-the-keyboard-light = ''
    the XPS 14 leaves the keyboard light to its key, still holding its other firmware settings
  '';
  firmware-notice-passes-clippy = ''
    clippy finds nothing in firmware-notice or its tests
  '';
  firmware-notice-says-each-release-once = ''
    firmware-notice says each device's newest offered release once, with whether it finishes at a
    restart, says nothing when nothing is offered, and keeps what it said when fwupd doesn't answer
  '';
  firmware-settings-passes-clippy = ''
    clippy finds nothing in firmware-settings or its tests
  '';
  formatting = ''
    everything is formatted and passes the linters
  '';
  gimp-accepts-its-configuration = ''
    GIMP reads its preferences, the theme's palette and colours
  '';
  gimp-takes-its-shortcuts = ''
    GIMP's window reads the declared shortcuts and refuses none of them
  '';
  git-engine-agrees-with-gitstatusd = ''
    git-engine answers as gitstatusd does across a prompt's repository states, counting all and
    stopping at one, under git's monitor too, and as git does where gitstatusd can't read
  '';
  git-engine-passes-clippy = ''
    clippy finds nothing in git-engine or its tests
  '';
  git-settings-stay-writable = ''
    programs write git's settings and the declared ones still win
  '';
  google-opens-its-links = ''
    a meeting's Tuple link joins through Tuple, or the browser without it, Google's own pages,
    documents included, open in a window of their own, any other link in the browser, and what
    isn't a link nowhere
  '';
  google-passes-clippy = ''
    clippy finds nothing in google or its tests
  '';
  google-reads-the-calendar = ''
    google reads the harness's Google account without writing its file, refreshes a stale token in
    memory, is signed out by a refused sign-in until a new one, keeps what was heard while Google
    can't be reached, tells the shell each meeting's times, answer, the one link it's joined by and
    how its series repeats, asking each series only every five minutes and keeping its rule while
    Google fails, and answers an invitation the shell asks it to, changing only Joel's own answer,
    cancels a meeting only when Joel holds it, telling every guest, moves one through the calendar
    Google lets him change it on, telling everyone asked, tells it the mail that wants Joel, newest
    first, each asked for once, with who it went to, who was copied and whom a reply to all copies,
    never Joel, its text, never an attachment's, and each file it carries, by its name, kind and
    size, but what its HTML shows inline, fetching one only when asked, never to open one that runs,
    into a folder only Joel reads or into Downloads, never over a file there, marking it read or
    archiving it when asked, marking it unread again, mail read here wanting Joel again at once, and
    the documents of each kind he looked at lately, his last first, asking Drive again only every
    five minutes and keeping them while Drive fails, makes a Meet meeting for its link when asked,
    only once Joel's sign-in grants Meet's scope, lists a Drive folder only when asked, the last
    few asked each told, folders first, a shortcut as what it points at, leaving out what it
    can't show and never asking of a folder no id names, and downloads a Drive file to open, pick
    or save, Google's own kinds as their office program's file, never opening one that runs, each
    download asked lately told, and a file cut short leaving nothing behind, and sends what Joel
    saves through the picker up to Drive once it's written and left a moment, a new file and then
    new versions of it, never a second file for one it may not replace, saying which program
    saved each as the picker named it, keeping each staged copy and letting go of those sent a
    week ago, with what was kept beside them
  '';
  google-switches-to-the-new-one = ''
    a switch restarts google after reloading its units
  '';
  greetd-waits-for-the-gpu = ''
    the XPS 14's greetd starts once the GPU's device unit, tagged by udev, appears, waiting at most
    15 s, rather than for the boot to go idle as elsewhere
  '';
  gtk-3-dialogs-ask-the-portal = ''
    a GTK 3 program's own file dialog never shows, asks the portal instead, and answers with what
    it chose, or as closed when it's cancelled
  '';
  gtk-speaks-in-the-theme = ''
    GTK's settings, the desktop's dconf and fontconfig's sans-serif all name the theme's sans face
  '';
  guide-compiles-for-macos = ''
    clippy finds nothing in the guide as macOS compiles it
  '';
  guide-passes-clippy = ''
    clippy finds nothing in the guide or its tests
  '';
  guide-proves-its-steps = ''
    the guide counts a step done only once observing it again says so, reads what real machines'
    tools printed, writes sticks that sfdisk, fsck.fat and their own key read back whole, signs EFI
    programs byte for byte as sbsign does, and finds every program it runs
  '';
  hibernate-unlock-lets-go-only-after-the-disk-asked = ''
    hibernate-unlock reads the TPM's counts, keeps them only for sleeps that write an image, takes a
    firmware start for a resume through the disk prompt and a wake for none, counts a disk as
    asking only when every token asks for something typed (a recovery key among them) and names the
    first that doesn't, and rescues a rolled-back hibernation:
    said with the lid open, tried once more with it shut, then powered off
  '';
  hibernate-unlock-passes-clippy = ''
    clippy finds nothing in hibernate-unlock or its tests
  '';
  icon-marks-past-the-basic-plane = ''
    an icon mark past U+FFFF reaches the shell whole
  '';
  idle-pi-windows-yield = ''
    the sorter is given the folder where the harness marks pi sessions nobody is at, and weighs pi
    sessions started outside a terminal as it weighs windows
  '';
  inkscape-accepts-its-configuration = ''
    Inkscape starts a drawing on the theme's desk and reads its stylesheet
  '';
  inlyne-reads-in-the-theme = ''
    inlyne draws a sheet and the keys page its settings name on the theme's background, headings in
    the strong text and none of its own colours, without asking the desktop for the theme its
    settings name, and markdown files open in it
  '';
  inlyne-says-the-file-it-shows = ''
    inlyne keeps the file it shows, read by its Recent's command as its process and whole path, and
    nothing once q closes it, clearing what a killed one left
  '';
  inlyne-says-the-files-it-opens = ''
    inlyne says each file it's started with to the desktop's list of files opened lately, then runs
  '';
  ipu7-outlives-its-open-files = ''
    the XPS 14's capture side is freed when its last node closes, not when the IPU unbinds
  '';
  ir-camera-passes-clippy = ''
    clippy finds nothing in ir-camera or its tests
  '';
  jobs-sorter-passes-clippy = ''
    clippy finds nothing in the jobs sorter or its tests
  '';
  jobs-sorter-sorts-by-behaviour = ''
    the sorter moves a group busy two seconds running, never the shell's or pi's, sends a scope
    started from jobs back there unless it sets limits, and weighs a window low only while every
    pi in it is marked as one nobody is at, waking at once when a mark goes
  '';
  joels-work-goes-before-agents = ''
    the sorter moves a terminal's work inside jobs, above pi's work and below timed work
  '';
  kernel-settings-hold = ''
    the kernel's patches apply and every setting holds
  '';
  kernel-settings-hold-dell-xps-14-da14260 = ''
    the XPS 14's own kernel applies its patches and holds every setting
  '';
  kernel-settings-hold-microsoft-surface-laptop-4-1958 = ''
    the Surface Laptop 4's own kernel applies its patches and holds every setting
  '';
  kernel-settings-hold-with-a-theme = ''
    the kernel holds every setting with an account's theme, its panic colours among them
  '';
  kernel-trouble-is-named = ''
    `device-check` names a kernel that oopsed, locked up, saw a hardware fault or held a task stuck,
    once, and not its harmless taints
  '';
  kitty-accepts-its-configuration = ''
    kitty loads its files without a complaint
  '';
  kitty-draws-the-ligatures = ''
    kitty draws !=, >= and -> as the mono face's ligatures with the account's fontconfig features,
    and |> and 1..=2 plain
  '';
  lag-watch-passes-clippy = ''
    clippy finds nothing in lag-watch or its tests
  '';
  lag-watch-tells-who-waited = ''
    lag-watch counts the desktop or apps waiting as trouble, the desktop's memory stalls only from a
    120 Hz frame's worth of the window, and its waits on the disk or a page being moved from the
    same, niri's turns for a CPU only past 3% of a frame, and work waiting on work as none, names
    the app's unit that stalled most, tells of the desktop's or an app's program the kernel ended
    for memory, names in a word the wait felt most, whose it was and since when trouble has lasted,
    finds the threads it watches by name, of niri only the one its socket names, and sums each
    generation by its load
  '';
  lid-docked-passes-clippy = ''
    clippy finds nothing in lid-docked or its tests
  '';
  linux-builder-lets-in-only-the-keys-it-is-given = ''
    a Mac's Linux VM given other machines' keys lets exactly those in as its builder beside the
    Mac's own, and a VM given none is the very guest nixpkgs caches
  '';
  load-proof-reads-its-signals = ''
    load-proof counts a refresh with the probe's frame ready and none shown as a frame niri missed,
    one that passed before the probe committed as the probe's own, none when a frame lands at every
    refresh, and a frame niri dropped for a later one apart; a key typed into a pty in raw mode
    comes back; a careful save leaves its newest text whole, nothing half written, and no file once
    the probe is done; a STUN request a stand-in server answers is timed and one it never answers
    is lost; each sound node's errors are read from pw-top's last iteration, cameras left out, and
    only those that rose on a running node named; the middle, 95th and 99th percentiles and worst
    of the times are read from them; and a loaded reading within each signal's margin of an idle
    one passes, one past any margin fails saying its figures, and one missing a signal or a figure
    fails; a run fits in five minutes, climbs to twenty builds, reads free memory as the kernel's
    MemAvailable, builds its own source only from the vendored crates, copies that source whole
    and writable, reads the line as the default route's link and what it carried in megabits a
    second, and passes only when every step did and nothing stopped it; and a command given more
    than it takes, `run --help` among them, is refused before anything starts
  '';
  load-proof-passes-clippy = ''
    clippy finds nothing in load-proof or its tests
  '';
  loader-goes-first-in-boot-order = ''
    the boot step gives the loader an entry of its own on the ESP, never taking a past install's
    entry on another partition for it, makes it active and first in BootOrder, and writes nothing
    once it is
  '';
  machines-yield-to-what-the-person-works-in = ''
    system virtual machines are weighed as a build is, and the account's containers and session
    machines are idle
  '';
  memory-modules-passes-clippy = ''
    clippy finds nothing in memory-modules or its tests
  '';
  memory-modules-reads-the-firmware = ''
    memory-modules reads each memory device's locator, size, type, speed, maker and part from the
    bytes the firmware lists, leaves out an empty slot, and reads LPDDR5 past 6400 MT/s as LPDDR5X
  '';
  mona-sans-vf-is-the-whole-face = ''
    the graphical role's fontconfig, asked for Mona Sans VF by name as GitHub asks, gives the variable
    file with every axis and lists no other file under that name
  '';
  mpv-accepts-its-configuration = ''
    mpv plays a moment of video with the account's settings and its controller's without a
    complaint, and video files open in it
  '';
  mpv-says-the-file-it-plays-now = ''
    mpv keeps the file it plays now, read by the launcher's command as its process and whole path, never a stream, gone as it quits
  '';
  mpv-says-the-files-it-plays = ''
    mpv says each file it plays to the desktop's list of files opened lately, a stream as it was given
  '';
  mycelium-links-what-a-machine-names = ''
    a workstation in a realm runs mycelium's node there, dialling each machine it names, as a user
    service on Linux signing in with the agents' key and an agent on a Mac whose sessions sign
    commits with that key; one naming none runs it dialling nothing, and one in no realm has
    neither a realm nor a node
  '';
  mycelium-keeps-each-realm-apart = ''
    a machine joining a further realm runs a node of its own for it, with its own state, mesh and
    steer folders under the harness's per-realm roots and its own link key, apart from the home
    realm's agents key; its pis are told which realm is home
  '';
  mycelium-serves-the-harness-files = ''
    each realm's node serves the harness's files of that realm, its quests and the sessions to
    restore, from the realm's own folders, and the home realm's alone the machine's waiting and
    verdicts, each keyed as the launcher reads it
  '';
  mycelium-declares-what-a-machine-says = ''
    a machine declaring what mycelium's node can't measure (its reach, what a held session
    outlives, whether work costs money, its taints, its agents by realm) has it written where
    mycelium reads it, in mycelium's words; one declaring nothing has no file, and agents in a
    realm it doesn't join are refused
  '';
  needs-you-is-true = ''
    over the agents' own files, Needs You holds now only what Joel can do now, a timed one first
    and then the quickest, puts what he can't yet or has put off under Later, saying what each
    waits for, keeps gone what he or the to do daemon found done, says when nothing watches one,
    keeps nothing of an ended quest and drops nothing by its age, and counts a session's want as
    his only while its quest has something ready or the question is his
  '';
  neovim-binds-every-key = ''
    Neovim binds every key keys.nix names and only those, each action provided, in its scope's
    buffer or plugin window, completion and the command line included
  '';
  neovim-draws-its-status-line = ''
    Neovim's status line names the file from its repository's root, the branch and its unsaved mark,
    a change, an error, the mode, a version and a terminal, and fits 35 columns
  '';
  neovim-finds-a-documents-pictures = ''
    Neovim finds a markdown file's picture by its path relative to the file, for the terminal to
    draw, with ImageMagick there for pictures other than PNG
  '';
  neovim-finds-in-every-group = ''
    every key group in Neovim has its own Space, which lists in one column, titled, the group's keys
    that work here and runs the one chosen, or says none does
  '';
  neovim-floats-by-shade = ''
    Neovim's floats, finder and suggestions have a blank border, not a line, on their shade with
    nothing showing through; a notice starts with its mark
  '';
  neovim-is-named-neovim = ''
    the account's entry for Neovim is named Neovim, not nixpkgs' "Neovim wrapper", and still runs
    nvim
  '';
  neovim-keeps-its-numbers-in-one-column = ''
    Neovim's number column gives the cursor line its own number, marked, as wide as the relative
    counts beside it, and is blank with numbers off
  '';
  neovim-knows-the-names-it-writes = ''
    Neovim spell-checks markdown without flagging niri, Quickshell, YubiKey, TPM or monospace, and
    still flags a misspelt word
  '';
  neovim-says-the-file-it-has-open = ''
    in a kitty window, Neovim says the file it has open to the window, the one it starts with once
    its terminal attaches and the next as it's entered, nothing for help, and nothing once it leaves
  '';
  neovim-says-the-files-it-opens = ''
    Neovim says each file it opens to the desktop's list of files opened lately, as each comes into
    view and going back to one too, and nothing that isn't a file on disk
  '';
  neovim-starts-in-the-theme = ''
    Neovim starts in bamboo, transparent, without a complaint
  '';
  neovim-starts-with-only-what-it-draws = ''
    Neovim opens a file with only what draws its first screen, every key's command standing in,
    loads the rest when used or once drawn, and lua_ls reads only its runtime
  '';
  network-revive-passes-clippy = ''
    clippy finds nothing in network-revive or its tests
  '';
  niri-accepts-its-configuration = ''
    niri loads the files an account gets
  '';
  niri-cursor-has-its-shapes = ''
    the cursor theme niri names has the shapes programs ask for
  '';
  niri-names-each-screen-once = ''
    niri's config names each screen in one output block, the panel's holding srgb-gamut, and refuses a
    second block for a screen, which niri would ignore
  '';
  niri-never-shows-its-own-screens = ''
    no niri key opens its screenshot screen, its key list or a quit that asks, and the launcher's
    Log Out quits without asking
  '';
  niri-says-what-the-gpu-gave-it = ''
    the session's niri lets smithay say which GPU priority niri got, keeping the rest of niri's own
    log filter
  '';
  niri-starts-able-to-go-first = ''
    the session's niri starts through its wrapper, which holds CAP_SYS_NICE
  '';
  nix-builds-yield-to-the-person = ''
    Nix's builds run at a lower weight than the person's programs
  '';
  nix-collects-before-the-disk-fills = ''
    Nix collects garbage during a build that fills the disk
  '';
  nix-offload-decides-when = ''
    a builder is asked for the moment a build or VM check is queued or running, by a queue
    still writing, within the day's hours and never beside one not yet seen gone; it's named
    for big-parallel work alone, offering nothing a VM test needs, so small builds and VM
    tests stay, with its store named for a build sent there whole when the helper gives one,
    and it's named as being made until it stands or won't; it's given back once idle, at the
    deadline or once gone, with a wait after a failure or one given back idle while work waits,
    and none after one given back in use;
    only builders it asked for are ever given back; a stop ends at once, leaving its builder
    for the next start, which takes it back before the deadline, keeping its price and idle
    time, or gives it back, priced to then or its own idle delete; standing builders are
    named while they answer, their names asked of resolved itself so one gone can't hide
    the rest, and only to builds that ask for them, never in the file every build reads,
    while builders beside are named in that file while they answer, for their big work
    alone, and never with the standing ones; a
    day's journal sums to its hours and cost, each at its price; and `there`
    sends a flake's build to the lent builder as one command, returning its status and
    journalling what ssh sent, or, when none stands or none answers, builds nothing and exits
    75, saying so, unless --here asks it to build here; a tree's uncommitted changes go as a
    commit of what its index holds, files staged or added with -N included and untracked ones
    left, with the branch and index untouched;
    `there --changed` runs a tree's own changed-checks on the lent builder, else only
    evaluates on the first evaluator that answers and takes the repository's owner, and
    builds what it names as `there` does, exiting 69 rather than handing it back when no builder
    takes it, else tells changed-checks to run here
  '';
  nix-offload-passes-clippy = ''
    clippy finds nothing in nix-offload or its tests
  '';
  nixos-role-twice-is-once = ''
    importing a NixOS role twice leaves the machine unchanged
  '';
  no-name-says-a-replaced-word = ''
    no check's name and no jitsusama.* option's says a name docs/glossary.md replaces, but the
    other senses and the renames still waiting that tests/names-kept.nix keeps, each printed with
    its word, and the check refuses a stand-in name breaking each of its rules
  '';
  nothing-names-its-own-look = ''
    no module but the theme's names a colour or a face
  '';
  nothing-spells-a-mark = ''
    no module but the theme's spells a mark's code point instead of naming it
  '';
  now-lists-by-group = ''
    Now lists what wants Joel first, then what reaches other people, what's playing and the modes
    left on, by name within each, and a group it doesn't know last
  '';
  one-niri = ''
    the screens go off through the machine's patched niri, not a second one
  '';
  one-ruleset-for-every-program = ''
    the firewall, libvirt and Podman share nftables, and a reload flushes none of them
  '';
  one-ssh-agent = ''
    a workstation with a screen answers SSH with the TPM only
  '';
  onepassword-offers-its-quick-acts = ''
    the account's 1Password entry offers Quick Access and then Lock 1Password, through switches the
    installed 1Password takes
  '';
  onepassword-works-with-chrome = ''
    1Password has its Chrome helper and unlocks with polkit
  '';
  only-external-screens-reach-the-seat = ''
    ddcutil's rules give the seat a DisplayPort connector's AUX bus and an MST branch's, and keep
    the panel's eDP, HDMI's, the GPU's own and the touchpad's buses root's, ahead of seat-late
  '';
  open-with-passes-clippy = ''
    clippy finds nothing in the portal's chooser or its tests
  '';
  palette-graphs-draw-in-braille = ''
    a row's graph is ten braille characters over a dim track, a bar from the left, a spark from the
    right and a strip a reading a member, warning at 95%; graphs line up in one column, and a
    figure wears no glyph
  '';
  palette-rows-keep-their-words-apart = ''
    a palette's row and field keep their words apart and inside them, however long, ending what
    doesn't fit in an ellipsis
  '';
  pause-for-calls-follows-the-calls = ''
    pause-for-calls counts a call app's running recording as a call, leaving out PipeWire's filters,
    dictation, the camera's videos and monitors, pauses Spotify two seconds into a call and plays it
    five seconds after, and finds each Spotify on the bus
  '';
  pause-for-calls-passes-clippy = ''
    clippy finds nothing in pause-for-calls or its tests
  '';
  pebble-lights-follow-the-sound = ''
    pebble-lights finds the speakers' control and sound card, keeps their lighting on while sound
    plays and a while after unless the screen is locked, and lets the speakers' own button win
  '';
  people-holds-everyone-ready = ''
    the people daemon answers a get as a step-in, waiting within its patience for every source
    that can answer, asks ahead for a watch, says what it holds and gathers a burst of changes
    into one, says a source running is answering before anything asks it, never waits on a source
    signed out, starts a source that stops again and asks it about everyone watched, says a get
    nothing answered in time isn't ready, and refuses an ask or a kind it doesn't know
  '';
  people-passes-clippy = ''
    clippy finds nothing in the people daemon or its tests
  '';
  people-runs-where-a-source-names-a-program = ''
    the people daemon runs for Slack's source on the daemons' socket, restarting as the new one
    after a switch, and doesn't run where no source names a program for it
  '';
  pi-ask-answers-on-the-machines-plan = ''
    pi-ask starts pi for the launcher's Ask row speaking RPC with nothing loaded but the machine's
    extensions, the digest of what's true on the machine and, on Linux, the launcher's tool as its
    one tool, on pi's own plan and quick model by default, or through pi-claude on the Claude plan
  '';
  pi-ask-knows-the-machine = ''
    pi-ask's digest says what's true on the machine as each question starts: the time, its
    battery, disk and network, what waits on the person and the quests being worked, from the
    machine's own files alone
  '';
  pi-ask-runs-the-launcher = ''
    pi-ask's one tool lists the launcher's places and actions and runs one, passing on what was
    done, or that a disruptive one only waits for Return
  '';
  pi-asks-claude-code-its-login-once = ''
    pi starts Claude Code to ask for its login once for any number of asks within a minute, so a
    question waits on one start of it rather than several, and asks again while it isn't
    logged in
  '';
  pi-claude-keeps-to-the-plan = ''
    pi-claude starts pi with Claude Code's provider, on Sonnet unless a model is asked for, off the
    mesh, signed in to the plan's own folder and with nothing that would bill an API instead
  '';
  pi-keeps-its-own-settings = ''
    pi's settings take core's at a switch and keep pi's own keys; a file holding them isn't
    rewritten, a missing one is made, and one not JSON is refused untouched
  '';
  pi-loads-its-packages = ''
    pi's package list is the harness and neovim.pi on every machine, a package a machine adds comes
    after them rather than in their place, and pi is given each git package as its current link
  '';
  pi-measures-each-day-in-the-background = ''
    the harness's measure runs each morning in background.slice with the store's node, catching up on
    a morning slept through, skipped where the harness isn't installed, and never on a Mac
  '';
  pi-writes-what-the-last-boot-left = ''
    the launcher's list of the pi sessions the last boot's end took away is written as the account's
    session starts, in background.slice with the store's node and no module-type warning, skipped
    where the harness isn't installed, and never on a Mac
  '';
  pi-notes-windows-on-sessions-elsewhere = ''
    each minute the harness notes which windows here are joined to a session another machine holds,
    so the next boot can offer them back, in background.slice with the store's node, skipped where
    the harness isn't installed, and never on a Mac
  '';
  pi-packages-compiles-for-macos = ''
    pi-packages and its tests pass clippy as a Mac compiles them, where libproc says which
    processes use a version
  '';
  pi-packages-passes-clippy = ''
    clippy finds nothing in pi-packages or its tests, which hold a live process's version unchanged
    through an update, keep the version before current and the one a live pi says it loaded, and
    never swap in a build that fails or leaves a native module unbuilt
  '';
  pi-packages-swap-in-the-background = ''
    pi's git packages are updated at a switch before pi's settings name their links, and every
    fifteen minutes in background.slice on Linux and by a launchd agent on a Mac, and a pi's start
    or end, watched in each package's loaded folder, removes the versions no pi runs, however many
    pis start or end at once
  '';
  pi-runs-on-the-machines-plan = ''
    a machine on the Claude plan has one pi, starting on the claude-code provider and Sonnet with
    no pi-claude beside it, and a work machine's pi is unchanged
  '';
  pi-says-what-starts-its-sessions = ''
    pi tells mycelium the command its sessions start with in each realm: plain pi at home, pi-claude
    in the Claude plan's realm, and plain pi alone on a machine on that plan
  '';
  pi-says-when-the-plan-runs-out = ''
    pi says plainly when a Claude plan limit is used up, which limit and when it resets, that
    nothing went to extra usage, and in words it never retries
  '';
  pi-tells-where-as-the-prompt-does = ''
    pi's status line finds the folder by the prompt's roots, asks the prompt's git-engine, and marks
    git with the theme's glyphs
  '';
  pi-wears-the-theme = ''
    pi's text, strings and deepest thinking are the account's text, string and alert colours, and
    every colour it names is hex
  '';
  platform-profiles-reach-the-legacy-file = ''
    every profile Dell's driver offers, the XPS 14's SoC slider takes too
  '';
  plymouth-waits-passes-clippy = ''
    clippy finds nothing in plymouth-waits or its tests
  '';
  pnpm-keeps-one-store = ''
    pnpm 10 and 12 find an account's one store from what its shells and user manager are given
  '';
  processor-telemetry-passes-clippy = ''
    clippy finds nothing in the processor telemetry reader or its tests
  '';
  programs-nobody-starts-stay-unlisted = ''
    the entries of virt-manager, the NixOS manual and umpv are masked as hidden where the launcher
    reads them, while the programs stay installed
  '';
  qemu-starts-after-its-sockets = ''
    the qemu VM tests run starts once the sockets it connects to are there, and fails naming one
    that never comes, ending what its script started
  '';
  qt-programs-wear-the-theme = ''
    Qt programs in the session take GTK's platform theme, and Quickshell is started without it
  '';
  quickshell-compiles-every-file = ''
    Qt's own compiler compiles every file of the account's shell and the greeter without refusing or
    crashing
  '';
  quickshell-opens-programs-through-niri = ''
    programs the shell opens survive the shell restarting
  '';
  quickshell-installs-what-it-imports = ''
    every file a shell file imports by its path is installed beside it
  '';
  quickshell-passes-qmllint = ''
    qmllint finds nothing in the account's shell or the greeter, leaving out only types Quickshell's
    files don't export and Qt enum parameters
  '';
  quickshell-restarts-with-its-files = ''
    a switch restarts the shell when a file it reads changes
  '';
  quickshell-takes-its-shape-from-the-theme = ''
    every corner and border in the shell is the theme's
  '';
  record-window-passes-clippy = ''
    clippy finds nothing in record-window or its tests
  '';
  resolved-is-set-per-network = ''
    a laptop leaves resolved's global DNS settings as they ship
  '';
  run-subtests-keeps-what-is-named = ''
    run-subtests keeps a script's boot and helpers and only the subtests named, and runs a pytest
    test whole with pytest picking the tests named, refusing a word pytest would read as more
  '';
  rust-programs-pass-clippy = ''
    clippy finds nothing in the single-file programs or their tests
  '';
  se-keys-builds-for-any-name = ''
    the Mac's key maker builds, shellcheck and all, for a name with an apostrophe
  '';
  session-backup-passes-clippy = ''
    clippy finds nothing in session-backup or its tests, which hold it to only adding to its one
    folder, sealed copies that open, carrying on where a stopped run left off, and removing an
    old log from the machine only once Drive still holds its copy
  '';
  session-backup-stays-in-the-background = ''
    an account naming a folder backs its sessions up hourly in background.slice, catching up on an
    hour slept through, trying a failed run again and left running by a switch, and one naming
    none runs nothing
  '';
  session-machines-have-a-bridge = ''
    libvirt's bridge helper lets session machines onto virbr0, and its default network starts at
    boot
  '';
  session-memory-is-protected = ''
    every slice above session.slice carries a hard memory floor
  '';
  shaper-finds-the-rate = ''
    the shaper reads a queue from delayed pings, not one late ping or a silent host; cuts under what
    the line carried, holds while it drains, and climbs while it's quiet, past what it held only
    once busy five seconds, so bursts fit under the line; forgets a cut ten quiet minutes old, and
    keeps one the line then proves; cuts no download arriving faster than its rate, nor beside one
    an upload carrying under half the line's best, nor again for a queue back within two seconds;
    climbs back from a cut no queue returns for, so late pings alone don't wear an upload down, yet
    retries no cut near one confirmed; never cuts Wi-Fi more than 5% under the best it carried,
    while following a cable down, unless a busy direction's queue stands six seconds at that floor,
    or 40% while real-time traffic flows, which a call's packets turn on for 30 s; remembers the
    line's best, not its latest cut; reads sets, routes and queues from what the kernel prints
  '';
  shaper-passes-clippy = ''
    clippy finds nothing in the shaper or its tests
  '';
  shell-preview-follows-the-working-copy = ''
    shell-preview links each built file the working copy holds back to it, copies what the build
    made, and ends the nested desktop it started however it's ended, asking it before killing it
  '';
  shell-preview-passes-clippy = ''
    clippy finds nothing in shell-preview or its tests
  '';
  shutdown-status-passes-clippy = ''
    clippy finds nothing in shutdown-status or its tests
  '';
  shutdown-status-reads-the-last-stage = ''
    shutdown-status reads only the last stage's steps, says forced once when it begins unmarked, and
    leftovers after three quiet seconds
  '';
  slack-finds-who-it-knows = ''
    slack-people says who Slack knows by an email address with their time zone, presence and
    status, on a grid by a search their profile confirms, and one it knows nobody by; finds each
    once with the harness's sign-in, keeping who it found across a restart, and asks how they
    are each time, only ever reading; finds their conversation with Joel in plain words, only
    what's in a direct message with them, newest last; asks ahead only after a step-in, finding
    their conversation once and never how they are; and asks nobody once Slack refuses the
    sign-in or with none at all, saying so; and slack-unreads keeps in a file, whole, the direct
    messages with unreads, the channels mentioning Joel and his mentions in threads, never a
    channel merely unread, each named once a run and opening in Slack, a refused sign-in kept as
    signed out with nothing counted, and with no sign-in no file and nothing asked
  '';
  slack-passes-clippy = ''
    clippy finds nothing in slack-people or its tests
  '';
  slack-unreads-go-with-the-service = ''
    slack-unreads starts once pi has signed in to Slack, restarts as the new one after a switch,
    is restarted when its watchdog hears nothing, and takes its file with it however it stops
  '';
  slack-waits-in-its-place = ''
    Slack's source in People asks about each address once a minute as Joel steps in, never one
    known to be nobody, and ahead about those listed, in order, each once; Write in Slack stands
    dimmed, looking in Slack, until it's answered, or not in Slack once a step-in found nobody,
    and isn't listed for someone already known not to be there; answers that come together are
    taken as one change, and those already known as none; their conversation is found by any of
    their addresses, its drawer saying who said each message, newest last
  '';
  smartd-stops-cleanly-without-disks = ''
    the workstation's own smartd command, run with no disks, stops cleanly rather than failing
  '';
  sound-hears-only-what-can-be-heard = ''
    the launcher's Sound leaves out a jack with nothing plugged in and a display's output with no
    display, and never listens to a headset's microphone while it plays music
  '';
  speaker-amps-have-every-dai-named = ''
    the XPS 14's SoundWire machine driver names only DAIs its amplifiers register
  '';
  spend-watch-accepts-its-configuration = ''
    spend-watch passes its tests and accepts the config an account gets, a feed and a cap among it
  '';
  spend-watch-passes-clippy = ''
    clippy finds nothing in spend-watch or its tests
  '';
  spend-watch-stays-in-the-background = ''
    the AI spend watch yields to the account, comes back when it fails, and leaves its status behind
    when it stops
  '';
  spotify-backs-out-on-escape = ''
    Escape goes back a page in Spotify once nothing is left to close or let go of
  '';
  spotify-draws-in-the-theme = ''
    Spotify's client is built in the theme's colours and faces
  '';
  spotify-player-accepts-its-configuration = ''
    spotify-player starts with its settings
  '';
  spotify-player-signs-in-where-spotify-answers = ''
    spotify-player's sign-in asks ncspot's Spotify app to answer at the one address that app takes
  '';
  stands-alone = ''
    nothing here names a machine, its repository or employer
  '';
  surface-clock-passes-clippy = ''
    clippy finds nothing in surface-clock or its tests
  '';
  swayimg-accepts-its-configuration = ''
    swayimg draws a picture with the account's settings and the machine's fonts without a complaint,
    and pictures open in it
  '';
  swayimg-moves-by-its-keys = ''
    in swayimg, pressed as a keyboard presses them, → and ← go to the next and previous picture and
    End and Home to the last and first, and the picture it opened with, and none after, is said to
    the desktop's list of files opened lately
  '';
  swayimg-says-the-picture-it-shows = ''
    swayimg keeps the picture it shows, read by its Recent's command as its process and whole path,
    and nothing once q closes it, clearing what a killed one left
  '';
  the-askpass-outlives-its-shell = ''
    the askpass asks a restarted shell again, never asks twice what's answered, and refuses about 20
    s after the shell goes unless it comes back
  '';
  the-askpass-says-who-asks-for-what = ''
    the askpass names the program and the key, passkey or host behind each prompt, and whether it
    wants a yes, a secret, words or a touch
  '';
  the-camera-gives-limited-range = ''
    the XPS 14's camera relay, run on a full-range ramp, squeezes it exactly into limited range and
    neither of its converters changes it after
  '';
  the-camera-holds-no-memory-it-never-uses = ''
    neither half of the XPS 14's camera asks at probe for a firmware log only Intel's offline logger
    writes, and the capture side frees its firmware's memory when it unbinds
  '';
  the-camera-takes-its-boards-tuning = ''
    the XPS 14's camera reads Dell's tuning for the board a machine names, and Intel's otherwise
  '';
  the-camera-takes-the-one-plugged-in = ''
    the camera program takes a camera on a removable USB port over the machine's own while it's
    plugged in, never a built-in one on a fixed port, and the machine's own with none
  '';
  the-click-strength-finds-the-intensity = ''
    the click strength's setter finds a haptic pad's intensity, the XPS 14's among them, and writes
    only it, only when it differs
  '';
  the-click-strength-holds-after-sleep = ''
    a machine naming a click strength sets it as each hidraw device appears and after every sleep,
    and one naming none runs nothing
  '';
  the-colour-camera-says-it-is-upside-down = ''
    the XPS 14's camera bridge reports its colour sensor rotated 180 degrees
  '';
  the-console-greets-the-account = ''
    a console greets the account by its description's first word, or its name, with the keys back to
    the desktop, and keeps NixOS's banner with no account
  '';
  the-console-font-refuses-a-stand-in = ''
    the console font's build draws the face asked for at Medium, among its other names too, and
    refuses another face or one without a Medium, saying what fontconfig offered
  '';
  the-console-wears-the-face = ''
    the initrd's console font is the account's mono face at Medium, PSF 2 at 16 by 32 with at most
    256 glyphs and a blank slot 32; none without an account
  '';
  the-console-wears-the-theme = ''
    the console takes the account's terminal colours in its own order, whatever the theme, and a
    machine with no account keeps the kernel's
  '';
  the-desktop-comes-first-in-its-session = ''
    niri and the shell sit first in session.slice, at ten times its other units' CPU weight,
    each with a hard memory floor that session.slice's own covers with a gibibyte to spare
  '';
  the-desktop-draws-letters-unhinted = ''
    the graphical account's desktop settings tell GTK and Chrome to draw letters unhinted, as macOS
    does
  '';
  the-disk-loss-panic-tells-trouble-apart = ''
    `disk-loss-panic` counts a warning or worse from the disk's path or a hung task as trouble and
    nothing else, sees an unarchived kernel log in pstore, and finds a disk that's gone failing
  '';
  the-disk-piece-reads-the-guards = ''
    the disk piece says the worst disk as the guards wrote it, and a guard gone quiet
  '';
  the-ergodox-command-reaches-the-board = ''
    `ergodox` writes each command whole to the board's channel alone
  '';
  the-ergodox-layout-builds = ''
    the layout fits the board and answers with its own serial
  '';
  the-ergodox-watch-tells-the-layer = ''
    the layer watcher finds the board and says only what it's told
  '';
  the-firmware-takes-what-is-declared = ''
    `firmware-settings` names what differs, writes it with the password, then wipes it, even when
    stopped partway, and refuses a value it can't read
  '';
  the-glossary-names-each-thing-once = ''
    no word in docs/glossary.md names two things in one context, and no name it replaces is
    replaced twice or written as any context's word, each clash is printed, and the check
    refuses a stand-in glossary breaking each of its rules
  '';
  the-gpu-security-link-loads-with-the-gpu = ''
    the XPS 14's initrd loads mei_me and mei_gsc_proxy with xe, inside the 20 s xe waits for its
    CSME link
  '';
  the-guide-finishes-with-the-machines-own-steps = ''
    `guide finish` sees each step a machine's repo adds once someone is signed in, its check by
    its program's path, and a step given GitHub's id or another's is refused
  '';
  the-guide-keeps-one-accounts-spare-on-the-yubikeys = ''
    the installed guide keeps the spare SSH key of the one account `jitsusama.yubikeySpare` names
    on each YubiKey, and a machine where two accounts want one is refused
  '';
  the-guide-makes-the-keys-the-modules-name = ''
    the installed guide makes every TPM key ssh-tpm-agent's and tpm-signing's modules name, for
    the account naming it, with the PIN each asks for
  '';
  the-guide-shell-keeps-its-vm-tests-qemu = ''
    the guide's development shell holds the QEMU with canokey its VM tests boot under, so the
    shell's root keeps it from being collected and compiled again
  '';
  the-keyboard-light-is-heard = ''
    the shell finds the keyboard's light among the kernel's and hears each change its firmware
    makes, once, with its level, off as dark
  '';
  the-launcher-answers-the-rain = ''
    the weather's parts and the rain by the hour come from one forecast, and "rain", "umbrella"
    or "when will it rain" answer when it next rains, here or in a place; here is the location, by
    its town or else its position, ahead of the place set and then the time zone, and the footer
    says which
  '';
  the-launcher-answers-what-is-asked = ''
    sums, bases and bits, addresses, subnets, characters, measures, zones, colours, the machine's
    figures and its parts' heat answer
  '';
  the-launcher-asks-a-fresh-pi = ''
    the Ask row says why pi-ask can't start before it's asked and within 3 s of Return, which
    asks again rather than closing, and each question is answered by a pi of its own
  '';
  the-launcher-catalogues-what-it-does = ''
    the launcher's catalogue tells a place from an action, offers what ends something rather than
    doing it, and lists no figure, copy or command's start
  '';
  the-launcher-chooses-files = ''
    a graphical machine's portal asks the launcher's chooser for a file to open or save, GTK 3
    programs ask the portal, their own dialogs included, and open-with's description says it
    answers it
  '';
  the-launcher-chooses-files-as-asked = ''
    the launcher chooses the files a program opens or saves in the folder it asks for, the files
    opened lately first, then its folders and files, then its filters, and answers what was picked;
    a place kept elsewhere, as Google's Drive, is listed after home's files, and its files are
    fetched before they're answered; saving there, its folders take the name, ask before replacing
    one, and answer the path the place stages it at once its folder is made, marked beside it with
    the program that asked
  '';
  the-launcher-finds-emoji = ''
    emoji are found by name and meaning, best first
  '';
  the-launcher-finds-the-desk-light = ''
    the launcher says how the desk light is lit and sets its brightness, warmth and power through
    litra, and the bar hears changes but not a camera putting it out
  '';
  the-launcher-finds-the-keyboards-keys = ''
    a key is found by what it types, and shown where it is, and the board's Layer and Lights hold
    choices that run its own command
  '';
  the-launcher-finds-the-people = ''
    People with Tuple as its source lists favourites and those called lately, finds anyone as they
    stand, calls, joins and favours them, steps in before calling, lists a source's page for them
    after the ways to reach them with its facts in the drawer and the time where they are, and
    never starts a stopped Tuple
  '';
  the-launcher-finds-the-quests = ''
    the launcher lists open quests driving first, finds finished ones and ids, opens pi on one in
    its tree or goes to its session, and sees one made while it was shut
  '';
  the-launcher-reads-talks-boundedly = ''
    the orchestrators' talks are read only from the inboxes holding them, of 120, and focus
    moving among 120 sessions makes no reader again, so the shell's threads stay bounded
  '';
  the-launcher-finds-the-queue = ''
    the launcher's queue names each job in words, running first, says its time against its last
    run or what it waits for, its quest and memory, and goes to its session or copies its name
  '';
  the-launcher-finds-the-runs = ''
    the launcher lists the day's fleets and councils, running first, with progress or cost and time,
    goes to their session, opens an answer whole, and hides amounts while sharing
  '';
  the-launcher-finds-what-is-meant = ''
    the right row comes first for each query, within budget
  '';
  the-launcher-groups-every-shortcut = ''
    every shortcut of every layer lands in one place in Shortcuts, a desktop chord always in a group,
    a new chord finds a group of its own accord, every group is named in the vocabulary, and
    Shortcuts steps from the layers to their groups to the chords, typing finding any of them flat
  '';
  the-launcher-holds-a-programs-controls = ''
    a program's step holds its windows, its own actions, Quit asked once, then Media, Sound and
    Notifications, in order and only where they apply, each reaching only that program, with what
    another provider gives it to copy last, and a hidden program another provider gives acts to is
    listed and steps in rather than starting
  '';
  the-launcher-holds-still-as-it-refreshes = ''
    a list the launcher shows keeps its order and its selection as what's in it changes
  '';
  the-launcher-keeps-its-trials = ''
    each opening of the launcher is kept as its keys by kind, its pick and how long it took, and a
    miss only where letters were typed and nothing picked
  '';
  the-launcher-lands-where-it-did = ''
    a thing is one row, the same letters keep their row, one slip teaches nothing
  '';
  the-launcher-makes-texts-ready-lightly = ''
    making a text ready to match and keeping it grows the launcher's memory by at most 10 KB a
    text, where one grown a letter at a time took 14
  '';
  the-launcher-means-one-thing-a-word = ''
    no name, word or question in the launcher's vocabulary finds two things at once unless it's
    listed as meant to, and each clash, unknown name, missing glyph and thing with no place is
    printed, and the check refuses a stand-in vocabulary breaking each rule of what Joel reads
  '';
  the-launcher-names-each-thing-its-term = ''
    each launcher row naming a glossary thing gives its term as Context/word, or a place named
    for a context the context alone, the glossary holds that word or context, and the row's name
    says it, a part's after its thing's name; each one that doesn't is printed, and the check
    refuses a stand-in vocabulary breaking each of its rules
  '';
  the-launcher-names-the-parts-it-builds = ''
    every part the calculator, the time, today, the waits, a hardware address, a network and a
    character show is named as the vocabulary names it
  '';
  the-launcher-says-why-the-desktop-lags = ''
    the launcher's Lag leads with the worst of what waited, offers to see what's behind it from
    the top, says how fast the desktop answers in plain words, and says nothing's slowing the
    desktop when nothing is
  '';
  the-launcher-shows-no-replaced-word = ''
    nothing the launcher's vocabulary shows of a thing, a place or a command says a name
    docs/glossary.md replaces, but the other senses and the renames still waiting that
    tests/launcher-shown.py lists, each printed with its word, and the check refuses a stand-in
    vocabulary breaking each of its rules
  '';
  the-launcher-shows-the-moment-in-now = ''
    Now's last rows are the time and the date as the launcher places each anywhere, never a
    version Now draws, the location, saying its town and when the place keeps another zone than
    the clock, the weather, placed the same way, read as the launcher opens and kept while it's
    asked again, then the calendar's own row for the next meeting, and Now counts nothing while
    only they are there
  '';
  the-launcher-sums-within-a-key = ''
    the launcher's sums and measures, worked out by libqalculate in the shell's own process, take a
    typical key under five milliseconds
  '';
  the-litra-follows-the-cameras = ''
    `litra` finds the lights by their ids, speaks their reports, and counts a camera streaming only
    by a whole file naming a program
  '';
  the-mac-guards-its-disk = ''
    a Mac's guards run under launchd, with a Mac's paths and nothing to confine a run
  '';
  the-macs-faces-are-the-themes = ''
    the graphical role's fontconfig, asked for a Mac's system or monospaced font by a name pages give
    it, gives the theme's sans or monospaced face, carrying the name asked for
  '';
  the-machine-lists-processes-for-its-parts = ''
    the processors' and the memory's headlines make no parts and spawn nothing, and their parts,
    once looked at, list the processes once
  '';
  the-margins-glyphs-stand-level = ''
    every glyph in the margin stands one height on the gap's middle line, read from the right by
    kind, in the colour that says how much it asks; what was on air lingers, what a notice will say
    waits for its card, and a wait shows once the same one lasts; each stands in a square cell the
    gap's size with the same room all round, and What's on points at the row the keys are on
  '';
  the-margins-marks-are-drawn-level = ''
    every mark the margin can show, a device's kind among them, is the theme's drawing of it,
    standing the grid's full height and centred across it as Qt draws it
  '';
  the-mic-mute-light-follows-the-mute = ''
    `mic-mute-light` asks after the mute only once a change to a mute or the default source has
    ended, and lights the light for a muted source alone
  '';
  the-music-follows-the-player = ''
    the music is the player that started last, never playerctld's echo
  '';
  the-notices-keep-their-order = ''
    the notices' stack puts a critical one in front and the rest in the order they came, one unheard
    at the back, and turns and brings one forward as the keys and a click ask
  '';
  the-notices-land-on-niri = ''
    the account's notices, on its real niri nested in a headless sway in the sandbox, draw a notice
    sent as a card on niri's notices layer, and what Super+Shift+N runs puts a critical one away
    unread in the history; and Google Drive's notice that answers a save is kept unmissed, by its
    x-answers hint, while its failure is missed
  '';
  the-notices-wait-while-joel-is-away = ''
    a notice that comes while nothing has been typed or moved stays on niri past the time its sender
    gave it, never counting down
  '';
  the-npu-finds-its-compiler = ''
    the XPS 14's graphics drivers hold the NPU's driver and Intel's compiler beside it
  '';
  the-overview-takes-its-pieces = ''
    a machine moves, cuts, retunes and adds the overview's pieces
  '';
  the-panel-dims-after-a-bad-read = ''
    the XPS 14's kernel keeps to VESA's brightness controls when its boot-time read of the panel's
    eDP revision comes back wrong
  '';
  the-panel-holds-still = ''
    the XPS 14's kernel leaves the panel's self refresh and frame buffer compression off, since
    PSR2, PSR1 and compression each set the screen flashing after a replug or a lid-shut boot
  '';
  the-panel-knows-its-luminance = ''
    the XPS 14's kernel reads the panel's luminance range from DisplayID's Display Parameters block
  '';
  the-panel-offers-variable-refresh = ''
    the XPS 14's kernel reads the panel's refresh range from DisplayID's Adaptive Sync block
  '';
  the-panel-slows-on-a-low-battery = ''
    the XPS 14's panel drops to 60 Hz below 30% on battery, holds there, and comes back to its
    fastest rate on the charger
  '';
  the-panic-screen-wears-the-theme = ''
    the kernel's panic screen draws the account's text colour on its background, whatever the theme,
    and keeps the kernel's with no account
  '';
  the-people-are-one-person-each = ''
    every source's people join into one person per email address, a source that knows two
    addresses for someone joining them, each standing as the first source that says, favourites
    listed first, then whoever there's been something with, the last first, someone only asked to
    what's to come left to be found by typing, with their details for the drawer and their acts in
    the one order
  '';
  the-picker-picks-what-the-keys-say = ''
    the region picker starts on the middle quarter, moves and sizes its box by arrows within the
    screen, and names a 16 by 10 grid's cells by two letters
  '';
  the-pinentry-asks-the-shell = ''
    linux-id's pinentry asks the askpass in the program's words
  '';
  the-power-profile-answers-before-the-network = ''
    the power profile daemon's unit, the XPS 14's patched one too, waits for neither the network nor
    the boot's end, and keeps upstream's settings
  '';
  the-profile-saves-power-on-a-low-battery = ''
    the power profile follows the charger, goes to power saver below 20% on battery, and holds a
    profile picked by hand
  '';
  the-prompt-gives-way = ''
    from 120 columns to 16 the prompt keeps the folder and glyph, leaves room to type and marks what
    it hides, with refs in files or reftable
  '';
  the-prompt-starts-nothing = ''
    the prompt starts only git-engine and its mkfifo, then forks nothing across prompts in and out
    of a repository, reftable included
  '';
  the-screen-keeps-its-level-while-the-lid-is-shut = ''
    a shutdown with the lid shut keeps the screen's brightness saved while it was open, so the
    next boot doesn't come up dark, and a keyboard's light is saved either way
  '';
  the-screens-come-back-after-sleep = ''
    once the machine wakes, swayidle turns back on the screens it turned off before sleep
  '';
  the-screens-go-off-before-sleep = ''
    before every sleep swayidle locks the screen and then turns the screens off, even when the
    lock is late, which still fails
  '';
  the-shell-follows-dictation = ''
    the shell follows each dictation state, watches again a service its socket restarts, and tries a
    socket nothing listens on once, never looping
  '';
  the-shell-follows-tuple-calls = ''
    the shell reads from Tuple's log lines a call's start, end and length, who's in it, sharing or
    ringing, Joel's mute, and a call's link
  '';
  the-shell-hears-tuples-call = ''
    the shell's Tuple reads a call from tuple-people's stream, who's in it, since when, mutes,
    screens and cameras, tells comings and goings, and wakes on Tuple's log
  '';
  the-shell-holds-a-daemons-records = ''
    the shell holds a daemon's records over one connection it keeps: a daemon that's down is said and
    never waited on, one that starts is connected to by itself, keys watched ahead and a get's answer
    land by key, and what was held stays through a gap
  '';
  the-shell-reads-the-spend-watch = ''
    the AI spend piece and collection draw what the watch wrote, put what wants a look first, step
    into each figure's parts, hide amounts while sharing, and answer reports by their words
  '';
  the-shell-speaks-its-javascript = ''
    the shell calls no method Quickshell's JavaScript lacks
  '';
  the-shell-starts-nothing = ''
    the account's zsh starts only git-engine, its fifo and direnv, and each prompt after runs direnv
    alone
  '';
  the-shell-reads-the-documents = ''
    the shell reads the documents from the google program's file, none from a line from before
    them, knows which Google program opens each kind, and says who owns a document and when Joel
    last opened it beside its title, with its details for the drawer; reads the Drive folders
    listed and the downloads, finding each folder asked together, writes the lines that list a
    folder and download a file, a save alone saying where, and says how the newest download asked
    stands; and says a Drive file in the picker by its size and when it changed, or what Google's
    own kind comes as, picked by that name and kind, adopting its document row, with why a file or
    a folder couldn't be had; and reads the saves into Drive, writes the line that sends one again,
    tells each settled since the shell started once, oldest first, by its folder's name, sent as an
    answer with Open in Drive and failed with why and what mends it, a copy that couldn't be read
    going back to the program that saved it while it's open, and finds a failure's notice by its
    file alone
  '';
  the-shell-reads-the-mail = ''
    the shell reads the google program's whole state from its file, a line from before mail
    included, and nothing from half a line; says when mail came in words, and what its row and its
    details say, who was copied and the files it carries among them, each file by its kind and size;
    writes a reply to who sent it, and to all, copied to everyone else it went to, and a mail to
    everyone else a meeting names; writes the lines that mark mail read or unread again or archive
    it or fetch a file it carries; says how that fetch stands; and writes the line making a Meet
    meeting and says how one asked for stands
  '';
  the-shell-tells-the-next-meeting = ''
    the shell names as next the meeting about to start or going on, never one over, all day or said
    no to, says when it is in words, when it's about to start and when it's said as a notice,
    whether Return joins it or shows it, whether it's today's and how Joel answered it, gives its
    details for the drawer, how it repeats in words and how many of those asked answered each way
    among them, which may be moved and to what later or typed time, what that clashes with and who's
    told, writes the lines that answer, cancel and move it, and knows which Meet call a window is
    and whose meeting it is
  '';
  the-shutdown-mark-cannot-stop-a-reboot = ''
    the mark a healthy stop leaves can't fail the reboot, and what reads it outlives the last stage
  '';
  the-sign-in-consoles-wear-the-theme = ''
    the sign-in consoles are kmscon in the account's mono face, its padding and terminal colours,
    doubled on the XPS 14, and Ctrl+Shift+plus zooms
  '';
  the-signer-picks-who-signs = ''
    git's signing program signs with Joel's key at a terminal and the agent's over pipes or with
    SIGN_AS, and takes back a signature that doesn't verify
  '';
  the-splash-keeps-systemd-quiet = ''
    with the splash, systemd writes only failures to the console
  '';
  the-splash-outlasts-niris-hold = ''
    niri holds its first frame for the wallpaper, only with one to wait for, for less time than
    Plymouth takes to quit, so what the splash left stays on the screen until the desktop draws
  '';
  the-splash-reads-the-generation-from-the-entry = ''
    the splash is told the generation only from an entry whose name carries one
  '';
  the-splash-stays-until-niri-draws = ''
    the splash lets go of the screen as greetd starts, keeping the GPU open, and quits ten seconds
    after greetd starts rather than with the boot, so the console never shows between it and niri;
    a machine without the splash keeps greetd waiting for Plymouth to quit
  '';
  the-surface-boots-with-its-gpu-and-a-clock-that-reads = ''
    the Surface Laptop 4 loads its GPU's driver and the CMOS clock in the initrd, and keeps
    the firmware's unreadable clock from taking rtc0
  '';
  the-surface-keeps-the-clock-its-firmware-starts-from = ''
    the Surface Laptop 4 reads its controller's clock for the firmware's offset before timesyncd
    starts, through the device the controller's module gives, and sets that clock at shutdown and
    every hour
  '';
  the-surface-keyboard-types-at-the-disk-prompt = ''
    the Surface Laptop 4's initrd has every driver on its keyboard's path, and keyd lays that
    keyboard out there
  '';
  the-surface-loads-its-touchscreen-once-up = ''
    the Surface Laptop 4 installs its touchscreen's driver with the panel's qualified settings, and
    loads it from a unit after multi-user.target rather than while the system starts
  '';
  the-surface-offers-its-firmware-through-fwupd = ''
    fwupd, given the Surface Laptop 4's ESRT with its UEFI at 4.355.140 and its TPM at 7.2.2.0,
    offers the UEFI's capsule from the remote the module writes, at the version the capsule itself
    carries, as a trusted upgrade, and nothing for the TPM
  '';
  the-surface-watchdog-wakes-again = ''
    the Surface Laptop 4 arms systemd's runtime watchdog at 30 s, and its sleep hook, run with an
    empty PATH against a stand-in of busctl, stops that watchdog before a sleep and starts it again
    after at the same timeout, touching nothing when it isn't running
  '';
  the-window-buffers-stay-under-load = ''
    the XPS 14's udev sets xe's `shrinker_non_purgeable_seeks` to 64, so reclaim under a build
    leaves window buffers alone
  '';
  thermald-starts-after-its-drivers = ''
    the XPS 14's thermal drivers load before thermald starts, and a thermald that stops is restarted
  '';
  timed-work-goes-ahead-of-other-work = ''
    timed work sits inside jobs above its holder, held by a unit that comes back, and pi finds it
  '';
  timekeeping-passes-clippy = ''
    clippy finds nothing in timekeeping, which works out the clock's offset and delay as
    timedatectl does, or its tests
  '';
  tuple-completes-in-zsh = ''
    Tab in zsh offers tuple's commands and subcommands, through Tuple's own completion and the real
    tuple
  '';
  tuple-keeps-its-logs = ''
    Tuple's service keeps the daemon's last three logs, newest first, before it starts, and an empty
    log pushes none out
  '';
  tuple-opens-its-links = ''
    xdg-open and gio send tuple:// links to Tuple's entry, which joins a call's link, starts Tuple
    bare, and refuses any other link with a notice
  '';
  tuple-shares-through-the-portal = ''
    Tuple reads the portal setting where the switch writes it
  '';
  tuple-takes-no-real-time-priority = ''
    Tuple's daemon may take no real-time priority, and goes first by a nice below 0 and a CPU weight
    above 100
  '';
  update-to-main-outlives-its-switch = ''
    neither NixOS nor nix-darwin restarts update-to-main while it switches them: the service never
    restarts on a switch, the Mac's daemon names no store path, both start it from the running
    system, and its askers are its group's members
  '';
  update-to-main-passes-clippy = ''
    clippy finds nothing in update-to-main or its tests
  '';
  update-to-main-updates-to-signed-main = ''
    update-to-main builds and switches to a main signed by a key it was given and descended from what
    it ran, builds nothing when main hasn't moved, refuses an unsigned, untrusted or rewritten main
    and a low battery, refuses to start or switch while the machine is busy or can't tell, leaves
    the machine as it was when the build fails, goes back when the switch
    fails or the machine isn't whole after it, leaves a new kernel for the next boot, joins asks to
    the update that runs, refuses any other ask, serves on a socket only its group writes, and keeps
    how each update ended in a file only its group reads; and a hosted main, unsigned, is built
    from its flake's folder with root reading the account's checkout, fetched there first when it
    lacks main, its private inputs fetched by their locks, refused when rewritten, and builds
    nothing without a token; and a followed input is built at its own signed main past the lock,
    builds again when only it moved, and is refused unsigned or behind its lock; and building
    elsewhere sends only the heavy pieces away, builds nothing here without a builder, sends a
    kept piece only to its own builders, and none at all when it has none, unless it may be built
    here, small, which waits under load, and builds what's built from one here with no builders
  '';
  usb-camera-holders-follow-the-stream = ''
    a USB camera's watch names its holders only while a streaming interface has a nonzero alternate
    setting, names its file for the camera, made a plain file name, and removes it when the node
    goes or the watch is stopped; a holder's name is one printable line, and only a unit in its
    cgroup is left out
  '';
  v4l2loopback-holders-passes-clippy = ''
    clippy finds nothing in the cameras' holders watcher or its tests
  '';
  vm-admins-switch-unasked = ''
    an account in wheel switches to another system and back with no password; one outside wheel, and
    what isn't a NixOS system in the store, is refused
  '';
  vm-admins-tune-memory = ''
    an account in wheel sets the watermarks' spacing, transparent huge pages, the multi-gen LRU and
    swappiness to a listed value with no password; one outside wheel can't, and an unlisted value needs one
  '';
  vm-admins-tune-the-work-slices = ''
    an account in wheel sets build.slice's and machine.slice's weight and cores for the boot with no
    password; one outside wheel can't, and no other unit is opened
  '';
  vm-apps-come-first = ''
    with no disk named, the account's app and session slices, and the system's user.slice, hold a 1
    ms latency on the drive behind /home, through the kernel (jobs, background, machine and system
    none); with one named, io.cost holds its model and qos, no slice holds a target, one left from
    before is cleared as io.cost turns on, and stopping its `disk-order-<disk>` unit turns io.cost
    off; every slice's io.weight follows its CPU weight;
    app.slice holds a hard 13% of memory, session.slice 1.5 GiB and every slice above them 16%, and
    jobs is slowed past half; timed work sits at 10000 inside idle jobs, and work in jobs can move
    itself there; a scope work in jobs starts goes back to the scope of the work that started it,
    and one with a memory ceiling stays; each command written into jobs.service gets a scope of its
    own in jobs-work.slice, at weight 100, with its own swap, and the holder stays; one naming a
    quest pi's sessions weigh goes in that quest's slice under jobs-work.slice at its weight for
    CPU and disk, which follows the file as it changes, and one naming none or an unlisted quest
    stays; two quests' work on one CPU shares it four to one by weight, and one alone takes it all
  '';
  vm-bluetooth-types-at-the-disk-prompt = ''
    a keyboard paired in the running system is sealed to the TPM under Secure Boot's PCR 7, with its
    link key nowhere in the ESP; after a restart the prompt asks while the pairing is still being
    opened, the keyboard types the passphrase at it and the disk opens, the laptop's address
    unchanged; with nothing sealed the pairings are skipped, and a seal that won't open leaves the
    prompt asking for the passphrase
  '';
  vm-boot-asks-in-the-theme = ''
    the splash asks for the disk, names a slow job, and lasts a restart to its end; a sign-in
    console greets the account, draws the theme's face, padded, signs in to a shell told it's
    kmscon, greets again once it's left, and its keys go back to the desktop; one kmscon can't draw
    falls back to the login prompt, greeting included
  '';
  vm-boot-asks-on-every-screen = ''
    with two screens of different sizes, the splash's ask shows whole on both, its bottom left
    corner on the smaller one included
  '';
  vm-chrome-says-its-open-tabs = ''
    Chromium, standing in for Chrome, installs the open-tabs extension from the local file the
    Chrome module names, unasked, its tabs and nativeMessaging permissions granted and nothing
    disabling it, and chrome-open-tabs lists two pages it opened, brings one to the front and
    closes the other; and with no extension asked, chrome-recent-sites reads two pages it
    opened from the session file it writes, each with its address, in one window with one in
    front, looked at lately
  '';
  vm-checks-go-first = ''
    the daemon sits in a cgroup beside its builds, each build in one of its own handed cpu, io,
    memory and pids; a build's qemu goes from idle to normal at nice -10, every thread and a later
    one, and its build's cgroup is weighed at 1000 for CPU and disk, while a compiler's build beside
    it stays idle at 100; a build running a stand-in systemd-nspawn, with no qemu, is weighed at
    1000 and a service in it goes to normal at nice -10; a build asked for from a quest's slice
    takes the slice's weight and follows it; the daemon's own cgroup carries oomd's mark to pass it
    over; ten restarts of the daemon, vm-first looking every 20 ms, each start and leave the builds
    running, the daemon builds again, a build asking for a range of user ids gets 65,536, a
    uid-range build that makes a cgroup of its own is refused handing it pids with "Permission
    denied" and the daemon restarts beside it, a finished build that wrote 64 MiB is said with at
    least that and its client's command, one that wrote 64 MiB without waiting for the disk says at
    least that between what it wrote and what it left, a build's sync -f is refused with "Function
    not implemented" while its sync runs and root's sync -f outside the daemon works, a stopped
    quest's build is said with its quest, and a stopped build leaves no process nor cgroup
  '';
  vm-clock-keeps-time = ''
    the clock sets from the machine's servers past a network's silent one
  '';
  vm-commits-are-signed = ''
    commits are signed with the TPM's keys, PINs asked for
  '';
  vm-desk-facts-follow-a-replug = ''
    desk-facts names a USB sound card plugged into a running machine, stops naming it once it's
    pulled out, and names it again when it's plugged back in; names a card not on USB by each of
    its devices, saying what one plays at only while it plays; and names a USB keyboard with its
    polling rate until it's pulled out
  '';
  vm-desktop-captures-and-locks = ''
    the desktop takes a screen, a window, a region, a recording and the camera's pictures by the
    keys, and its lock screen, polkit's prompt and the askpass answer and hold
  '';
  vm-desktop-keeps-pace = ''
    the launcher draws half its keys, arrowed or typed, within a second and nearly every one within
    two of a keyboard pressing them, while every core is busy in jobs.slice and build.slice, so it
    never falls whole seconds behind
  '';
  vm-desktop-launcher-answers = ''
    the launcher lists each thing once for everything Joel types, copies what it answers, keeps
    emoji in their own place, and takes the weather's here from where whereabouts says the laptop
    is
  '';
  vm-desktop-launcher-goes-places = ''
    the launcher goes to pi's sessions, quests and runs and to windows, opens and moves them, joins
    a meeting about to start by its name, lists it in Now, counts one within the hour down as a
    want, marked from ten minutes off and said at its reminder and as it starts, shows one further
    off with who's asked and how each answered, its details beside it, answers its invitation,
    writes to its guests, copies its call's link and opens its attachments, cancels one Joel holds
    once he's sure, moves one Google lets him change to a time he types once he's sure, and lists
    its Meet call in Now while it's open, says Google signed out in Now with Return signing in again
    in pi, counts the mail that wants Joel in Now with Return listing it as Mail does, details and
    all, opens mail by its subject, keeping one Mail window, replies to it, and to all on mail that
    went to others too, flips its Read switch both ways, archives it, and opens a file it carries or
    saves a copy, lists the documents Joel opened lately in Docs' Recent and opens one, the same row
    its title finds, joins a Meet call by its code after Meet's New Meeting and copies a new
    meeting's link last, finds a person by name with the ways to reach them, their next meeting,
    their Recent and their address last, and steps to them from a meeting's guests and a mail's
    sender, sets the volume, loses no letter typed after its chord, and shows the shortcuts
  '';
  vm-desktop-opens-and-sets = ''
    the desktop keeps and gives back copies, opens pictures and documents, and sets sound, the
    power profile, displays and Wi-Fi from the launcher
  '';
  vm-desktop-says-what-happens = ''
    notices come, go and are acted on, and the margin, What's on, Tuple's calls and caps lock say
    what's happening and answer the keys
  '';
  vm-desktop-starts-clean = ''
    the desktop boots without an ordering cycle and its shell without an error, the shell draws
    nothing and barely wakes while nothing shows, the meters answer the keys, programs wear the
    theme, and swipes and Bluetooth's agent work
  '';
  vm-dictation-hears-the-chord-let-go = ''
    an account outside the input group hears how long Super and D were held and a tap's, nothing of
    D alone, another key or Super after D, and can't read the keyboard itself
  '';
  vm-dictation-listens-once-it-hears = ''
    dictation's watchers hear it listening only once the microphone sounds, never for a press with
    no microphone
  '';
  vm-disk-guard-keeps-the-disk = ''
    the guards answer each band, sparing what's in use
  '';
  vm-disk-loss-panic-panics-once-the-disk-is-gone = ''
    a warning from the disk's path changes nothing while the disk answers, and once the disk has
    dropped off its bus it panics the kernel, saying why first
  '';
  vm-first-finds-the-vms = ''
    vm-first reads a thread's nice and policy, knows a check with deadlines by its qemu, test
    driver, container or nested compositor, wrapped or cut short, and not a compiler nor a niri
    build's own test programs by their names, finds every process under a build and its
    containers' cgroups, knows the daemon from the workers it leaves behind, finds a worker's client
    and the quest's slice it sits in, keeps a quest's build below a VM's, adds up what a build read
    and wrote on every disk and what it left to write, and says a size, a client's command and a
    build's quest as a person reads them
  '';
  vm-first-passes-clippy = ''
    clippy finds nothing in vm-first or its tests
  '';
  vm-full-swap-ends-what-filled-it = ''
    full swap ends the cgroup that filled it through systemd-oomd, before the kernel's OOM killer,
    and leaves the rest
  '';
  vm-greeter-signs-in = ''
    the greeter signs in and out, not waiting on the account's setup
  '';
  vm-guide-installs = ''
    the guide, driven by its keys, holding the machine awake and drawing over what writes on its
    screen, installs a machine from its stick: two keys and a recovery key of seven words, shown
    alone for a phone and typed back, an install whose memory a restart took carried on through a
    key it enrolled, Secure Boot enforcing with its keys, its firmware enrolling them before the
    disk first asks; then seals the disk to the TPM with a PIN of its own, typed twice and never
    the YubiKeys', which opens it at the next boot, leaving one systemd-pcrlock policy; makes the
    account's TPM keys under one PIN and a spare SSH key on each YubiKey, refusing one asked for by the other's name; clears the TPM,
    opens with a YubiKey, seals again, makes the keys it lost again and downloads a lost spare; a
    writer writes a stick from the rescue bundle the install sealed to the recovery key, which the
    firmware boots while enforcing; then the install's own stick, which it made the machine's by
    carrying its keys and bundle and signing itself with them, boots enforcing and reinstalls the
    machine with the same keys, packing the account's home folder onto a USB drive plugged in
    mid-way and putting it back with its modes, owner, link and TPM keys, which still load, and
    its cache left behind, whose guide frees the first install's policy; and a firmware cleared
    of its keys gets them back from the machine's next boot, not while a program it would start is
    unsigned, and the TPM alone opens the disk with its own PIN once they're enrolled, a YubiKey
    with its PIN when the TPM refuses
  '';
  vm-guide-recovers = ''
    a repair stick finds a machine's disk, opens it with a YubiKey's PIN, checks its filesystem
    without changing it, mounts it whole, copies its home folders onto a USB drive without their
    caches, sets which generation boots next, replaces a lost key, reinstalls a boot loader gone
    from the boot partition, and gives a root shell inside
  '';
  vm-guide-signs-a-stick-for-two-machines = ''
    a repair stick the guide signed with two machines' db keys boots with Secure Boot enforcing under
    each machine's firmware, which holds only that machine's keys, and firmware holding neither
    refuses it
  '';
  vm-guide-writes-a-stick = ''
    the guide writes a stick as the person, holding the writer awake, through UDisks2, unmounting
    what was mounted from it, its passphrase seven words shown alone for a phone and typed back,
    another's words refused, the old stick's rescue bundles read from its vault mounted only for
    reading and the writer's own kept for the person and carried onto the new stick, which is the
    writer's own machine's, so the bundle it keeps for itself, sealed to a FIDO2 key, is opened with
    that key's PIN and signs the stick's boot program with the machine's db key; written again, the
    stick keeps its passphrase, typed back and proven against its old vault, and the bundle opens
    by its passphrase with the key still plugged in; the first write is signed for the writer's
    machine alone, and the second, asked to sign for every machine whose bundle the person keeps,
    opens another machine's bundle by its passphrase and is signed with both machines' db keys
  '';
  vm-hibernation-comes-back = ''
    a machine hibernated to a swap file on btrfs inside LUKS, with zram in front and no resume=,
    comes back from an lz4 image through systemd's initrd as it was, once the TPM and a PIN typed
    at its prompt open the disk, lets the lock go for that one prompt, and boots anew after
  '';
  vm-ir-camera-opens-for-readers = ''
    the IR camera waits for udev and its loopback, opens only while read, its frames grey, no frame
    from before a reader opened
  '';
  vm-keyring-opens-without-asking = ''
    the keyring opens with its TPM-sealed password, unasked
  '';
  vm-libvirt-stops-quickly = ''
    libvirt's daemon stops at once, its udev thread starved
  '';
  vm-lid-stays-at-the-desk = ''
    a shut lid sleeps with no monitor, keeps running with one plugged in whose screen is off,
    holding one lock through the card's changes, and sleeps once it's unplugged
  '';
  vm-loopback-says-who-holds-it = ''
    a loopback camera's file names each program another account holds it with, not its relay's unit
    however a program names itself, one line a name, catches a holder that forks, empties when they
    close it or the camera stops, keeps its folder, follows a new loopback, and takes no loopback of
    another name
  '';
  vm-machines-find-each-other-at-home = ''
    a machine on a network it trusts finds another there by its .local name through both
    firewalls, and nobody finds a machine with mDNS off, though the port is open
  '';
  vm-mv7-voice-is-levelled = ''
    an MV7's voice comes up 12 dB, its room and s sounds less, its peaks under -1 dBFS; another
    mic's are untouched
  '';
  vm-names-give-ipv4-when-ipv6-answers-first = ''
    a .local name whose answer over IPv6 comes first, which resolved then gives alone, still gives
    its IPv4 address, and both families, through the name service; and a program that has looked
    up an absent .local name still finds a present one
  '';
  vm-network-revives-units = ''
    what the network let down starts, and reloads on change
  '';
  vm-niri-is-traced-by-its-own = ''
    an account in wheel traces its own niri off the CPU with no password, each sleep said with its
    kernel stack, monotonic time and waker, and a wait in epoll with the file that woke it, with
    the graphics buffers reclaim backed up, read back and kept counted beside it, with the most
    kept at once; nobody else starts it or reads it
  '';
  vm-passkeys-come-from-the-tpm = ''
    passkeys and security keys are made in the TPM, each after a yes
  '';
  vm-pebble-speakers-are-tuned = ''
    a Pebble X Plus hears its tuning, again once it's plugged back in, and headphones don't
  '';
  vm-plugging-in-wakes-it = ''
    every USB 2 and 1.1 root hub may wake the machine, so a cable plugged into a sleeping laptop
    wakes it, and the SuperSpeed ones may not, so a USB 3 device pulled out doesn't
  '';
  vm-reboot-cannot-hang = ''
    a reboot forces on after a minute of a service not stopping, the watchdog resets a stuck last
    stage, and Plymouth hears which
  '';
  vm-screens-come-and-go = ''
    the launcher, a notice, the margin's marks and dictation's bar come back after the screen goes
    and returns, after a console switch and after a sleep and wake, after which the shell still
    hears a notice, dictation's keys and Keep Awake let go; when one of two screens goes, the
    margin and dictation's bar stay on the other and the margin comes back on both; the launcher
    opens, finds and runs a row on either of two screens as each goes and comes back, as both go,
    and after a sleep, its foot still telling the time after the wake; Quickshell warns of nothing
    meanwhile; and desk-facts stops naming the screen once it's gone and names it again once it's
    back
  '';
  vm-secure-boot-installs = ''
    the whole install works, and the disk opens only for it; the firmware starts the loader by
    an entry of its own, first in BootOrder; shut down with fwupd's firmware
    update waiting, it frees PCR 0, and the boot after opens with the PIN and locks PCR 0 again
  '';
  vm-speakers-are-tuned = ''
    the XPS 14's speakers hear the tuning, headphones don't
  '';
  vm-swap-comes-down-while-probed = ''
    the compressed swap comes down, its reset waiting out udev's probe
  '';
  vm-the-connection-stays-quick = ''
    under four uploads on an 8 Mbit/s connection 20 ms away, an app's ping stays under 80 ms with
    the shaper and passes 300 without; an app's upload leaves the jobs' under a quarter of what it
    sends; the shaper follows the connection down to 4 Mbit/s and back, and down again when the
    uploads ride a second link to the same router
  '';
  vm-the-desk-lights-come-back = ''
    a Litra plugged back in while a camera streams is told to light again, and the camera stopping
    puts it out; a Pebble X Plus plugged back in still has its lighting put out at a lock
  '';
  vm-the-network-keeps-its-order = ''
    an account's jobs, background, the system's builds and a guest it forwards for go in CAKE's bulk
    tin both ways, what Joel waits on at a terminal and an app's pages best effort, its calls, its
    UDP to Meet's media servers on 443 and Tuple's daemon video, even what it leaves to send as
    it ends, names and the shaper's pings voice; a call's packets turn real-time on; a slice made
    again keeps its class; the client's own network isn't slowed; inside a WireGuard VPN the same
    classes reach the VPN's own CAKE; the queues follow the default route, a link unplugged and
    plugged back in is shaped afresh both ways, udev sets nothing on the IFBs, and they go when the shaper stops
  '';
  vm-updates-itself-to-signed-main = ''
    an account in update-ask updates the machine to signed main with a real switch, reads how it
    ended from the file kept past the switch's restart where an account outside can't, and asked again
    with main where it was builds nothing; an account outside it can't ask, an unsigned main is
    refused, and a main the health check refuses is switched to and then gone back from; and a
    machine following its host's word builds main unsigned, root reading the objects someone
    fetched into their checkout and the token asked as someone with none of root's groups, with
    no token builds nothing, and building elsewhere with no builders refuses a piece not marked
    light; and a signed commit to a followed input's main alone moves the machine, an unsigned
    one is refused
  '';
  vm-webcam-microphones-yield = ''
    a webcam's microphone ranks below a USB microphone's, again once it's plugged back in
  '';
  vm-wifi-joins = ''
    wifid joins, moves past a silent access point, nudges a stuck look, finds sign-ins, keeps
    others' routing rules, keeps the card awake on the charger
  '';
  what-vm-tests-share-does-what-it-says = ''
    the helpers VM tests share run a command as the account asked, with its runtime folder and
    bus, read a journal's cursor or refuse output without one, and pace each wait to how busy the
    writer is as it starts, from as long as it says to four times that, describing the machine when
    one fails; a VM test written for pytest gets the driver's machines as fixtures, each test is a
    subtest named by its docstring, a failure fails it with pytest's account of why, and the
    driver ends as a failed subtest ends it
  '';
  whereabouts-passes-clippy = ''
    clippy finds nothing in whereabouts or its tests
  '';
  wifid-passes-clippy = ''
    clippy finds nothing in wifid, its sign-in window or their tests
  '';
  wifid-reads-the-air = ''
    wifid's tests pass: what it hears and offers, how each network is joined, the addresses it
    shows, what it keeps, and sign-ins
  '';
  wifid-sign-in-reads-its-network = ''
    the sign-in window's tests pass: its proxy lets on only the window, and it follows which card
    wants a sign-in
  '';
  without-syncfs-passes-clippy = ''
    clippy finds nothing in without-syncfs or its tests
  '';
  without-syncfs-refuses-only-syncfs = ''
    a program without-syncfs runs is answered "Function not implemented" by syncfs through a
    seccomp filter of its own while fsync runs, the filter refuses x32's syncfs too and lets every
    other call and another calling convention's through, and it says how it is used and what it
    could not run
  '';
  work-fills-swap-alone = ''
    the sorter gives each piece of work in jobs, awaited and timed work its own scope, so oomd ends
    only the one that filled swap, and pi's scopes share one slice weighted below a terminal's
  '';
  work-swaps-its-own-memory = ''
    jobs is slowed past half of memory, with no hard limit that would end its work
  '';
  work-yields-to-what-the-person-works-in = ''
    jobs is idle beside app.slice, pi finds it, and a window's scope is not idle
  '';
  wrappers-run-once-a-boot = ''
    a machine with measured boot keeps the SUID wrappers' unit active once it's run, so measured
    boot's wait for sysinit.target can't run it past its start limit
  '';
  writes-go-out-sooner = ''
    a machine with a screen keeps at most 128 KiB unsent in a socket, sends at full speed after
    idle, and asks for ECN with its fallback
  '';
  xdg-open-opens-each-kind-in-its-program = ''
    xdg-open opens markdown in the editor declared for text, in the terminal named for it, and a
    picture in its viewer
  '';
  yubikey-on-every-workstation = ''
    Linux and Mac workstations both have the YubiKey's tools
  '';
  zathura-accepts-its-configuration = ''
    zathura opens a page with the account's settings and stays open without a warning, and PDFs open
    in it
  '';
  zathura-keys-go-where-they-say = ''
    zathura's Left and Right turn the pages, Tab goes back to the place used last and again returns,
    and the question mark key asks the launcher for the keys
  '';
  zathura-says-the-document-it-has-open = ''
    zathura says the document it has open, read by its Recent's command as its process and whole
    path, and nothing once q closes it
  '';
  zathura-says-the-files-it-opens = ''
    zathura says each document it's started with to the desktop's list of files opened lately, then
    runs
  '';
  zoxide-lists-the-folder-visited-last-first = ''
    the account's zoxide lists the folder visited last first when asked, though another was visited
    more often, without writing its database
  '';
  zoxide-lists-without-writing = ''
    the account's zoxide lists its folders, the most visited first, without writing its database,
    and a visit still writes it
  '';
  zsh-is-the-login-shell = ''
    an account on NixOS logs in to zsh, as one on a Mac does
  '';
}
