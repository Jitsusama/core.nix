# What each check proves, by its name, as a sentence's predicate: checks.nix puts each in its
# check's meta.description, and every-check-says-what-it-proves fails on a check with none or
# one with no check. A check added names what it proves here, in its place by name. Long
# because it's one table, a line or two for each of core's checks.
{
  a-bluetooth-keyboard-may-wake-it = ''
    a machine with Bluetooth lets a USB or PCI controller of Bluetooth's class wake it, so a paired
    keyboard's key does, in a rule udev reads without a complaint
  '';
  a-card-says-how-it-stands-in-one-row = ''
    a card's status line, the launcher's and held notices' alike, gives its middle hints way first
    as room runs short, the keys list's and Return's staying
  '';
  a-claimed-chord-is-refused = ''
    a chord two layers claim, or one claims twice, is refused
  '';
  a-failed-boot-says-what-stopped-it = ''
    a failed boot's shell names up to four failed units and their last lines inside 80 columns, and
    the way out, and nothing when none failed
  '';
  a-faint-theme-is-refused = ''
    a theme with text under WCAG's 4.5:1, or text roles under APCA's Lc 75 for text and strong or
    60 for muted, or a mark's hue under Lc 45, on the grounds they're read on and a selected row,
    doesn't build, and APCA gives its own reference values
  '';
  a-file-no-linter-reads-fails-at-once = ''
    treefmt fails within a minute, naming them, on two files no formatter reads that git lists
    before thousands of others, where released treefmt waits on git for good
  '';
  a-file-reloaded-mid-read-is-read-again = ''
    a FileView reloaded while it's still reading reads once more as that read lands, however many
    reloads came meanwhile, so it holds what was written last; one reloaded idle reads once
  '';
  a-file-says-its-size-and-when-it-changed = ''
    a file in the launcher's chooser of files says its size in SI units, one decimal under 10, and
    when it changed by how long ago, a folder its count; its plate says its kind with its numbers,
    its preview and its facts, each number kept to its unit, and only its facts before it's said;
    every word is the launcher's words', and text past what's counted says it's over its count; a
    folder's entries order by name, change or size, folders first, 2 before 10; a save's name puts
    the caret before its extension; and one kept elsewhere, as in Google Drive, says the same on
    its row and plate from what its place listed, its place's facts after, leaving out what isn't
    given; a file nothing has read is its ending's kind, and a picture the shell draws is
    previewed as it is; a file in the Nix store goes by its own name past its hash, kept whole
    where the store's own listing holds it
  '';
  a-firmware-update-frees-pcr-0-only-for-fwupds-entry = ''
    the shutdown step takes PCR 0 out of the TPM's policy only when BootNext names fwupd's
    firmware updater, and leaves the policy alone for any other BootNext or none
  '';
  a-long-name-reads-at-one-pace = ''
    a selected name too long for its row slides at one reading pace, gathering speed from rest and
    settling back to it in at most a fifth of a second at each end, never stepping back; and words
    too long for the field's end give way at a whole word, no separator left before the ellipsis;
    and the field's path shares its room with a folder's path giving way first, down to its least,
    before the other steps share what's left
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
  a-mac-sshd-lets-a-gone-caller-go = ''
    every Mac's sshd asks a quiet caller every 15 s and lets it go after four unanswered
  '';
  a-mac-sshd-takes-only-keys = ''
    every Mac's sshd refuses passwords, typed or asked for, and never signs root in
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
  a-notice-keeps-its-links = ''
    a notice's body keeps its links, bare addresses included, and only the spec's bold, italics,
    underline and links as markup, everything else escaped as its sender's words
  '';
  a-capture-says-what-it-took = ''
    a screenshot's or a recording's drawer says where it is and what kind, when it was taken to
    the second, its size, and a picture's dimensions or a recording's length once they're known
  '';
  a-piece-says-all-it-knows = ''
    the drawers of Now's own pieces, the disk, the notices waiting, lag, a screen share, a
    recording and do not disturb, say where and what they are, then what moves first, and their
    identifiers last, leaving out what they can't say and what the row's side already says
  '';
  a-player-says-all-it-knows = ''
    a media player's drawer says everything its player tells MPRIS, what moves first and what
    identifies it last, leaving out what it can't say, and its art only from where it may be drawn
  '';
  a-program-says-all-it-knows = ''
    a program's drawer says where its windows are and what it is, then what it plays and what
    others say of it, the kinds of file and links it opens, its categories and keywords, where it
    starts, its version and package, and its command, window class, desktop id and store path
    last, from the programs daemon's record of its entry, leaving out what it can't say
  '';
  a-browser-window-deeper-in-is-its-programs = ''
    a browser's window of its own opened deeper than its program's own page is that program's, by the
    longest page of the same host and profile, never another program's on that host nor another
    profile's
  '';
  a-prompt-waits-for-a-person = ''
    a D-Bus call from the account's shells, as run0 asks for polkit's prompt in, outlasts systemd's
    25 seconds
  '';
  a-quiet-file-says-nothing = ''
    Quickshell's FileView, told not to print errors, says nothing of a file that opens and refuses
    every read, as one not told still warns
  '';
  a-realms-link-key-in-a-file-stays-apart = ''
    where there's no TPM, a realm's link key is a file in a private folder outside the one the
    account's own agent loads, served on its own socket by an agent holding only it, and an account
    naming no realm has none
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
  a-row-kept-is-never-drawn-again = ''
    the launcher's list keeps a row listed before and now wherever it moves, a key listed twice as
    two rows, and draws afresh only a row new to the list, the same rows taking no step and a row
    falling or rising any way taking one
  '';
  a-sender-keeps-to-its-level = ''
    a notice reaches no further than its sender is kept to, however critical, and each level shows,
    counts and stays as it says
  '';
  a-session-behind-the-lock-waits-for-its-kin = ''
    a session handed over while the screen is locked, beside its sender in the sessions' kitty,
    waits once the lock is gone until the node says which window is the sender's, by the app id it
    opened with, then goes to the sender's workspace without the focus
  '';
  a-session-is-known-by-its-own-record = ''
    pi's sessions are those whose records name a process running on this boot since when they
    say, never found by a window's title, each with its window found by the app id it opened with,
    or else through the kitty it runs in, and gone once its record or its process is; and those to reopen are the last boot's until
    put off, then those another machine carried here, newest first, never put off
  '';
  a-shut-launcher-makes-no-document-rows = ''
    the launcher's Documents make no row while it's shut, as the clock's minute passes or a
    document comes, then hold every document once it opens, made once, and keep them, making none,
    once it's shut again
  '';
  a-silent-hub-is-driven-as-one-stream = ''
    the XPS 14's kernel drives an MST hub whose first branch didn't answer its link address as a
    single stream until the sink goes away, so the monitor behind it still lights
  '';
  a-stick-carries-apples-macos-apart = ''
    a stick given Apple's restore image lays it last, as a partition of its own named for its zip,
    with a usbmuxd that skips pairing and a line saying `guide revive`; core's own stick has none
  '';
  a-theme-changes-everything = ''
    another theme leaves none of the first's colours behind, but in the web kit's other look
  '';
  a-things-drawer-says-its-parts = ''
    a thing's drawer says each part as a fact, its acts left out, a holder of figures opened in
    place and a level's bar a stroke; a screen's says whether its mode is preferred or custom, its
    refresh to every digit niri has where its row rounds it, variable refresh, scale as a multiple,
    where it sits, its modes and its density to three figures, an off one only what it offers;
    the backlight daemon's files say whether the room sets the level and what it's on its way
    to past a gap too slight to move, the room's light with the lid shut or that none is heard,
    measured to three digits in lx or klx, and every point of its curve, dimmest first, kept
    where the daemon keeps it, its lux every digit in the same units; the machine daemon's
    backlight says its steps, those read back only where they differ, its stepping and driver
    only where the kernel says them, and its sensor its count grouped, its lux a count and an
    offset only where it isn't none; and the facts that identify a thing end its drawer, marked,
    each said once
  '';
  a-timer-is-read-from-what-is-typed = ''
    a timer's span and name are read from what's typed after "timer" however it's written, a span
    of nothing or past a week is unfit, the time left reads as a clock counting whole seconds up,
    a span is said in its largest units, timers are listed rung first, then by what each has
    left, and the stopwatch's count runs while it runs, reads to the tenth rounded down, and lists
    its laps newest first, each its own stretch; a timer's side says its time left, paused or
    when it rang, and Now holds timers while any runs, is paused or has rung, or the stopwatch
    runs, told by the one listed first and naming them in that order, the stopwatch last; Now's
    row says the one on, or of several how many run before the first's, "2 running", and after
    it where the first rang or is held, "rang 03:27 · 1 running", the count only while any
    runs, and the stopwatch alone gives no items, counted live from its side
  '';
  a-trusted-cable-finds-machines = ''
    a trusted cable's network is a stranger's cable with mDNS on, matched by its card and picked
    first, and a machine naming none gets no such network
  '';
  a-want-rings-low = ''
    a machine with pi plays pi's low bell struck twice when a session wants Joel, not the theme's
    high message
  '';
  a-watched-file-comes-back = ''
    the shell's status file hears a status once its folder is made, after the shell starts and again
    after it's removed, and says nothing while it's gone
  '';
  a-yubikey-touch-types-nothing = ''
    a YubiKey's keyboard interface is refused in the running system and in the initrd, so a touch
    types no one-time password at the disk prompt, and udev reads the rule without a complaint
  '';
  agents-work-runs-without-real-time = ''
    the jobs sorter takes real time from work under jobs.service, jobs-work.slice and
    jobs-timed.slice, and from nothing that holds Joel's own terminal work
  '';
  an-arrival-is-said-once-each-time = ''
    what comes true by itself is said once, and nothing of what stood as the shell started; a
    piece that tells something new is said again, its own earlier notice taken back first and no
    other's; one whose words alone change is said again in place, by its id, and not once Joel has
    put it away; and one that goes takes back whichever of its notices is up
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
    links are doing; says a device running low at its row's side; lists only devices paired,
    then Nearby, which has the radio look only while it's open and lists the named devices heard,
    strongest first, each with its signal in dBm and as a bar, then counts those without a name,
    which it lists by address when stepped into; and says what BlueZ answered a pairing or
    connecting that failed, in the launcher's words
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
    btop's Escape backs out a step, off a process chosen, and never ends btop, which q does, rather
    than opening its menu
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
  cadence-asks-when-a-target-slips = ''
    cadence asks the session keeping the cycle fast once about each missed target or overdue card,
    tells the steward when that session doesn't answer or is gone, tells an orchestrator of a
    session idle two hours by its log's last write, and revives a crashed orchestrator while
    leaving one that was ended
  '';
  cadence-passes-clippy = ''
    clippy finds nothing in cadence or its tests
  '';
  call-audio-runs-in-real-time = ''
    an app's audio thread can take SCHED_RR 8, below PipeWire's, for at most 200 ms at a time
  '';
  call-transcripts-writes-down-both-sides = ''
    a call's two recorded sides come out as one transcript, cut at each side's pauses, in the order
    they spoke, each turn with its speaker and its time in the call
  '';
  call-transcripts-writes-only-what-was-said = ''
    a call writes down only what was said: a loud noise with no speech in it and a silent far end
    give no turn and no bracketed sound
  '';
  calls-are-summed-up-only-where-the-model-runs = ''
    the XPS 14's account gives call-transcripts its summarizer, on the GPU, and a graphical machine
    whose hardware names no device for it gives none, so never fetches the model
  '';
  calls-passes-clippy = ''
    clippy finds nothing in calls or its tests
  '';
  calls-tells-a-call = ''
    calls counts a call app's running recording as a call, leaving out PipeWire's filters,
    dictation, the camera's videos and monitors; and a-call-is-on says a call is on once the same
    recording goes on two seconds later in any account's PipeWire, none for a moment's recording or
    with nobody logged in, and that it can't tell when a PipeWire won't answer
  '';
  cargo-builds-on-their-own-filesystem = ''
    a workstation account's cargo keeps each workspace's intermediates in its folder on the
    builds' own filesystem, the disk guard collects there two levels down, and the machine
    mounts it
  '';
  cargo-links-with-mold = ''
    cargo links an account's Rust builds with mold
  '';
  changed-checks-names-what-moved = ''
    changed-checks names a check added, changed, removed or broken by its derivation, fails on a
    broken one, builds those added and changed unless asked only to name them, the VM tests only
    with --build and without --no-vm, compares the system with the most checks unless told one,
    sends a commit origin has to a lent builder to build by itself and anything else, or what no
    builder took, builds here, and evaluates a commit's checks once; where nix-offload takes it,
    the whole run goes to `there --changed` naming this machine's system and a base as its merge
    base's commit; with a base origin's main doesn't hold, edits not yet committed, or nowhere
    taking it, it runs nothing and exits 75, and it runs here only with --here
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
  claude-plan-feed-passes-clippy = ''
    clippy finds nothing in spend-watch's feed for a Claude plan or its tests
  '';
  claude-plan-feed-says-how-the-plan-stands = ''
    spend-watch's feed for a Claude plan says each window the plan has as a cap of a share used,
    with its reset, labelled with the plan's name, as its contract example says; says nothing
    signed in as no fault, an expired sign-in as stalled without asking, and an answer with no
    usable window as an error; and reads at the watch's pace, backing off while the endpoint fails
    or asks it to wait
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
  collector-passes-clippy = ''
    clippy finds nothing in the collector, which keeps the traces programs send it, or its tests
  '';
  collector-stays-in-the-background = ''
    the collector runs in background.slice, restarts when it fails, and has a runtime folder for
    its socket
  '';
  console-font-passes-clippy = ''
    clippy finds nothing in the console's font tool or its tests
  '';
  daemon-client-passes-clippy = ''
    clippy finds nothing in daemon-client, the daemon and client the daemons' tests share
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
  cryptsetup-finds-systemds-plugins = ''
    the cryptsetup a person types reads a disk's YubiKey and TPM slots through systemd's plugins
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
  each-list-keeps-its-kinds-order = ''
    each kind of list the launcher shows keeps one order with nothing typed, A to Z only its last
    tie and numbers read by their value, a choice never moving when chosen, and each place that
    lists things sorts by its kind's order with no A to Z made by hand, through Order.sorted, which
    keeps rows alike in the order they came where Qt's own sort shuffles them
  '';
  each-machine-signs-with-its-hardware = ''
    a Linux workstation signs through its TPM, one with no TPM with Joel's key on his YubiKey and
    the agents' key in a file served where the TPM's would be, and a Mac through its Secure
    Enclave, all with one program and Joel's key by default, and signing alone, with no hardware,
    signs nothing
  '';
  each-mark-means-one-thing = ''
    no two of the theme's marks share a glyph unless they are named as meaning one thing, as heat
    and temperature are
  '';
  eero-completes-in-zsh = ''
    eero completes in zsh with its views and commands, each node, each device by its name or, where
    another shares it, its address, each profile, and --json after a view
  '';
  eero-passes-clippy = ''
    clippy finds nothing in eero or its tests
  '';
  eero-reads-what-eero-says = ''
    eero draws the home network from every answer in its contract and a short history exactly as
    home-network.json holds it, its times in UTC and a read not yet made as unknown, and tells eero
    failing, refusing and a session aged past refreshing apart; its daemon reads the network every
    30 seconds while anyone listens and every 5 minutes otherwise and the rest at their own paces,
    keeps the last figures when eero can't be reached, saying since when and why, and writes each
    listener the model on joining and on every change; it keeps 30 days of what moves, a sample at
    most every 25 seconds, and a coming, going or roam only between reads 10 minutes apart at
    most, and draws the last hour's series into a model a busy hour keeps under 350 kB; it times
    its own look at the internet, the gateway node only from home, and an outage from the first
    look out to the first back; it tells, as notices, the internet out and back, a node offline
    and back online, a device new to the home network, firmware waiting and changing, and an event
    no other notice says, each once and none before eero first answers or on its first answer,
    and keeps what each said, drawing 30 days of it into the model, at most 50, a history from
    before keeping its rows; and it sends the conferencing setting as a bare boolean, counted
    taken only once eero reads it back; every view's page matches its golden one, saying
    when eero isn't answering or the reading is old, colouring only on a terminal, each view's JSON
    is the record's own slice, and a watch draws its view again from each model the socket writes
  '';
  ergodox-on-every-workstation = ''
    every workstation can flash the ErgoDox, a Mac as much as Linux
  '';
  every-assert-says-what-was-wrong = ''
    every Rust assert! and every Python assert of anything but a comparison carries a message
    saying what was expected, but those still waiting for theirs
  '';
  every-card-is-the-one-card = ''
    the launcher, the held notices and a notice take their frame from the one card and stand their
    ink on its content, never drawing a frame, reading the inset or tinting a row's ground
    themselves
  '';
  every-check-says-what-it-proves = ''
    every check says here what it proves, and every description here names a check
  '';
  docs-name-what-exists = ''
    every doc's links and paths name what's there, but the exemptions named
  '';
  every-fixed-wait-says-why = ''
    every sleep in a test polls, or says what passes while it waits in a comment or by a named
    constant, but those still waiting to become waits on their events
  '';
  every-folder-says-what-it-holds = ''
    every folder says what goes in it, but those still waiting for their note
  '';
  every-long-file-says-why = ''
    every code file past 800 lines and folder of more than 40 entries says why it stays whole, but
    those still waiting to be split
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
  every-pattern-names-a-file = ''
    every path pattern merge-net's must-run.txt, ruff.toml's per-file ignores and treefmt's
    unread kinds past treefmt-nix's own give matches a file the repository holds
  '';
  every-script-meets-shellcheck-at-its-strictest = ''
    every shell script meets shellcheck's optional checks for a hidden failure, a set -e turned
    off, an unset uppercase variable and a case with no default, but those still waiting
  '';
  every-suppression-says-why = ''
    every noqa, type: ignore, shellcheck disable and qmllint disable says why, after its codes or
    in a comment just above, but those still waiting for their reason
  '';
  every-test-function-lives-once = ''
    no function in test code, in Python, shell, QML, JavaScript or Rust, is copied from another
    file, but those still waiting to move to the test library
  '';
  every-test-runs-or-fails = ''
    no test passes without running, by an #[ignore] with no reason, a printed skip, an early return
    or pytest's or QML's skips, but those still waiting to run where they can
  '';
  every-vm-test-keeps-to-the-pyramid = ''
    no VM test holds more than twenty subtests, but those past it today, each held to its count
  '';
  ex-reads-vims-command-lines = ''
    a : line is read as Vim reads it: its range, a command by its full name or fixed short form, a
    bang and what follows, :g and :v, the rows a range names, and Vim's own errors for what it can't
    take
  '';
  example-apple-laptop = ''
    a laptop on the MacBook Air M2, aarch64-linux with no TPM or Secure Boot, evaluates
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
  for-you-weighs-again-only-for-news = ''
    For You weighs what waits on Joel again only when the node says something new of it: each
    kind told again with only when it was heard moved weighs nothing again, no item or verdict
    holds when it was heard, and a quest that ends takes its item away
  '';
  for-you-weighs-what-needs-joel = ''
    For You weighs what needs Joel by what it blocks, what's due, what he can do now, its kind
    and how long it has waited, the heaviest first and ties as they came, says each row's
    strongest reason at its side, reads the looks folder's files into what waits and what's
    history, newest first, leaving out any it can't read, and gathers what other sources give it,
    as Passwords' one to-do, into what waits, weighed beside the rest, and what's done
  '';
  formatting = ''
    everything is formatted and passes the linters
  '';
  gcloud-broker-asks-as-the-named-account = ''
    gcloud-broker is installed only where a machine names a service account, and asks only as it
  '';
  gcloud-broker-passes-clippy = ''
    clippy finds nothing in gcloud-broker or its tests, which mint and keep tokens from a stand-in
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
  github-broker-answers-only-its-account = ''
    github-broker listens on a socket only its account may open, and runs in the background
  '';
  github-broker-passes-clippy = ''
    clippy finds nothing in github-broker or its tests, which mint a narrow token from a stand-in
  '';
  google-answers-on-systemds-socket = ''
    google serves the daemon interface on %t/daemons/google.sock, a socket systemd holds, made only
    once pi has signed in to Google, as google itself starts only then
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
    five minutes and keeping them while Drive fails, and Drive's own places, its Recent of every
    kind, Starred, Shared with Me and the shared drives he hasn't hidden, each by its own search
    and order with every figure a row shows and whether there's more, asked with the documents
    and one failing costing the rest nothing, makes a Meet meeting for its link when asked,
    only once Joel's sign-in grants Meet's scope, lists a Drive folder only when asked, the last
    few asked each told, folders first, a shortcut as what it points at, leaving out what it
    can't show and never asking of a folder no id names, and downloads a Drive file to open, pick
    or save, Google's own kinds as their office program's file, never opening one that runs, one
    to open or pick kept in the cache, readable only, and fetched again only once Drive changes
    it, the copies handed out longest ago let go past their room, each
    download asked lately told, and a file cut short leaving nothing behind, and sends what Joel
    saves through the picker up to Drive once it's written and left a moment, a new file and then
    new versions of it, never a second file for one it may not replace, saying which program
    saved each as the picker named it, keeping each staged copy and letting go of those sent a
    week ago, with what was kept beside them, and serves the same in the daemon interface, a record
    for each meeting, mail, document and the account, a watch told them all and then only those a
    change touched and those gone, of the kinds it watches, an act handed on as the old socket's
    asks are, on the socket systemd hands it
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
    first that doesn't, reads why a hibernation rolled back from the kernel's own lines (memory,
    the swap file gone or full, a driver or program by its name refusing, the writer failing, a
    wakeup, or nothing said), and rescues it: said with the lid open, each notice word for word in
    plain text, slept through with it shut when memory or swap ran short, otherwise tried once more
    with it shut, then powered off; sets the top level's hard memory floors aside and keeps them,
    and puts each back as it was; switches zswap off for the image and on again after, only when
    it was on; and says whether an image fits by the kernel's sums (half of
    memory less what it keeps back and twice the reserve, against what's in use less cache,
    slabs, the graphics pool's freed pages, the free pages waiting on each processor's list and
    the anonymous memory swap has room for); and keeps the processor's count when a restore
    begins, and says from it how long the restore took, in seconds
  '';
  hibernate-unlock-passes-clippy = ''
    clippy finds nothing in hibernate-unlock or its tests
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
    the strong text and none of its own colours, its words in the serif upright's cut, without
    asking the desktop for the theme its settings name, and markdown files open in it
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
  keeping-awake-holds-the-lid-only-where-it-sleeps = ''
    keeping awake holds the lid only where a shut lid on the charger would suspend, and tells the
    shell whether it does
  '';
  kernel-settings-hold = ''
    the kernel's patches apply and every setting holds
  '';
  kernel-settings-hold-apple-macbook-air-m2-a2681 = ''
    the MacBook Air M2's own Asahi kernel applies its screen patches and holds every setting
  '';
  kernel-settings-hold-apple-macbook-pro-m4-pro-a3401 = ''
    the 14-inch M4 Pro's own Aurora kernel holds every setting of Aurora's Apple fragment and core's
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
  key-counts-keeps-no-order = ''
    key-counts keeps a total of presses and holds for each keyboard, layer, modifiers and key and
    nothing more, the same file whatever order the keys came in, a key held until it repeats
    counting one hold, and a restart carrying on from the file
  '';
  key-counts-passes-clippy = ''
    clippy finds nothing in key-counts or its tests
  '';
  keys-speak-the-language = ''
    every surface naming its kinds has each act they hold, by the key language's keys, binds no
    key the language gives to another act and no act by a key not its own, in each mode its acts
    are pressed in, and Neovim's keys, its command line's among them, agree with the language;
    that the language gives no key or command name to two acts, names no command Neovim names
    otherwise, gives no surface's act an arrow the leader doesn't begin in normal or visual mode,
    lets the find line reach no change and Return no act that ends or starts something, gives
    h, l and Backspace nothing on a list, Escape nothing but stepping back, and guards no act
    that would never ask; that a modal layer of ours binds an arrow in normal only alone, as its
    letter's motion, and no layer of ours closes or quits by Escape; each way of
    straying shown to fail first, a leader's arrow and an arrow bound as its letter shown to
    pass, the reader leaving out a key it takes only while selecting among them
  '';
  kitty-accepts-its-configuration = ''
    kitty loads its files without a complaint
  '';
  kitty-draws-the-budget-mark-in-one-cell = ''
    kitty draws the robot pi's footer marks a budget with in one cell, the space after it left
    blank, though the mono face inks it wider than a cell
  '';
  kitty-draws-every-letter-whole = ''
    kitty draws each letter of limit and will as the letter alone, in regular, bold, italic and
    bold italic, so texture healing never gives one a glyph wider than its cell
  '';
  kitty-draws-the-ligatures = ''
    kitty draws !=, >= and -> as the mono face's ligatures with the account's fontconfig features,
    and |> and 1..=2 plain
  '';
  kitty-lists-its-tabs = ''
    Programs › kitty matches each kitty OS window to niri's window by order or a title only one
    has, lists every tab niri shows, the window used last first and its tabs and panes as kitty
    used them, says what runs in each by its group's leader through env and a session's mycelium,
    an ssh session's machine, each row's name, state, folder and drawer, goes to and closes one
    through its own kitty and opens a new window through the kitty used last or a new one; and
    reads ~/.ssh/config's machines but patterns and Match blocks and ~/.ssh/known_hosts' but
    hashed, marked and pattern lines, with each one's keys, other names and port, how many times
    and when zsh's history last reached each, orders them most used first, says each one's row,
    drawer and ssh, and orders fd's repositories by zoxide's last visit
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
    finds the threads it watches by name, of niri only the one its socket names, names in a call
    the worst-waiting thread of Tuple's daemon and of Chrome apart, Chrome's only while a page
    records the microphone, and in an idle window nothing,
    reads a stand-in's named thread from the kernel, and sums each generation by its load
  '';
  lately-follows-each-source = ''
    For You's Lately counts what's unread from the mail, Slack and the desktop's other notices,
    each until its own source says it's done, never the shell's own notice for a Needs You
    item or a session's want, and History says how each went, newest first
  '';
  launcher-completes-in-zsh = ''
    launcher completes in zsh, typed in a real line editor against a stand-in shell: its command,
    each row as its step is typed, the rows inside the steps typed so far, its flags anywhere, and
    nothing after a step that names nothing
  '';
  launcher-fits-what-it-shows-to-the-terminal = ''
    `launcher` lays what the shell shows out to the terminal's width, a count and then a side giving
    way by whole items and a name cut only at a word, inks it only where the terminal takes colour,
    in its sixteen unless it says it takes 24-bit, sets every row's drawer after the rows when
    asked, and says a miss and the nearest names in the launcher's words
  '';
  launcher-passes-clippy = ''
    clippy finds nothing in launcher, the launcher in a terminal, or its tests
  '';
  lid-docked-passes-clippy = ''
    clippy finds nothing in lid-docked or its tests
  '';
  linux-builder-collects-only-what-it-is-asked = ''
    the Linux VM's collection, run for a collecting key, collects all that's dead, or at most the
    GiB asked, or nothing for free, prints the store's free bytes, and refuses anything else
  '';
  linux-builder-lets-collecting-keys-only-collect = ''
    a Mac's Linux VM lets a collecting key in only restricted to its collection, never plainly as
    a builder, and is changed from the cached guest so it keeps room
  '';
  linux-builder-offers-the-features-it-is-given = ''
    a Mac's Linux builder offers benchmark and big-parallel and any feature it's given, on the Mac's
    line and in the guest's Nix, and with none given offers only its own
  '';
  linux-builder-keeps-a-changed-guests-time = ''
    a Mac's Linux VM given keys or features steps its clock with chrony at any time, past a second,
    in place of timesyncd, and one given nothing keeps the cached guest's clock
  '';
  linux-builder-keeps-room-in-a-changed-guest = ''
    a Mac's Linux VM given keys or features collects garbage once a sixth of its disk is free, back
    to five twelfths, and one given nothing keeps the cached guest's 1 GiB and 3 GiB
  '';
  linux-builder-lets-only-those-named-restart-it = ''
    a Mac's sudo lets each account named restart its Linux VM and Nix daemon without a password,
    by those two whole commands and nothing else, and gets no rule when none is named
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
    and writable, reads the connection as the default route's link and what it carried in
    megabits a second, and passes only when every step did and nothing stopped it; and a command
    given more than it takes, `run --help` among them, is refused before anything starts
  '';
  load-proof-passes-clippy = ''
    clippy finds nothing in load-proof or its tests
  '';
  loader-goes-first-in-boot-order = ''
    the boot step gives the loader an entry of its own on the ESP, never taking a past install's
    entry on another partition for it, makes it active and first in BootOrder, and writes nothing
    once it is
  '';
  m4-pro-kernel-describes-both-external-screens = ''
    the 14-inch M4 Pro's device tree describes both external screens' coprocessors, their firmware
    memory and their crossbars as its own ADT has them
  '';
  machine-holds-the-machine-ready = ''
    the machine daemon says hello on the socket systemd hands it, says every record of a watch's
    kinds at once and their rates as of now a moment later, a disk's rates, sensors, link and file
    systems found through what's mapped over it, a disk taken out as gone and programs ranked by
    their share and memory, the screen's backlight and the room's light told as they move, reads
    nothing once nothing is watched, and refuses an ask or a kind it doesn't know
  '';
  machine-passes-clippy = ''
    clippy finds nothing in the machine daemon or its tests
  '';
  machine-runs-beside-the-shell = ''
    the shell brings the machine daemon, on the daemons' socket, which systemd holds whether or not
    it runs, restarted by a switch
  '';
  machines-yield-to-what-the-person-works-in = ''
    system virtual machines are weighed as a build is, and the account's containers and session
    machines are idle
  '';
  marks-past-the-basic-plane = ''
    a mark past U+FFFF reaches the shell whole
  '';
  meaning-adds-only-below = ''
    the launcher's search by meaning, in Qt's engine, tells the daemon only the things that stay, by
    the words each is known by, and the parts places hold by their names, each name once and none
    a thing has, and adds at most three rows below what typing found, none found already and none
    the launcher no longer holds, each saying in the foot how it was found, a part meant standing
    where every place past its category would, and at most three pictures below those Recent
    found by their names and words, none listed already
  '';
  meaning-finds-a-picture-by-what-it-shows = ''
    the meaning daemon, on the CPU with the service's SigLIP 2 towers, looks at each picture in its
    folders and finds each drawn shape by words saying what it shows and no other, finds nothing
    for words showing none, hears and finds one saved while it runs, and after a restart looks at
    none again
  '';
  meaning-finds-a-thing-by-what-it-means = ''
    the meaning daemon, on the CPU with the service's model, finds each of the launcher's things by
    words sharing none of its letters, nearest first, finds nothing for words meaning none of them,
    and after a restart answers whole at once from the meanings it kept
  '';
  meaning-switches-to-the-new-one = ''
    a switch restarts meaning after reloading its unit, so the shell asks the new one
  '';
  memory-modules-passes-clippy = ''
    clippy finds nothing in memory-modules or its tests
  '';
  memory-modules-reads-the-firmware = ''
    memory-modules reads each memory device's locator, size, type, speed, maker and part from the
    bytes the firmware lists, leaves out an empty slot, and reads LPDDR5 past 6400 MT/s as LPDDR5X
  '';
  merge-net-catches-a-broken-merge = ''
    merge-net runs each VM test a head of main changed since the last it passed, four at once,
    as bulk where nix-offload there takes it, sends again only what no builder took, never runs
    one again where it passed, reverts the merge it finds broke a test that failed twice and
    holds switching until the revert is on main, runs a held test first at a new head and lifts
    the hold the moment it passes, quarantines one that failed then passed, or whose revert
    won't apply with its breaker as owner, reverts nothing for one quarantined at the round's
    head or main's newest, starts nothing while the machine is held, says
    which changes run their VM tests before merging, and lets a head merge whose same change
    passed before a rebase that moved none of its files
  '';
  merge-net-passes-clippy = ''
    clippy finds nothing in merge-net or its tests
  '';
  mona-sans-vf-is-the-whole-face = ''
    the graphical role's fontconfig, asked for Mona Sans VF by name as GitHub asks, gives the variable
    file with every axis and lists no other file under that name
  '';
  mpv-accepts-its-configuration = ''
    mpv plays a moment of video with the account's settings and its controller's without a
    complaint, its messages and statistics in the monospaced face and its subtitles in the serif
    upright's cut, and video files open in it
  '';
  mpv-keys-go-where-they-say = ''
    mpv's Left, Right, Down and Up seek where h, l, j and k do, a number before a key does it that
    many times and alone chooses nothing, Escape stops one, 0 and $ go to the start and the end,
    [ and ] alone change the speed, Z alone delays the subtitles, J and L do what j and l did, yy
    copies the frame as a PNG and y alone nothing, ? and F1 ask the launcher for the keys, ]b and
    [b, gg, G and a number before G go through the list, f and Escape go in and out of full
    screen, Space pauses at once, even before a ?, q, Q, Escape and Space w close nothing, ZZ
    closes it, : asks the launcher for its command line by mpv's socket, what mpv-command carries
    back does as it says (a time, :set, :y and :q, over a socket mpv names itself where it was
    given none), and a program running mpv with its own forced Escape always gets it
  '';
  mpv-says-the-file-it-plays-now = ''
    mpv keeps the file it plays now, read by the launcher's command as its process and whole path, never a stream, gone as it quits
  '';
  mpv-says-the-files-it-plays = ''
    mpv says each file it plays to the desktop's list of files opened lately, a stream as it was given
  '';
  mpv-says-where-it-stopped = ''
    the launcher's lister reads where the account's mpv stopped each file it quit part way through
    or held on its last frame, newest first with its length, when it was left, its size, whether
    it's a video or audio, and every option mpv kept for it, its speed, volume and tracks among
    them, nine at most, only the last read as watched whole, passing over a folder's redirect
    entry, a stream and a deleted file, and mpv started as Resume starts it plays on from there,
    and as Play From the Start does, from its start
  '';
  mycelium-links-what-a-machine-names = ''
    a workstation in a realm runs mycelium's node there, dialling each machine it names, as a user
    service on Linux signing in with the agents' key, or the key its realm names, and an agent on a
    Mac whose sessions sign commits with that key; one naming none runs it dialling nothing, and
    one in no realm has neither a realm nor a node
  '';
  mycelium-signs-a-macs-links-with-the-enclave = ''
    a Mac's node signs its links in with the agents' key from the Secure Enclave, through that
    key's own library and no agent, so the login session's agent never asks for an approval;
    a Mac without that key's module and Linux's node are given no such options
  '';
  mycelium-keeps-each-realm-apart = ''
    a machine joining a further realm runs a node of its own for it, with its own state, mesh and
    steer folders under the harness's per-realm roots and its own link key, apart from the home
    realm's agents key; its pis are told which realm is home
  '';
  mycelium-flows-one-way = ''
    a realm flow is kept one way by both realms' nodes, the sending realm's letting text out and
    the receiving realm's letting it in, and by a machine in the sending realm alone, so what it
    sends waits for a link; no node names a flow nobody declared, and one into its own realm or
    between two realms the machine joins neither of is refused
  '';
  mycelium-lets-a-link-key-only-link = ''
    each link key a machine's system names, on NixOS or nix-darwin, is authorized only to run
    mycelium's end of a link or an attach, as the realm and machine it names, with a terminal for
    the attach; a machine naming none authorizes nothing of mycelium's
  '';
  mycelium-runs-among-the-sessions = ''
    on Linux, every realm's node runs in the agents' slice, beside Joel's programs rather than among
    them
  '';
  mycelium-serves-the-harness-files = ''
    each realm's node serves the harness's files of that realm, its open and ended quests, the
    sessions to restore, its runs of subagents and reviews, what its sessions landed, and the talks
    in its steer inboxes,
    from the realm's own folders, and the home realm's alone the machine's waiting, verdicts and
    queue, the queue's jobs and the queue whole from the login's runtime folder, each keyed as
    the launcher reads it; and each node restarts when what it serves changes, since it reads
    that only as it starts
  '';
  mycelium-declares-what-a-machine-says = ''
    a machine declaring what mycelium's node can't measure (its reach, what a held session
    outlives, whether work costs money, its taints, its agents by realm) has it written where
    mycelium reads it, in mycelium's words; one declaring nothing has no file, and agents in a
    realm it doesn't join are refused
  '';
  needs-you-is-true = ''
    over the agents' own files as the node serves them, For You's to-dos hold now only what Joel
    can do now, a timed one first
    and then the quickest, puts what he can't yet or has put off under Later, saying what each
    waits for, keeps gone what he or the to do daemon found done, says when nothing watches one,
    keeps nothing of a quest gone from the index and drops nothing by its age, and counts a
    session's want as his only while its quest has something ready or the question is his
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
  neovim-says-what-it-has-open = ''
    in a kitty window, Neovim says where its own server listens to the window once its terminal
    attaches, and nothing once it leaves, and open-now asks it there for its files and tabs
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
  niri-centres-a-window-alone = ''
    niri's layout says always-center-single-column, so a window alone on its workspace sits in the
    middle of the screen
  '';
  niri-cursor-has-its-shapes = ''
    the cursor theme niri names has the shapes programs ask for
  '';
  niri-is-asked-for-full-range = ''
    niri's unit asks for full-range RGB on a screen whose EDID takes it, by the name its patch reads
  '';
  niri-is-asked-for-panel-saturation = ''
    niri's unit asks for a laptop panel's saturation only where a machine sets one, by the name its patch reads
  '';
  niri-names-each-screen-once = ''
    niri's config names each screen in one output block, the panel's holding srgb-gamut, and refuses a
    second block for a screen, which niri would ignore
  '';
  niri-never-shows-its-own-screens = ''
    no niri key opens its screenshot screen, its key list or a quit that asks, and the launcher's
    Log Out quits without asking
  '';
  niri-patches-add-no-settings = ''
    no patch of niri's adds a setting a niri started before it would refuse at a switch, all of its
    settings with it, but srgb-gamut's, in every niri booted since, and opens-next-window-on's
    action and opens-next-window-as-column's field of it, which only IPC takes
  '';
  niri-refreshes-once-a-frame = ''
    the session's niri, nested in the sandbox, walks its windows at most once a frame while a
    client's commits wake it hundreds of times a second
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
    and none after one given back in use or one that stood and then went;
    only builders it asked for are ever given back; a stop ends at once, and a reload once
    nothing uses or wants its builder, leaving its builder for the next start, which takes it
    back before the deadline, keeping its price and idle time, or gives it back, priced to
    then or its own idle delete; standing builders are named while they answer, their names
    asked of resolved itself so one gone can't hide the rest, and only to builds that ask for
    them, never in the file every build reads, while builders beside are named in that file
    while they answer, for their big work alone, and never with the standing ones, and alone
    as themselves in a file of their own that's empty while none answers; the one lent is also
    named alone with nothing mandatory, still offering nothing a VM test needs, in a file for
    builds that may send any piece away, never with a builder beside or standing, and emptied
    while none is lent, unless the second lent builder, else the warm VM builders, passed their
    last proofs and stand in, until the one lent passes again; a proof builds a derivation no
    store holds, over ssh with the store's key, host key and a 10 s connect timeout, or through
    the daemon in slots of its own for a builder beside, runs one at a time a minute apart, and
    fails at its time limit, ending what it started; a lent builder failing two in a row is named
    nowhere, its relay ended and given back as lost, a builder beside failing two is left out
    until one passes, and each folder's health.json says how they stand; a day's journal sums
    to its hours and cost, each at its price; and
    `there` sends a flake's build to the lent builder as one command, returning its status and
    journalling what ssh sent, waiting afresh for each loss after the build sent again ran a
    while and for a lender restarting, saying how long, or, when none stands or none answers,
    builds nothing and exits 75, saying so, unless --here asks it to build here; with
    --here-when-idle it builds here only while this machine's CPU pressure has stayed under
    10% and never a VM test, else sends it as ever; with --bulk, its VM tests waiting for a VM
    builder's room let any run's that isn't bulk go first, waiting or coming; a tree's
    uncommitted changes go as a commit of what its index holds, files staged or added with -N
    included and untracked ones left, with the branch and index untouched; with
    --print-out-paths, what was built is copied into this store from the builder and only then
    printed, and a copy that fails fails the build;
    `there --changed` runs a tree's own changed-checks on the lent builder, else only
    evaluates on the first evaluator that answers and takes the repository's owner, and
    builds what it names as `there` does, exiting 69 rather than handing it back when no builder
    takes it, else tells changed-checks to run here; and `relay` dials a far side again
    while it closes before saying its first line, passes that line and everything after
    both ways, and hangs up once, without dialling again, on one that says nothing in time,
    and ends once its far side has answered no caller for long enough, never while idle; and
    `lines` copies into root's files only the lender's lines whose URI, whole, key and host key
    a standing builder's settings give, or a lent builder's instance does, named by prefix and
    zone and its host key asked of the cloud once, each line of eight fields and the rest plain
  '';
  nix-offload-lenders-outlive-a-switch = ''
    a switch that changes a lender's unit reloads it rather than restarting it, and a lender
    that ends superseded is started again from the new unit; every VM lender, and no other,
    warms each builder it stands, by default with a check core still has
  '';
  nix-offload-lines-reach-root-alone = ''
    with nix-offload's lines set, root's Nix reads its builders only from the folder their own
    account writes, the service runs as it with its settings, and the lender's account needn't
    be trusted; unset, root reads the lender's own and its account must be
  '';
  nix-offload-passes-clippy = ''
    clippy finds nothing in nix-offload or its tests
  '';
  nixos-role-twice-is-once = ''
    importing a NixOS role twice leaves the machine unchanged
  '';
  no-name-says-a-replaced-word = ''
    no check's name and no jitsusama.* option's says a name docs/glossary.md replaces, but the
    other senses and the renames still waiting that tests/repository/names-kept.nix keeps, each printed with
    its word, and the check refuses a stand-in name breaking each of its rules
  '';
  noise-cancelled-cleans-a-noisy-clip = ''
    a person's voice recorded over a room's noise comes out of the noise-cancelled denoiser, on
    the CPU, with the noise past it at least 10 dB quieter and the voice within 6 dB of how it
    was spoken
  '';
  noise-cancelled-switches-to-the-new-one = ''
    a switch restarts noise-cancelled after it reloads the units, so the microphones offered are
    never the old program's
  '';
  nothing-names-its-own-look = ''
    no module but the theme's names a colour or a face
  '';
  nothing-spells-a-mark = ''
    no module but the theme's spells a mark's code point instead of naming it
  '';
  notices-stand-on-the-cards-columns = ''
    a notice drawn on the one card, as the pop-up does and as the held list does with a gutter,
    holds its ink the inset from the frame on every side, at 2 and at 1.5, with its mark centred
    on the title's capitals and its text column moved by the gutter alone
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
  onepassword-answers-op = ''
    1Password's command-line tool, op, runs setgid in the onepassword-cli group, so the app answers it
  '';
  onepassword-brings-ops-zsh-completion = ''
    the account's zsh is given op's own completion, the file the op the system wraps was built with
  '';
  onepassword-manager-passes-clippy = ''
    clippy finds nothing in 1Password's manager for Passwords or its tests
  '';
  onepassword-manager-passes-no-secret-on = ''
    1Password's manager for Passwords says an item op gives as an entry with no hidden detail's
    value and no password's history, reads a secret only for the one detail asked, says
    1Password's categories and kinds in Passwords' own words, names each entry's account from
    op's own, for 1Password's link to it, reads each login's password alone from op's many
    items for an audit, and, run against a stand-in op, says it's refused by Integrate with
    1Password CLI while 1Password's settings have it off, and only that 1Password asks to let it
    in the moment it's turned on
  '';
  onepassword-offers-its-quick-acts = ''
    the account's 1Password entry offers Quick Access and then Lock 1Password, through switches the
    installed 1Password takes
  '';
  onepassword-starts-with-the-session = ''
    the session starts the system's 1Password without its window, through a switch it still takes
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
  passwords-completes-in-zsh = ''
    passwords completes in zsh with its commands, audit among them, each entry the daemon holds by
    its title, a held entry's details by their labels, each password manager and --json, and no
    Tab asks a password manager anything
  '';
  passwords-holds-no-secret = ''
    the passwords daemon holds every entry each password manager lists without a secret, even one
    given with its secrets on, answers a held list or get without asking a manager, gives a secret
    only to the client that asked to see it or to the clipboard marked sensitive and cleared when
    its time is up, says a one-time code's period, gives a one-time code's detail as its code and
    never its seed, says a manager signed out and gives no secret from it, tells a watch of entries as they arrive, and refuses an ask or a kind it doesn't know;
    audits every password only when Joel asks, refusing otherwise and for a manager signed out,
    counting the entries that share one and matching each hash's suffix against a stand-in Have I
    Been Pwned's range, padding counting nothing, sending it five characters alone and saying no
    secret in an answer or a log;
    keeps what an audit found in the state folder for Joel alone, holding no secret nor any hash,
    dresses the entries with it again once the daemon starts again, holds since when trouble was
    found until a check finds none, and tells a watch each change;
    and the passwords command sets each answer in columns under a header for a person and
    tab-separated for a program, gives the daemon's answer with --json and no secret but show's,
    finds an entry by its title and a detail by its label, lists an audit's entries with a
    problem, and exits as its usage says
  '';
  passwords-lists-1passwords-ssh-keys = ''
    passwords keys lists the keys an SSH agent holds at 1Password's socket, each by its type, size,
    fingerprint, comment and public half, as rows and as JSON, nothing for an agent holding none,
    and says how to turn the agent on when none listens, against a stand-in agent alone
  '';
  passwords-passes-clippy = ''
    clippy finds nothing in the passwords daemon or its tests
  '';
  passwords-runs-each-password-manager = ''
    on a machine with 1Password, the passwords daemon runs 1Password's manager through the system's
    op on the daemons' socket systemd holds, copies with wl-copy, writes no core dump and has a
    state folder of its own for what the checks found, and the launcher's Passwords knows
    1Password's program
  '';
  pause-for-calls-follows-the-calls = ''
    pause-for-calls pauses Spotify two seconds into a call, as calls counts one, and plays it five
    seconds after, a moment's recording pausing nothing and a reconnect keeping the call, and finds
    each Spotify on the bus
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
    signed out, asks one back from a sign-out, or from saying later, again about everyone watched
    it hasn't said once it says who someone is, starts a source that stops again and asks it about
    everyone watched, says a get nothing answered in time isn't ready, and refuses an ask or a kind it doesn't know
  '';
  people-passes-clippy = ''
    clippy finds nothing in the people daemon or its tests
  '';
  people-runs-where-a-source-names-a-program = ''
    the people daemon runs for Slack's, Tuple's and Google's sources on the daemons' socket, which systemd
    holds for it, restarting as the new one after a switch, and neither it nor its socket is
    there where no source names a program for it
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
  pi-answers-a-tool-it-has-not-loaded = ''
    on Claude Code's provider, a call to one of pi's tools that isn't loaded goes to pi to answer as
    not found rather than ending the turn, and a broken relayed stream says what broke it
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
  pi-carry-is-on-the-path = ''
    pi-carry, the harness's program mycelium carry runs to carry a quest to another machine and put it
    back, is on the path of every account with pi, run by the store's node
  '';
  pi-keeps-its-folder-private = ''
    pi's own folder and its session logs' folder are the account's alone on every machine with pi, so a
    token a tool printed into a log isn't readable by another account
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
  pi-hosted-keeps-turns-its-own = ''
    pi built with the host's patches, applied exactly, shows both seams a host detects, builds a
    session's agent from pi's own options through the host, and a turn its durable generation
    commits waits for the host to end or continue it, once, across recovery
  '';
  pi-hosted-keeps-each-conversations-keys = ''
    pi built with the host's patches, applied exactly, marks a model runtime that reads keys and
    headers through an auth context, gives each runtime the key, provider headers, model headers
    and OAuth headers its own context holds, never falls back to the process for one its context
    lacks, refuses header variables where it can't read the context, and reads the process for a
    runtime given none, as ordinary pi does
  '';
  pi-hosted-screen-reaches-its-viewers = ''
    pi built with the host's patches, applied exactly, copies (a fullscreen selection too)
    through a hosted screen's viewers' terminals, sends a viewer who joins late the images
    already on the screen, and edits a draft, an extension's panel's too, in the external
    editor of the viewer that asked through its host, keeping the draft and saying why when
    the edit fails and refusing it in place when the host can't
  '';
  pi-hosted-screen-stays-in-its-session = ''
    pi built with the host's patches, applied exactly, ends only a hosted screen when it quits
    or crashes, suspends only the viewer that asked when its host can and refuses otherwise,
    binds nothing after quitting during startup, sends no more of its first words once it
    quits, builds nothing an extension asks of it once stopped and tells every screen of a
    theme change, while ordinary pi keeps its process exits
  '';
  pi-hosts-start-with-the-session = ''
    each realm a machine names gets an attended and a background pi host, started with the
    account's session where the harness is installed, run by the host's pi and its own node, ready
    once it says so, in agents.slice with its realm's environment and its folders first on its
    PATH, left stopped after refusing
    what it was given and left running through a switch, and a machine naming none, or a Mac,
    gets none
  '';
  pi-hosts-refuse-an-unproven-runtime = ''
    a realm's pi reaches its hosts only with the proof that pnpm test:host passed on the runtime
    it is in; a proof of another runtime, or one that isn't a proof, fails evaluation, naming the
    realm and how to prove it
  '';
  pi-rings-low = ''
    the account's ding rings loud for seconds, as the harness's done chime plays it, and the
    shell's want sound rings loud, both under the theme message's pitch
  '';
  pi-rings-when-a-run-is-done = ''
    pi's done chime is told a ding and the store's pw-play, and no voice or want sound, on Linux,
    and a Mac is told nothing, ringing with its own
  '';
  pi-runs-on-the-machines-plan = ''
    a machine on the Claude plan has one pi, starting on the claude-code provider and Sonnet with
    no pi-claude beside it, and a work machine's pi is unchanged
  '';
  pi-starts-on-opus = ''
    off the Claude plan, pi's settings on Linux and on a Mac start every session on
    anthropic/claude-opus-5-5 at high thinking, so each switch writes that default back
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
  polkit-takes-a-yubikey-touch-once-one-is-named = ''
    once a machine names a YubiKey, polkit's sign-in takes its touch before the password, from a
    root-owned file, and no other login does; with none named, polkit asks for the password alone
  '';
  processor-telemetry-passes-clippy = ''
    clippy finds nothing in the processor telemetry reader or its tests
  '';
  programs-holds-whats-installed = ''
    the programs daemon says hello on the socket systemd hands it, says an entry whole from the
    nearest data folder with each file kind's globs and description, says an entry changed again
    and one removed as gone, reads a program's list as its watch begins and as its file changes and
    not while nobody watches, says a list whose command fails as empty and why, reads what a
    program has open as it's asked, follows a data folder through each link a switch replaces, and
    refuses an ask or a kind it doesn't know
  '';
  programs-passes-clippy = ''
    clippy finds nothing in the programs daemon or its tests
  '';
  programs-runs-beside-the-shell = ''
    the shell brings the programs daemon, on the daemons' socket, which systemd holds whether or not
    it runs, given the sources each program's module lists, restarted by a switch
  '';
  programs-nobody-starts-stay-unlisted = ''
    the entries of virt-manager, the NixOS manual and umpv are masked as hidden where the launcher
    reads them, while the programs stay installed
  '';
  programs-live-in-their-own-files = ''
    no shell, QML, Python or JavaScript past five lines is written as a Nix string, but a short
    splice of values, one that says why it's kept in Nix, and those still waiting to move out
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
  quickshell-keeps-to-the-space-scale = ''
    no padding, margin, gap or line in the shell or a module's surface is a step nudged by a number
    off the space scale
  '';
  quickshell-passes-qmllint = ''
    qmllint finds nothing in the account's shell or the greeter, leaving out only types Quickshell's
    files don't export and Qt enum parameters
  '';
  quickshell-restarts-with-its-files = ''
    a switch restarts the shell when a file it reads changes
  '';
  quickshell-takes-its-shape-from-the-theme = ''
    every corner and border in the shell and in each module's surface is the theme's
  '';
  reader-passes-clippy = ''
    clippy finds nothing in the reader or its tests
  '';
  reader-says-the-file-it-shows = ''
    the reader keeps the file it shows, read by its Recent's command as its process and whole path,
    and nothing once q closes it; what a killed reader left is cleared as it starts
  '';
  reader-says-the-files-it-opens = ''
    the reader says each file it's started with to the desktop's list of files opened lately, then
    runs as it would
  '';
  reader-says-the-keys-it-takes = ''
    the keys the launcher lists over the reader's window are the reader's own, chord for chord, with
    what each does, its group and its act in the key language
  '';
  reader-sets-its-pages = ''
    the reader's tests pass as it builds: tables from their words and on the text's pitch, pictures
    by their first bytes, places kept and read back with the heading each sits under, how far down
    it is and when it was left, an old place without them still read, the places listed newest
    first without a file gone, a mark opened by its letter, saves heard and the watch moved to
    another file,
    the file shown said, the theme read, search across a hyphen, an equation shown as written
    without MiTeX, links to markdown set in place, labels on their links' baselines, Neovim's
    counts and keys, visual mode's motions, text objects and copies of the file's markdown, where
    vi's paragraph, heading, view, bracket and file moves land, whole-word search, the command line
    read as Neovim reads it, and gO's outline
  '';
  reader-takes-its-keys = ''
    the reader draws a long page in the theme and takes Neovim's keys, G, gg, a count, Ctrl+F, Ctrl+O,
    zoom, / and n, Space ? and Escape, marks the cursor's line with a rule in the accent, copies the
    block at the cursor, its paragraph, that with its blank line and to its end with y y, y i p,
    y a p and y }, moves the cursor and not the view with j and k and the view alone with Ctrl+E and
    Ctrl+Y, selects from the cursor's line with v and V and every visual-mode key, v's cursor a
    block in the terminal's cursor colours on a title's letter, copies each selection's markdown
    onto the clipboard with y and Ctrl+C, a paragraph the page wraps as its one line, and follows
    a link by g x's label, to a heading, putting the cursor on it, and to another markdown file set in the same window, Ctrl+O and Ctrl+I across the two, then vi's [b ]b { } ]]
    [[ H M L zt zz zb % * # gO gf and Ctrl+], :noh, :5, E492 and E481, copies the file with :%y
    and a selection's lines with :'<,'>y, says E16 past the end, edits the line with Ctrl+W,
    Ctrl+U and Ctrl+H and drops it with Ctrl+C, keeps the search's marks on
    Escape at rest, says E21 for every change (d d, c c, S, x, p, d and c with a motion, :d, d on
    a selection), keeps a mark between openings, one on a heading's line under that heading and
    one on the last line saying its share of the whole page, goes between files with :bp, :e and
    :bn, says
    E484 of a file that isn't, lists the keys on :h, lists the lines :g/pat/ matches, the marks on
    :marks and the jumps on :ju, going to one on Return, a chosen row too long for them reading
    on in place, shows the command line's menu on :, narrowed
    by what's typed and chosen from with Tab, Shift+Tab, the arrows, Ctrl+N, Ctrl+P, Ctrl+E and
    Ctrl+Y, the files after :e, runs the last line again on @: and an earlier one from :his, and
    closes on :q; markdown files open in it
  '';
  record-window-passes-clippy = ''
    clippy finds nothing in record-window or its tests
  '';
  reopening-finds-each-screen = ''
    Reopen Them All puts each session back on its own screen, found by its maker, model and serial
    rather than its connector, and on the one it was kept on of two alike, a gone screen's
    workspaces after the one in use's and then to it, each to its rank, once it's back, and each
    window that reopens itself by its program and title, beside the windows kept with it, under
    the one kept above it in its column, and a window already where it goes left there; and those
    that orchestrate are begun first, the next session only while the machine queue says the
    memory and the processors have room and the disk isn't stalling, a queue unread or gone quiet
    holding nothing back
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
  sccache-shares-builds-across-worktrees = ''
    sccache's server runs among the jobs, started as its unit starts it, and two workspaces in
    folders of their own build one dependency through what an account's shells and user manager
    are given, finding that server's socket, and the second's compile of it comes from the cache
  '';
  screen-text-finds-a-screenshot-by-its-words = ''
    screen-text's daemon, on the CPU, reads a screenshot moved into its folder and finds it by its
    words, with the line that matched, giving its lines by key; lets its record go once it's
    removed; and after a restart reads nothing it read before
  '';
  screen-text-reads-a-screenshot = ''
    screen-text, on the CPU, reads a picture drawn as the check builds, larger than the detector's
    tile, giving back its heading and a row of two lines apart, each whole and in reading order
  '';
  screen-text-switches-to-the-new-one = ''
    a switch restarts screen-text after reloading its unit, so the shell asks the new one
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
    an account naming a folder, or one of the backup's own, backs its sessions up hourly in
    background.slice, catching up on an hour slept through, trying a failed run again and left
    running by a switch, and one naming none runs nothing
  '';
  session-machines-have-a-bridge = ''
    libvirt's bridge helper lets session machines onto virbr0, and its default network starts at
    boot
  '';
  session-memory-is-protected = ''
    session.slice and app.slice carry hard memory floors and agents.slice a soft one, and every
    slice above them carries one hard and one soft floor, the same at each
  '';
  sessions-sit-beside-the-programs = ''
    agents' sessions have a slice beside Joel's programs at their weight, and the sorter watches both
  '';
  shared-work-ranks-with-the-sessions = ''
    work other sessions wait on has a slice beside jobs at the sessions' disk target, below Joel's
    programs on the processor, with a memory ceiling, named in SHARED_SLICE
  '';
  shaper-finds-the-rate = ''
    the shaper reads a queue from delayed pings, not one late ping or a silent host; cuts under what
    the connection carried, holds while it drains, and climbs while it's quiet, past what it held
    only once busy five seconds, so bursts fit under the connection; forgets a cut ten quiet minutes
    old, and keeps one the connection then proves; cuts no download arriving faster than its rate,
    nor beside one an upload carrying under half the connection's best, nor again for a queue back
    within two seconds; climbs back from a cut no queue returns for, so late pings alone don't wear
    an upload down, yet retries no cut near one confirmed; never cuts Wi-Fi more than 5% under the
    best it carried, while following a cable down, unless a busy direction's queue stands six
    seconds at that floor, or 40% while real-time traffic flows, which a call's packets turn on for
    30 s; remembers the connection's best, not its latest cut; reads sets, routes and queues from
    what the kernel prints; replays recorded days as one stay per network across midnight, each
    second's carried rate from the counters and each direction's own delay voted from its
    reflectors, and names a cut with no queue of its own, a rate held low on an idle connection and
    one starved under a full one; the rebuilt controller cuts a direction only once its own delay
    has stood a second, a tenth under what got through, climbs a quarter a second until it nears its
    last queue and 5% after, takes the air's lateness and an idle direction's path off, leaves the
    connection unlimited with no timestamps, and goes back to its base when a queue outlasts half a
    minute of cuts; live, it answers what a replay of the same recorded lines does, read as the day
    files grow and across midnight, for the link the recorder names, and lets the link go unlimited
    when the recorder falls silent or begins a new stay; and on sixteen simulated connections, clean
    and rough, from a known base, none or a wrong one, it holds the review's bar against the same
    connection unshaped: a p95 delay of 100 ms or half the unshaped one, 75% of each way carried,
    and no direction under half 30 s longer
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
    sign-in or with none at all, saying so; and slack-unreads holds over its socket the direct
    messages with unreads, the channels mentioning Joel, his mentions in threads and channels
    merely unread, each named once a run and opening in Slack, and what Joel read last and how
    Slack stands, as records listed and watched, each said to a watch only as it changes and as
    gone once read, only the account held, as signed out, while Slack refuses the sign-in, and
    with no sign-in nothing held and nothing asked; holds each message wanting Joel, those
    unread in a direct message, one never opened too, and those in a channel mentioning him,
    never his own nor a thread's reply, each read from its conversation's history once as its
    unreads come, page by page, with its web link, and said done, read in Slack, once Slack's
    read mark passes it, beside that mark, the account saying them whole only once every
    history wanted was read; and says each message a search finds by
    where it is, who said it and its first words, the messages around one oldest first, and
    Slack too busy apart from signed out or out of reach
  '';
  slack-message-is-one-row = ''
    a Slack message is one row whether For You holds it or Search Slack found it: one key, named
    by who said it, Joel's own as You, its first line after its name or, found past it, the
    message from the matched word on, its eyebrow where it is or that it's a direct message, and
    Return opening the message itself, else its conversation
  '';
  slack-passes-clippy = ''
    clippy finds nothing in slack-people or its tests
  '';
  slack-unreads-go-with-the-service = ''
    slack-unreads starts once pi has signed in to Slack, restarts as the new one after a switch,
    is restarted when its watchdog hears nothing, and takes its ask in For You with it however
    it stops,
    while its socket, systemd's and held only once pi has signed in, stays
  '';
  slack-waits-in-its-place = ''
    Slack's source in People asks about each address once a minute as Joel steps in, never one
    known to be nobody, and ahead about those listed, in order, each once; Write in Slack stands
    dimmed, looking in Slack, until it's answered, or not in Slack once a step-in found nobody,
    and isn't listed for someone already known not to be there; answers that come together are
    taken as one change, and those already known as none; their conversation is found by any of
    their addresses, its drawer saying who said each message, newest last
  '';
  slack-reads-a-notice-read-there-once = ''
    Slack's own desktop notice of a message Joel read in Slack is read in Notices once, said as
    read in Slack, after the change that found it, so the shell logs no binding loop doing it,
    and a change remakes only the lists whose keys it names, a message heard leaving the
    conversations and Recent as they were
  '';
  slack-says-only-whats-current = ''
    Now's Slack piece gathers what Slack's daemon holds in Slack's order whatever order it was
    heard in, a change moving only the parts whose keys it names, and counts what wants Joel only while the daemon answers and Slack answered it,
    saying Slack can't be reached when it can't, and signed out once Slack refuses the sign-in;
    it's listed only with something to read, never with nothing or the daemon not answering,
    naming the first two conversations and how many more, and Slack's own Unread counts the
    conversations it lists; Slack's own desktop notice of a message read in Slack is found by
    its conversation, sender, time and words, one to one, and none is found on a guess
  '';
  smartd-stops-cleanly-without-disks = ''
    the workstation's own smartd command, run with no disks, stops cleanly rather than failing
  '';
  sound-hears-only-what-can-be-heard = ''
    the launcher's Sound leaves out a jack with nothing plugged in and a display's output with no
    display, and never listens to a headset's microphone while it plays music; a device's drawer
    says where it is and which way it goes, its state, its volume in decibels, its parts, how it
    keeps time while it runs (dropouts warned past none, load to three significant digits from
    its busy time over the cycle or warned as over 100% where it didn't finish, a follower in
    its driver's cycle), its format, the kernel's facts, and its priority and identifiers last,
    never whether it's muted nor its level; and channels at one level are balanced, a stereo
    pair's otherwise named without front
  '';
  space-is-set-by-rung = ''
    no QML file of the shell's names a space by its step on the size scale, rather than by the
    rung of the space ladder it sits between, the beat or a level's pitch
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
  spotify-player-keeps-its-streaming-sign-in = ''
    spotify-player's authenticate signs the streaming client in with its token, saving the
    credentials it plays with
  '';
  spotify-player-signs-in-where-spotify-answers = ''
    spotify-player's sign-in asks ncspot's Spotify app to answer at the one address that app takes
  '';
  spotify-recent-passes-clippy = ''
    clippy finds nothing in spotify-recent or its tests
  '';
  spotify-recent-reads-what-played = ''
    spotify-recent lists what Spotify played lately one row a playlist, album or show, each with
    its size, nine at most, Liked Songs and artists left inside; walks Joel's playlists, albums,
    artists and shows page after page; refreshes a spent sign-in only once, under a lock, and
    writes it back 0600 in its own shape keeping its refresh token; finds a device to play on;
    and says signed out, nowhere to play and out of reach apart
  '';
  sshd-lets-a-gone-caller-go = ''
    every NixOS machine's sshd asks a quiet caller every 15 s and lets it go after four unanswered
  '';
  sshd-takes-only-keys = ''
    every NixOS machine's sshd refuses passwords, typed or asked for, and never signs root in
  '';
  stands-alone = ''
    nothing here names a machine, its repository or employer
  '';
  sun-passes-clippy = ''
    clippy finds nothing in sun, the sun's reckoning the calendars and night light share, or its
    tests
  '';
  surface-clock-passes-clippy = ''
    clippy finds nothing in surface-clock or its tests
  '';
  swayimg-accepts-its-configuration = ''
    swayimg draws a picture with the account's settings and the machine's fonts without a complaint,
    its words in the monospaced face, and pictures open in it
  '';
  swayimg-leads-with-a-gallery = ''
    swayimg's page in the launcher leads with the screenshots' gallery while it holds any, else
    the newest folder of its Recent's, and offers each of those folders once, newest first, saying
    how many pictures it holds and where and opening on Recent's newest still there, each
    gallery's drawer saying where its folder is, then its kind, previewing its newest picture and
    saying its name, when it was taken or added and how many there are, and its drawer says when
    the newest screenshot was taken and how many there are
  '';
  swayimg-moves-by-its-keys = ''
    in swayimg, pressed as a keyboard presses them, → and ← go to the next and previous picture and
    End, Home, G and gg to the last and first, l and h as → and ←, ]b and [b as well,
    a number before G to that picture and before a move that many, j and k move a large picture
    as ↓ and ↑ do, dd and Delete trash it and u brings it back, yy copies it,
    ? and Space ? ask the launcher for the keys, Escape stops a number or a pair waiting, Escape and
    q close nothing and ZZ and Space w close it, [ and ] alone turn nothing, : asks the launcher
    for its command line by swayimg's process, what swayimg-command carries back does as it says
    (a number, :d, :rotate, :y, :open and :q), the gallery answers the same keys, the picture
    it opened with, and none after, is said to the desktop's list of files opened lately, and the
    launcher's gallery opens on the picture it's given, newest first
  '';
  swayimg-says-the-picture-it-shows = ''
    swayimg keeps the picture it shows, read by its Recent's command as its process and whole path,
    and nothing once ZZ closes it, clearing what a killed one left
  '';
  the-apple-silicon-stick-evaluates = ''
    the stick for an Apple silicon Mac, aarch64-linux on Asahi's kernel and booted as BOOTAA64.EFI,
    evaluates
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
  the-command-line-lists-every-command = ''
    the launcher's : line lists the cursor row's acts, Return's first, then the surface's commands
    and :q :help :{n}, each in Vim's help notation with its title and key; typing narrows it by the
    command's start, then the act's words; Tab and the arrows choose and write a line in, round to
    what was typed; Return runs exactly what's typed, a short form or more, saying E492, E477 or
    E488 otherwise, each key's work over 400 commands within three times a bare copy of them or a
    pass over them, timed in turn
  '';
  the-console-greets-the-account = ''
    a console greets the account by its description's first word, or its name, with the keys back to
    the desktop, and keeps NixOS's banner with no account
  '';
  the-console-setup-yields-to-graphics = ''
    the consoles' setup is done with when a compositor takes the console for graphics while it's
    set, and fails as the setup does when the console still shows text
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
  the-drawer-over-the-card-meets-it-square = ''
    a floating surface over another, as the launcher's drawer over its card on a narrow screen,
    meets it with a square foot and region, its top and the card's foot staying round
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
  the-gpu-pool-never-stalls-for-big-pages = ''
    the XPS 14's patched TTM pool asks for the 2 MiB pages xe gains from without reclaiming or
    retrying in the caller, so the try only wakes background reclaim and falls back
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
  the-home-network-says-each-figure = ''
    the Home Network place lists the gateway node first and each node after the one it hangs
    from, its devices under it, and says every figure the eero contract places in the internet's,
    a node's, a device's and a profile's drawer, each in its own unit, none eero didn't give, the
    internet's timeline setting each notice the daemon told among eero's events, newest first
  '';
  the-kernel-boots-with-its-modules-keys = ''
    a machine with Secure Boot boots a kernel image carrying the certificates of the keys its
    modules are signed with, in bootspec and the system's kernel link alike
  '';
  the-keyboard-light-is-heard = ''
    the shell finds the keyboard's light among the kernel's and hears each change its firmware
    makes, once, with its level, off as dark
  '';
  the-launcher-acts-only-on-what-it-can = ''
    a program's act on a copy shows only while the clipboard holds what it takes, a picture, one
    web address, or an existing file's path, markdown for the reader and a document for zathura,
    with its line and column, found from the top only while it shows, and opens it: Neovim in
    the kitty used last or a new one, at the line and column, Chrome's page in a new window, the
    reader's file, zathura's document at the page copied after it, and GIMP's
    picture in a file of its own, copied from the clipboard's history or written by wl-paste;
    and Open a File by Name and Open a Repository on GitHub show only while the repositories
    under ~/src, read once for both as the shell starts and as the launcher opens, hold one they
    can open, listing the one whose git last wrote first, a repository's files the one changed
    last first, each beside its folder there, its hidden ones but not what git ignores, and only
    those with a remote on GitHub, origin's first; Chrome's downloads, read as Chrome is
    stepped into, are file rows in its Recent, dated, their size and when beside them, and in
    each one's drawer where it is then its kind, a picture previewed, and where it came from, with
    Open the Latest Download listed and found from the top
    only while there's one, opening it as the desktop opens a file, and standing in its place
    saying an ellipsis until the first reading answers; and Neovim lists every file
    each Neovim has open once, the one looked at last first, said modified and by its tab, going
    to its buffer through open-now, each file's drawer saying where it is, when it was last in a
    window and its buffer, and counts them in its drawer, while its acts found from the top ask
    open-now nothing and make no open file's row
  '';
  the-launcher-answers-the-rain = ''
    the weather's parts and the rain by the hour come from one forecast, and "rain", "umbrella"
    or "when will it rain" answer when it next rains, here or in a place; here is the location, by
    its town or else its position, ahead of the place set and then the time zone, and the footer
    says which
  '';
  the-launcher-answers-what-is-asked = ''
    sums, bases and bits, addresses, subnets, characters, measures, zones, colours, the machine's
    figures and its parts' heat answer, and WARP says how its link stands from the record warp
    prints, connected, degraded, down, not answering or unreadable, and nothing with no warp-cli,
    every figure of the record a part in its own unit and in words.json's words, its settings
    saying who set each, its TLS and DNS a step in; what it read while shown sparks Round Trip,
    Loss and Traffic, each with a drawer over the tunnel, one read sparking nothing; and its
    switch says who holds it, when WARP always on comes back, and a switch held or refused
  '';
  the-sessions-kitty-outlives-a-switch = ''
    a switch never restarts the kitty every session's window is in, so no switch closes them
  '';
  the-sessions-kitty-comes-before-its-sessions = ''
    the kitty every session's window is drawn by sits in agents.slice above its sessions' weight,
    with a hard memory floor agents.slice keeps as large, so a session opening never starves the
    window Joel types in
  '';
  the-screens-go-off-on-niri = ''
    on niri, Turn Every Screen Off closes the launcher and then asks niri to power its monitors
    off, and :set noscreens asks it too, as read on the way to niri's socket; and a scale niri
    refuses says niri's words for why on the warning line
  '';
  the-newest-surface-takes-the-keys-on-niri = ''
    on niri, the launcher opened over held notices takes the keys and gives them back as it
    closes; held notices take them from the open launcher and give them back, and a notice
    arriving never takes them; and a click on a window gives that window the keys, leaving the
    launcher open without them
  '';
  an-agents-return-does-its-harmless-act-on-niri = ''
    on the account's whole shell on niri, its sessions the realm's mycelium node's, a listening
    agent's Return asks what to say to it and a paused one's lands on Resume, which a second
    Return sends, while Ctrl+] still steps into every act either has; a drawer names the repo
    its record says over the folder it works in; a paused one's glyph is never drawn in a
    listening one's colour; and an orchestration's waiting sessions fold into one row after its
    own, which Return steps into and typing still finds each in
  '';
  the-launcher-answers-whats-typed-on-niri = ''
    on the account's whole shell on niri, typed into, the launcher answers units, zones and
    colours and copies them, says once and quietly what needs a network couldn't, holds an
    answer's row from the keystroke, finds the power acts by their names and the names they had
    and asks before they act, draws nothing closed as a minute turns though it says the new time
    as it opens, draws nothing left open and still, its caret steady, makes no window's rows
    shut as windows open and close yet a pick knows them shut and it finds them as it opens,
    searches text a program
    types at once once for its newest letters, a Return among them acting on the row those
    letters found, steps into For You's looks by :filter look and back out by -, glides the
    selection to its row by the arrows only and draws it on its row as
    rows above it change, draws again no row a narrowing letter kept and, for a first letter, only
    the rows in view and one past them, reads on a long name while it's selected, tells the meaning daemon the
    parts its places hold and adds one meant below what the letters found, saying where it lives,
    and warns of nothing it drew;
    WARP, read by the real warp from a stand-in warp-cli, says how its link stands in Network
    after Speed, and its service not answering, and its :disconnect runs warp disconnect and
    says when WARP comes back, or, its switch held, leaves it and says who holds it;
    and the notices it holds, taken as Super+N takes them, turn, forget and step into their cards
    by vi's keys
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
    a file in the Nix store opened lately goes by its own name past its hash, and keeps the hash
    where the store lists it; a place kept elsewhere, as Google's Drive, is listed after home's
    files, and its files are fetched before they're answered; saving, the line holds the name,
    kept from folder to folder, each folder leading with it unnarrowed, a name already there
    asking once, no saving nothing; saving there, its folders take the name, ask once before
    replacing one, and answer the path
    the place stages it at once its folder is made, marked beside it with the program that asked;
    the files selected in one ask are previewed by one file-preview; where several may be opened,
    :arga gathers the cursor row's file or a range's, each once, Return answers what's gathered,
    saying how many, and :args steps into Gathered, its plate counting them; a folder's Copy the
    Path comes last; in every folder the field asks for one file, several, a folder or where to
    save, in the program's word; :sort lists it by name, change or size, :sort! turned round, made
    afresh, and a restart keeps the order; :set hidden lists the hidden files; :cd and :e go to a
    folder, their paths completed from folders read; :w saves, asking once over a name there; ]b
    and [b go to the folder beside it, round from the last to the first, once its holder is read;
    a file holds gx, opening it in its own program
  '';
  the-launcher-finds-emoji = ''
    emoji are found by name and meaning, best first
  '';
  the-launcher-finds-the-desk-light = ''
    the launcher says how the desk light is lit, its drawer its lumens, node and serial, sets its
    brightness, warmth and power through litra, and the bar hears changes but not a camera
  '';
  the-launcher-finds-the-keyboards-keys = ''
    a key is found by what it types, and shown where it is, and the board's Layer and Lights hold
    choices that run its own command
  '';
  the-launcher-finds-the-people = ''
    People with Tuple as its source, read from the people daemon, lists favourites and those called
    lately, finds anyone as they stand while holding only those listed and found, calls, joins and
    favours them, steps in before calling, lists a source's page for them after the ways to reach
    them with its facts in the drawer and the time where they are, knows who an address is with no
    row made, says at home how many it lists and names them with no row made, tells a pick who's
    listed with no row made while unwanted, and never asks Tuple itself
  '';
  the-launcher-finds-the-quests = ''
    the launcher reads the open and ended quests the node serves, each with its own kind rather
    than the node's, lists open ones driving first, finds finished ones and ids, opens pi on one
    in the folder the harness names or goes to its session, offers the machines the node places on,
    each saying how it stands as Machines does, and sees one made since, remaking only the quests
    a change names and none a list written again leaves the same
  '';
  the-launcher-finds-the-queue = ''
    the launcher's queue names each job in words, running first, says its time against its last
    run or what it waits for, its quest and memory, and goes to its session or copies its name;
    the sessions said anew make no row again, and the queue written again only the job it changed
  '';
  the-launcher-hears-talks-from-the-node = ''
    Agents hears each talk from the node by its session's address, newest first, moves it on as
    its session writes a line, hears one made since and lets one go, reading no talk's file
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
    listed and steps in rather than starting; the list programs share heard again unchanged
    changes nothing, an act another provider gives makes its own row and no program's, and a
    step asked again makes none of its Recent's rows again
  '';
  the-launcher-holds-still-as-it-refreshes = ''
    a list the launcher shows keeps its order and its selection as what's in it changes
  '';
  a-place-reads-alike-everywhere = ''
    a place's row says nothing after its name; at its right it says how it stands where it says,
    else its word for none while it holds nothing, else how many it holds, and nothing for a place
    that only groups others; and its drawer names what it holds, in its own order
  '';
  the-rows-hear-only-a-new-place = ''
    the rows drawn wear the last step's glyph and hue and say whether they're home's, and are told
    only what changed: nothing on a refresh in the place they stay in, each change once on a step
  '';
  recent-means-one-thing = ''
    Recent lists at most nine things, newest first across its sources, each once where it's
    newest, none already shown above it, an undated source after the dated in its own order, and
    its row says how many and holds their names for its drawer, none while empty and an ellipsis
    while waiting,
    and a source's lines are read with their times apart, the shared list's by each program
  '';
  for-you-narrows-by-kind-and-source = ''
    For You's :filter keeps the rows of a kind (do, look or try) and of a source, by its id in any
    case, both where two words are given or one is given inside a filter, refuses a word that's
    neither or a second of one with Vim's E475 and none at all with E471, names the source then
    the kind for the head, and Tab offers only the kinds and sources that would keep something,
    each with how many, its name as the head says it, and its kind's, source's or program's mark
  '';
  an-act-shows-before-its-daemon-answers = ''
    a row an act asked a daemon to change is drawn as it will be, keeping what the launcher made
    of it, until its source's row reads the same; one to go is left out until it's gone; a hold
    comes due once its time is up; and the foot says what wasn't done, then why
  '';
  the-launcher-paces-its-refreshes = ''
    an open launcher takes the first of its sources' changes at once and those after it a pace
    apart, together, and a refresh taken another way takes the one waiting with it, so no two
    refreshes begin closer than the pace; and a source's rows are wanted only while it's held,
    searched, named at home or has an act waiting, so a change nobody sees builds none; and a
    key's act is said shown by the first frame begun after its refresh, which another act
    refreshed while that frame is drawn never holds back; and an opening says what its own work
    went on slowest first, and weighs the foot's room for Return's verbs again only once the
    words file or the face has changed
  '';
  the-launcher-plays-from-spotify = ''
    Spotify's program in the launcher reads Spotify only as it's stepped into, once an opening,
    and holds its Recent, newest first, each row timed and noting its kind and size, and Liked
    Songs, Playlists, Albums, Artists and Podcasts, each counted, each played, opened and copied as
    it says, its note saying when it's signed out or out of reach
  '';
  the-launcher-lays-sessions-under-their-orchestrators = ''
    every list of pi sessions keeps one order of how they stand, and Agents lays each under the
    session orchestrating it, a step in and at most two, an orchestration standing where its most
    urgent row would; and a session's row is kept only while everything it shows or goes to
    reads the same, its kitty window, held id and attach words among them
  '';
  pi-tells-of-its-sessions-as-an-agent-source = ''
    pi, as an agent source, sorts the node's sessions into those running, parked and closed this
    boot, and says each as the agent source contract's record, leaving out what pi didn't say
  '';
  pi-sorts-without-collecting = ''
    pi's sessions, as many and as large as Joel's, each changing its step with the launcher shut
    and then showing, start a garbage collection in fewer than one change in five, sorted and
    lived again without text
  '';
  the-launcher-says-every-machine-of-the-realm = ''
    Machines says every machine of the realms this one links, this one first, each in words with how
    it stands, and its drawer from what moves most to the model's identifier, leaving out what no
    node said, says how an update it was asked for ended, and how Start a Session says each machine
  '';
  the-launcher-keeps-its-trials = ''
    each opening of the launcher is kept as its keys by kind, its pick and how long it took, and a
    miss only where letters were typed and nothing picked, and the file gains a close's line alone
  '';
  the-launcher-keeps-its-trials-one-by-one = ''
    a close adds its own opening's line to the file and rewrites none before it, however many are
    kept, and a start carries the old list into lines and lets the oldest go
  '';
  the-launcher-lands-where-it-did = ''
    a thing is one row, the same letters keep their row, one slip teaches nothing
  '';
  the-launcher-lists-inkscapes-drawings-lately = ''
    Inkscape's Recent merges each drawing Inkscape opened by when, to the microsecond, newest
    first, with the shared list's files, each once, a drawing that's gone left out, each saying
    when, its drawer its size as its SVG says it in its own units, its weight and its path, and
    Return opening it in Inkscape; New Drawing gives Inkscape a fresh copy of the template, and
    New Drawing From the Copied Picture is offered only while an SVG, or a picture whose size
    the history keeps, is copied, and makes a drawing of it; and Inkscape's drawer in Programs
    says its last drawing and how many
  '';
  the-launcher-makes-texts-ready-lightly = ''
    making a text ready to match and keeping it grows the launcher's memory by at most 3.5 KB a
    text, where one whose letters were spread into calls took 7.6
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
  the-launcher-names-every-action-a-command = ''
    every launcher verb names a command the key language has, with :set's whole word as its
    argument and % only on an act taking rows, but for one Return alone does, which names none,
    and no two verbs on one row make the same line, a row that's an act itself, as Lock, held
    the same where it names one; no :set toggles, as wifi! does, and a switch names each
    state's own, as :set nowifi and wifi; each one that doesn't is printed, and the check
    refuses a stand-in vocabulary breaking each of its rules
  '';
  the-launcher-names-the-parts-it-builds = ''
    every part the calculator, the time, today, the waits, a hardware address, a network and a
    character show is named as the vocabulary names it
  '';
  the-launcher-plays-on-where-mpv-stopped = ''
    mpv's Recent merges each file mpv left by when it left it, newest first, with the files the
    shared list gives, each once and none open now, read once mpv is stepped into, each opening;
    one stopped part way has "12:04 of 41:10 · 12 min ago" at its side, Return resuming and From
    the Start inside, one watched whole says its length and when and play, and each drawer says
    where it is and its kind, then what's still to play, what mpv kept, its size and its path;
    and Play the Copied Link is offered only while the clipboard's one web address is one mpv
    plays, a video site's page or a media file
  '';
  the-launcher-reads-on-where-the-reader-left = ''
    the reader's Recent merges each file the reader kept a place in by when it was left, newest
    first, with the files the shared list gives, each once and none open now, read once the reader
    is stepped into, each opening; one left part way has "Installing · 40% · 12 min ago" at its
    side, Return reading on and each mark inside reading from its letter, one read to its end says
    so, each drawer says where it is and its kind, the heading, how far down, each mark and its
    path, and the reader's drawer says how many files are still to finish
  '';
  the-launcher-remakes-only-the-screens-that-changed = ''
    the launcher's Screens, asking niri again at every workspace change and opening, makes no
    screen's row again while niri's answer reads the same, and draws a screen that changed as it
    now stands
  '';
  the-launcher-remakes-only-the-windows-that-changed = ''
    the launcher's windows list This Window, then the rest the one used last first, and when the
    focus moves make again only the rows of the windows it moved between, a renamed one too, as
    Capture's rows of the windows do
  '';
  the-launcher-says-the-page-zathura-is-on = ''
    zathura's provider reads open-now's lines as each zathura's document, page and page count,
    says the page beside a document only one zathura has open, and its drawer says each one's
    page and how far through it is
  '';
  the-launcher-says-what-niri-refuses = ''
    a screen's change niri refuses says what was asked, what wasn't done and niri's words for why,
    begun in lower case with no closing full stop, a $ in them kept as written, and one to a
    screen gone says it's unplugged; the last lit screen not turned off says so the same way
  '';
  the-launcher-says-why-the-desktop-lags = ''
    the launcher's Lag leads with the worst of what waited, offers to see what's behind it from
    the top, says how fast the desktop answers in plain words, and says nothing's slowing the
    desktop when nothing is
  '';
  the-launcher-shows-a-terminal-what-it-holds = ''
    a step of `launcher show` names its row by its name in any case, a hyphen for a space, or the
    one name it starts, a miss offers the nearest names, and a row and its drawer reach the
    terminal as the words the launcher draws, a getter's and an empty fact left out
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
    only they are there; the location and the clock's zone are whereabouts' own, over its socket;
    and For You, asked again, makes none of its rows again, its looks, its to-dos, the sessions
    wanting Joel and To Look At's own, only one whose file or what it says changed, a session
    wanting Joel making only the look it left again and to-dos said anew as they were none; and a
    piece of what's on that moved is the only one whose row and acts are made again, a kept one
    running its piece's act as it is now; and For You's Desktop holds the notices unread as the
    notices' own rows, and its History those read, stepping in again after one is read making
    only that one's row again
  '';
  the-launcher-sums-within-a-key = ''
    the launcher's sums and measures, worked out by libqalculate in the shell's own process, take a
    typical key under five milliseconds
  '';
  the-launcher-turns-each-screen-off = ''
    where niri powers one output off by name, Turn It Off darkens a screen and Unplug It follows
    while another is lit, the last lit one darkening only after the key that asked comes up, a
    dark screen turning on by name; an older niri only unplugs, keeping the last screen on
  '';
  the-launcher-traces-each-opening = ''
    each opening of the launcher is a trace whose steps, backs, acts and searches are spans under
    it, a void above a list's first row is told from a list scrolled down, and the whole is sent
    in OTLP's JSON mapping
  '';
  the-launchers-graphs-draw-in-braille = ''
    a row's graph is ten braille characters over a dim track, a bar from the left, a spark from the
    right and a strip a reading a member, warning at 95%; graphs line up in one column, and a
    figure wears no glyph
  '';
  the-launchers-rows-keep-their-words-apart = ''
    a launcher row and field keep their words apart and inside them, however long, ending what
    doesn't fit in an ellipsis, and the launcher's note, nothing typed, ends where a row's value
    ends and says all it has room for
  '';
  the-lid-shuts-when-the-hinge-agrees = ''
    lid-hinge believes a shut switch once the hinge last read nearly shut, a light sensor lit at
    the switch reads dark, or none was lit, doubts it otherwise, and reads the hinge's buffer
  '';
  the-litra-follows-the-cameras = ''
    `litra` finds the lights by their ids, speaks their reports, says each one's lumens, name and
    serial, and counts a camera streaming only by a whole file naming a program
  '';
  the-mac-guards-its-disk = ''
    a Mac's guards run under launchd, with a Mac's paths and nothing to confine a run
  '';
  the-macs-faces-are-the-themes = ''
    the graphical role's fontconfig, asked for a Mac's system or monospaced font by a name pages give
    it, gives the theme's sans or monospaced face, carrying the name asked for
  '';
  the-machine-answers-from-its-daemon = ''
    Machine answers the processors, the memory, the disk and the uptime from the machine daemon
    reading the sandbox's own /proc, their headlines making no parts, and asks it for the programs
    using the machine only once a part saying them is looked at
  '';
  the-margin-takes-its-pieces = ''
    a machine moves, cuts, retunes and adds the margin's pieces
  '';
  the-margins-marks-stand-level = ''
    every mark in the margin stands one height on the gap's middle line, read from the right by
    kind, in the colour that says how much it asks; what was on air lingers, what a notice will say
    waits for its card, and a wait shows once the same one lasts; each stands in a square cell the
    gap's size with the same room all round, and Now points at the row the keys are on; For You's
    tray is lit by what For You counts as waiting on Joel, a session's want but an action, a
    to-do he can do now, a look, a source's to-do or a critical notice unread off screen, and by
    nothing else, one count for both
  '';
  the-margins-marks-are-drawn-level = ''
    every mark the margin can show, a device's kind and Slack's among them, is the theme's
    drawing of it, standing the grid's full height and centred across it as Qt draws it
  '';
  the-mic-mute-light-follows-the-mute = ''
    `mic-mute-light` asks after the mute only once a change to a mute or the default source has
    ended, or after any change while the light has yet to be set, and lights the light for a
    muted source alone
  '';
  the-music-follows-the-player = ''
    the music is the player that started last, never playerctld's echo
  '';
  the-notices-are-a-quickfix-list = ''
    the held notices are Vim's quickfix list: the newest first, each motion's landing, what an
    operator takes, the view kept around the cursor, the gutter's numbers, and u and Ctrl+R
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
  the-notices-take-vi-keys = ''
    the held notices read Vim's keys in each mode from one table: motions, operators and objects
    with counts, visual's operator taken with a second press, fresh presses only for what acts, the
    find and command lines, a reply's editing, Space ?'s list and the foot's hints
  '';
  the-notices-wait-while-joel-is-away = ''
    a notice that comes while nothing has been typed or moved stays on niri past the time its sender
    gave it, never counting down
  '';
  the-npu-finds-its-compiler = ''
    the XPS 14's graphics drivers hold the NPU's driver and Intel's compiler beside it
  '';
  the-panel-dims-after-a-bad-read = ''
    the XPS 14's kernel keeps to VESA's brightness controls when its boot-time read of the panel's
    eDP revision comes back wrong
  '';
  the-panel-holds-still = ''
    the XPS 14's kernel leaves the panel's self refresh and frame buffer compression off, since
    PSR2, PSR1 and compression each set the screen flashing after a replug or a lid-shut boot
  '';
  the-panel-is-asked-until-it-agrees = ''
    the XPS 14's kernel reads the panel's eDP display control until two reads agree
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
  the-passwords-place-copies-through-its-daemon-on-niri = ''
    on a nested niri, the Passwords place asks the passwords daemon nothing until Joel steps in,
    then lists once as his, leads with the locked manager's row, whose Return unlocks it, one
    refused saying why the next time the launcher opens, copies
    an entry's password on Return through the daemon to the clipboard as the launcher goes, the
    margin wearing the clipboard's mark and the foot, opened again, counting to its clearing,
    nothing on screen saying it, reads an entry in full as the selection rests on it, never as
    Joel's own ask, its drawer never saying its password, and holds its acts in the design's order
    once it's stepped into, Open the Website handing a stand-in Chrome its website as the
    launcher goes, its foot saying 1Password offers to fill it, with a stand-in for 1Password's
    manager
  '';
  the-passwords-show-every-figure-but-the-secret = ''
    each kind of entry wears its own mark, the favourites are listed first, then what changed
    lately, newest first, twenty at most, a row says its username at its side and names its
    password manager only while two are on the machine, an entry's acts come in the design's
    order, each only where it has what it needs, opening it in the browser its manager fills in
    only on an http or https website, its manager's link is filled only where the entry
    says every part, its drawer's eyebrow says its vault and kind, and its facts every figure but
    its username and its password, the design's first in its order, a hidden detail as hidden and
    never its value, a website sent in the clear as its health and a weak password as its strength,
    each in the warning's role, its password reused or breached as its health once every password
    is checked, its figures grouped, and nothing before, those needing attention by title, the
    foot counts a copy down to its clearing, or a one-time code to its lapsing, a manager's drawer
    in Programs says how many entries it holds in how many vaults and how many are favourites,
    each left out at none, and For You's one to-do for what the last check found stands while
    some login is breached or reused, is done once a later check finds none, says how many are
    breached then reused at its side, each left out at none, and is weighed as a thing to do
    ready now
  '';
  the-people-are-one-person-each = ''
    every source's people join into one person per email address, a source that knows two
    addresses for someone joining them, each standing as the first source that says, favourites
    listed first, then whoever there's been something with, the last first, someone only asked to
    what's to come left to be found by typing, the order reading how each stands once each and
    changing with the clock alone only as something with someone starts or ends, with their
    details for the drawer and their acts in the one order
  '';
  the-picker-picks-what-the-keys-say = ''
    the region picker starts on the middle quarter, moves and sizes its box by arrows within the
    screen, and names a 16 by 10 grid's cells by two letters
  '';
  the-pinentry-asks-the-shell = ''
    linux-id's pinentry asks the askpass in the program's words
  '';
  linux-id-passes-its-own-tests = ''
    linux-id builds with core's patches and passes its tests, among them
    Chrome's touch probe refused without asking, and an approval kept only
    for the site and operation it was given for
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
  the-query-field-takes-vi-modes = ''
    the launcher's field reads its keys from one table in two modes: insert's caret moves,
    Ctrl+W, Ctrl+U and Ctrl+H, the rows' keys and Ctrl+Y, Escape into normal, and normal's i a I
    A, cc and S into insert, normal's ]b and [b to the place beside and gx, on a prompt's words
    h l w b e 0 $ x dw cw and u editing them as Vim's do and - stepping out with them, a question
    asked once answered by y alone and let go by n, Escape or Ctrl+C, but not in its first moment,
    with an Escape that cancels a c or a count begun before it steps back, normal's moves through
    the rows as Vim's, with counts: j k, gg G, H M L and Ctrl+D U F B, its - out, its q, ZZ and
    Space w closing, its : opening the command line with the range a count before it gives, Tab
    to the place used last, and Space ? and F1 listing every key, while / ? n N search nothing on
    the rows; and Vim's d and y over the rows, dd 3dd d3j dgg dG yy y{motion}, each row's ending
    action and copy, the (y/n) ask once before what can't be put back, and yG or dG over 400 rows
    within half a key's 4 ms
  '';
  the-queue-is-heard-from-the-node = ''
    the machine queue is the harness's file as the node's record holds it, none of a version it
    doesn't read, and a write of only when it was written says nothing new
  '';
  the-sandboxes-door-trusts-no-one = ''
    the Nix daemon's door for sandboxes, run as its socket unit runs it, answers as a daemon and
    takes its caller as untrusted, so a sandbox passed it rather than the daemon's socket can't
    make itself root
  '';
  the-screen-keeps-its-level-as-it-changes = ''
    the screen's saved brightness is kept as it changes while the lid is open, through shutdown's
    saver, so a shutdown with the lid shut loses nothing set since
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
  the-scripts-are-the-script-faces = ''
    a letter of Hebrew, Arabic or CJK asked for in any of the theme's faces is drawn from the
    script face's family for that script, not DejaVu Sans, and each cut named for one script beside
    the serif or mono face is installed and has that script's letter
  '';
  the-serif-is-its-variable-files = ''
    asked for the theme's serif face, the machine's fontconfig lists its two variable files and no
    static cut, and gives the italic one for the italic
  '';
  the-serif-text-is-the-upright-at-large = ''
    the serif upright's cut answers to its own name and draws as Fraunces does at the axes the theme
    gives it, and not as the variable font's default
  '';
  the-session-goes-idle-as-the-screens-go-off = ''
    swayidle tells logind the session is idle at the same moment it turns the screens off
  '';
  someone-at-the-machine-is-marked = ''
    swayidle marks someone at the machine from the session's start and from any key or touch, and
    takes the mark away after two minutes without one, for a followed switch to wait on
  '';
  stepping-away-asks-logind-nothing = ''
    the lock and the screens going off as Joel steps away wait on no hold of logind's idle lock,
    only on Keep Awake's and a call's, so a program holding the machine awake keeps no lock off
  '';
  the-desktop-follows-dictation = ''
    the shell follows each dictation state, watches again a service its socket restarts, and tries a
    socket nothing listens on once, never looping
  '';
  the-desktop-follows-tuple-calls = ''
    the shell says who a Tuple call is with and how long it's gone on, finds a call's link in what
    was copied, and lists Tuple's Recent newest first, each call by who, how long or missed, and
    when
  '';
  the-desktop-finds-screenshots-by-their-words = ''
    what was taken lately finds screenshots by their words from the screen-text daemon, on one
    connection made only at the first search and let go at rest, each text asked once and the last
    four answers kept, each screenshot found with the line that matched; and finds a few thousand
    files by name, newest first, each key within a frame
  '';
  the-desktop-hears-tuples-call = ''
    the shell's Tuple holds the call through the real tuple-call: who's in it, since when, mutes,
    screens and cameras, Joel's room, comings and goings told, a ring and a missed call from
    Tuple's log, and muting and opening the room asked of the daemon
  '';
  the-desktop-holds-a-daemons-records = ''
    the shell holds a daemon's records over one connection it keeps: a daemon that's down is said and
    never waited on, one that starts is connected to by itself, a watch narrowed to those listed
    holds them alone, a search's answer is never held, keys watched ahead and a get's answer land by
    key, what was held stays through a gap unless the client lets it go there, a client of two
    kinds names both in every watch, a kind with none is known to hold none once answered, and a
    whole answer after a gap lets go what went in it
  '';
  the-desktop-reads-the-spend-watch = ''
    the AI spend piece and collection draw what the watch wrote, put what wants a look first, step
    into each figure's parts, hide amounts while sharing, and answer reports by their words; a
    budget's row, a window's and a cap's open a drawer of every fact budgets.json holds of it,
    and a period's its days and what its cuts went to most; the tokens piece is there only while the watch's tokens warn, with its acts for a running session
  '';
  the-desktop-speaks-its-javascript = ''
    the shell calls no method Quickshell's JavaScript lacks
  '';
  the-shell-starts-nothing = ''
    the account's zsh starts only git-engine, its fifo and direnv, and each prompt after runs direnv
    alone
  '';
  the-desktop-reads-the-documents = ''
    the shell reads the documents from the google program's state, none from one saying none,
    knows which Google program opens each kind, and says only when Joel last opened a document
    beside its title, with its details for the drawer; reads the Drive folders
    listed and the downloads, finding each folder asked together, writes the lines that list a
    folder and download a file, a save alone saying where, and says how the newest download asked
    stands; and says a Drive file in the picker by what a folder's read would, its size and when
    it changed, and its owner, or what Google's own kind comes as, picked by that name and kind,
    adopting its document row, with why a file or a folder couldn't be had; and reads the saves into Drive, writes the line that sends one again,
    tells each settled since the shell started once, oldest first, by its folder's name, sent as an
    answer with Open in Drive and failed with why and what mends it, a copy that couldn't be read
    going back to the program that saved it while it's open, and finds a failure's notice by its
    file alone; and reads Drive's own places, Drive's program opening every kind and Drive opening
    any file no other program does, lists each place holding anything with its count or over it,
    and says a Drive file's kind with a picture's size and a video's length and the drive it's
    in as its drawer's eyebrow, and the figures the program read of it in its drawer, what
    changes most first and its id last; and reads what Drive's copies kept here take, a figure
    not a number as none, saying it of the most kept only while there's any
  '';
  the-desktop-reads-the-mail = ''
    the shell gathers the google program's whole state from its daemon's records, each kind in
    its order, a signed-out account alone, and no account once the daemon stops answering; says
    when mail came in words, and a moment as a drawer says one, and what its row and its details
    say, who was copied and the files it carries among them, each file by its kind and size;
    writes a reply to who sent it, and to all, copied to everyone else it went to, and a mail to
    everyone else a meeting names; writes the lines that mark mail read or unread again or archive
    it or fetch a file it carries; says how that fetch stands; writes the line making a Meet
    meeting and says how one asked for stands; and lists as Recent the meetings held lately and
    the mail Joel had, newest first, each once, none said no to and none still wanting him
  '';
  the-desktop-reads-the-transcripts = ''
    the shell says what each call written down was, when it was in words and how long it went on,
    or that it's being written down; sets its summary or its transcript's opening from markdown as
    styled text, nothing it said read as markup; says who it was with; and lists the calls newest
    first, those with someone by any of their addresses, and a meeting's own
  '';
  the-desktop-tells-the-next-meeting = ''
    the shell names as next the meeting about to start or going on, never one over, all day or said
    no to, says when it is in words, when it's about to start and when it's said as a notice,
    whether Return joins it or shows it, whether it's today's and how Joel answered it, gives its
    details for the drawer, how it repeats in words and how many of those asked answered each way
    among them, which may be moved and to what later or typed time, what that clashes with and who's
    told, writes the lines that answer, cancel and move it, says when one over was held, knows
    which Meet call a window is and whose meeting it is, and lists as Meet's Recent the calls held
    on Meet, once by code and newest first, each saying who it was with and when, and counts the
    people known on a meeting, each once, making no one's row
  '';
  the-shutdown-mark-cannot-stop-a-reboot = ''
    the mark a healthy stop leaves can't fail the reboot, and what reads it outlives the last stage
  '';
  the-shutdown-names-the-running-generation = ''
    a shutdown's splash names the newest generation that is the running system, a specialisation's
    included, and none for a system never made one
  '';
  the-shutdown-splash-finds-its-serif = ''
    the serif the splash sets its words in is among the system's fonts, where a shutdown's splash
    looks for it
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
    niri holds its first frame for the wallpaper, only with one to wait for, asked by the name its
    patch reads, for less time than Plymouth takes to quit, so what the splash left stays on the
    screen until the desktop draws
  '';
  the-splash-prompt-is-the-locks = ''
    the splash's panel is the lock's field, a prompt's row inside its hairline, at each scale, its
    padlock's cell the inset rung in from the frame, what's typed as far after it as Qt puts it,
    between the glyph's advance and its ink with Prompt.qml's room after, and every face it
    places by its capitals measured at each scale
  '';
  the-splash-reads-the-generation-from-the-entry = ''
    the boot's splash is told the generation only from an entry whose name carries one
  '';
  the-splash-stays-until-niri-draws = ''
    the splash lets go of the screen as greetd starts, keeping the GPU open, and quits ten seconds
    after greetd starts rather than with the boot, so the console never shows between it and niri;
    a machine without the splash keeps greetd waiting for Plymouth to quit
  '';
  the-stick-types-colemak-on-every-built-in-keyboard = ''
    the install stick remaps the built-in keyboard of each of the four models core describes, its
    Apple ones included, so a passphrase typed there is the one the owner meant
  '';
  the-sun-sets-as-noaa-says = ''
    the sun rises and sets within a minute of NOAA's tables, in Greenwich and far north, and where
    it neither rises nor sets on a day it says whether it stays up or down, and its apparent
    longitude, the calendars' solar terms, is Meeus's worked example's to a thousandth of a degree
  '';
  the-surface-boots-with-its-gpu-and-a-clock-that-reads = ''
    the Surface Laptop 4 loads its GPU's driver and the CMOS clock in the initrd, and keeps
    the firmware's unreadable clock from taking rtc0
  '';
  the-surface-keeps-the-clock-its-firmware-starts-from = ''
    the Surface Laptop 4 reads its controller's clock for the firmware's offset before chrony
    starts, through the device the controller's module gives, and sets that clock at shutdown and
    every hour
  '';
  the-surface-keyboard-types-at-the-disk-prompt = ''
    the Surface Laptop 4's initrd has every driver on its keyboard's path, and keyd lays that
    keyboard out there
  '';
  the-surface-loads-its-touchscreen-once-up = ''
    the Surface Laptop 4 installs its touchscreen's driver with the panel's qualified settings, and
    loads it from a unit after multi-user.target rather than while the system starts, with no step
    after the load to fail a switch, and gives its axes the panel's size in millimetres
  '';
  the-surface-offers-its-firmware-through-fwupd = ''
    fwupd, given the Surface Laptop 4's ESRT with its UEFI at 4.355.140 and its TPM at 7.2.2.0,
    offers the UEFI's capsule from the remote the module writes, at the version the capsule itself
    carries, as a trusted upgrade, and nothing for the TPM
  '';
  the-surface-reserves-memory-to-hibernate = ''
    the Surface Laptop 4 writes a reserve of 1 GiB to /sys/power/reserved_size at every start, so
    a hibernation leaves room for what amdgpu copies out as it stops
  '';
  the-surface-watchdog-wakes-again = ''
    the Surface Laptop 4 arms systemd's runtime watchdog at 30 s, and its sleep hook, run with an
    empty PATH against a stand-in of busctl, stops that watchdog before a sleep and starts it again
    after at the same timeout, touching nothing when it isn't running
  '';
  the-viewfinder-keeps-its-own-keys = ''
    ZZ closes the camera's viewfinder, and Escape and q leave it open, even where a script in
    mpv's own folder binds Z, as the player's vi keys do, since the viewfinder loads none of them
  '';
  the-web-kit-carries-the-theme = ''
    the web kit's CSS holds every role of the machine's theme as its dark look and of core's light
    theme as its light one, and a first view's three faces come in under 350 KB
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
    clippy finds nothing in timekeeping, which reads chronyd's command port for Time, or its
    tests
  '';
  timers-are-kept-and-rung = ''
    the timers daemon tells every watch of a timer started and rings it as a critical notice once
    its end passes; the notice's Again sets it going afresh and putting it away ends it, ending one
    rung takes its notice back, a ring with no server to take it is said once one comes, what's
    kept outlasts the daemon, one that ended meanwhile ringing late, an ask it can't do is
    refused with a reason the launcher says in its own words, and the stopwatch runs, laps,
    pauses and resets, outlasts the daemon, and is heard only by watches of its kind. A timer
    coming due within a minute of its end plays its sound once, not again when its notice is said
    afresh, and quietly while a call is on; one found long past its end is said late, unheard
  '';
  timers-passes-clippy = ''
    clippy finds nothing in the timers daemon or its tests
  '';
  to-do-clears-each-to-do-by-itself = ''
    the to-do daemon finds a to-do done or ready by its check or its signal, a boot or a switch
    only after it was raised and a switch to a commit at it or past it, keeps a failing check to
    a run each ten minutes at most, stops a check that hangs with all it started and leaves
    nothing a check started once it ends, runs a check only for an owner the harness says is
    outside every sandbox and with the session's environment as it is then, keeps a to-do done
    through a failing check and a restart, says every figure it found, writes its verdicts only
    as they change, and over its socket tells a watch each to-do with its verdict and Joel's word,
    in the list For You puts it in, each look the agents left, and their counts, as they change
  '';
  to-do-passes-clippy = ''
    clippy finds nothing in the to-do daemon or its tests
  '';
  to-do-runs-in-each-account = ''
    the to-do daemon runs in each account with its own git, journalctl and systemctl named, its
    checks on the user manager's environment as it is when each runs, its socket systemd's and
    the harness's looks and Joel's word read beside it, and a switch restarts it
  '';
  tuple-call-holds-the-call = ''
    tuple-call follows Tuple's stream only while Tuple's daemon answers, at once when its log says
    it started, and tells a watch of a call, a sign-in gone and a stream ended as they happen, of a
    ring and a missed call from the log, a new log read from its start, keeps a call that ended in
    Recent's file and reads it back after a restart, runs each of the call's acts as Tuple's
    command, keeps the room's link, answers on systemd's socket, and refuses an ask or a kind it
    doesn't know
  '';
  tuple-call-passes-clippy = ''
    clippy finds nothing in tuple-call or its tests
  '';
  tuple-call-runs-on-the-daemons-socket = ''
    tuple-call runs on the daemons' socket, which systemd holds for it, with the tuple command the
    module installs, restarting as the new one after a switch
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
  tuple-people-follows-the-contacts = ''
    tuple-people's own tests pass: it reads Tuple's contacts as JSON, says only who changed, and waits
    twice as long each time Tuple's follow ends at once, up to a minute, and five seconds again after
    one that ran a minute; what the follow says as it ends is logged once while it says the same, a
    signed-out Tuple's in plain words
  '';
  tuple-shares-through-the-portal = ''
    Tuple reads the portal setting where the switch writes it
  '';
  tuple-takes-no-real-time-priority = ''
    Tuple's daemon may take no real-time priority, and goes first by a nice below 0 and a CPU weight
    above 100
  '';
  type-is-set-by-level = ''
    no QML file of the shell's sets a font's size, weight, axes, letter spacing or case other than
    from a level of the theme's type; nor does the reader's page or its foot say a size, a
    weight, a tracking or an axis of their own in the Typst they set
  '';
  update-to-main-outlives-its-switch = ''
    neither NixOS nor nix-darwin restarts update-to-main while it switches them: the service never
    restarts on a switch, the Mac's daemon names no store path, both start it from the running
    system, the Mac's waiting until that system holds it, and its askers are its group's members
  '';
  update-to-main-passes-clippy = ''
    clippy finds nothing in update-to-main or its tests
  '';
  update-to-main-updates-to-signed-main = ''
    update-to-main builds and switches to a main signed by a key it was given and descended from what
    it ran, builds nothing when main hasn't moved, neither fetches nor builds a hosted main whose
    folder hasn't moved, refuses an unsigned, untrusted or rewritten main
    and a low battery, refuses to start or switch while the machine is busy or can't tell, leaves
    the machine as it was when the build fails, goes back when the switch
    fails or the machine isn't whole after it, leaves a new kernel for the next boot, joins asks to
    the update that runs, refuses any other ask, serves on a socket only its group writes, and keeps
    how each update ended in a file only its group reads; and a hosted main, unsigned, is built
    from its flake's folder with root reading the account's checkout, fetched there first when it
    lacks main, its private inputs fetched by their locks, refused when rewritten, and builds
    nothing without a token; and a followed input is built at its own signed main past the lock,
    builds again when only it moved, and is refused unsigned or behind its lock; and building
    elsewhere sends only the heavy pieces away, to the builders named when some are, builds nothing
    here without a builder, sends a kept piece only to its own builders, and none at all when it
    has none, unless pieces may be built here, small, which keeps a piece's builders but a kept
    one's none and waits under load, and builds what's built from a kept one here with no builders;
    and a big piece goes to the builders with what needs nothing of it beside it, what's built from
    it after, and what was beside it stopped when it fails
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
    an account in wheel switches to another system the machine has run and back with no password, and
    to one it never ran only by authenticating; one outside wheel, and what isn't a NixOS system in
    the store, is refused; and with a YubiKey named but not there, polkit takes the password alone
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
    app.slice holds a hard 10% of memory and session.slice a hard 2.5 GiB, agents.slice a soft 15%
    and no hard floor, every slice above them a hard 15% and a soft 15%, and jobs is slowed past
    half; timed work sits at 10000 inside idle jobs, and work in jobs can move
    itself there; a scope work in jobs starts goes back to the scope of the work that started it,
    and one with a memory ceiling stays; each command written into jobs.service gets a scope of its
    own in jobs-work.slice, at weight 100, with its own swap, and the holder stays; one naming a
    quest pi's sessions weigh goes in that quest's slice under jobs-work.slice at its weight for
    CPU and disk, which follows the file as it changes, and one naming none or an unlisted quest
    stays; two quests' work on one CPU shares it four to one by weight, and one alone takes it all
  '';
  vm-a-call-is-on-reads-every-account = ''
    root's a-call-is-on says a call is on while a call program records in an account's PipeWire, and
    none before it starts or once it ends
  '';
  vm-a-call-is-written-down = ''
    a call program recording in Joel's PipeWire is heard by call-transcripts from its microphone
    and its own playback, and kept as one record under the meeting on, asked of Google's daemon,
    which ends with the call; its transcript holds both sides' three turns in order and nothing
    another program played; then a stand-in for the summarizer's model is started, holding a
    delay on sleep, and ended with nothing written when another call starts, and started again
    once it ends; its summary quotes Joel's line word for word with who it was said to and when,
    in summary.md and its record, the model's process and the delay are let go after, and a
    notice from Calls says the summary is ready, whose Read It opens it in the session's reader;
    a notice raised as the call started offers the line for its chat, and the next meeting,
    stopped from that notice's Stop Transcribing, lets its recorders go with the call still on
    and isn't recorded once rejoined; a call holds logind's lock against going idle while it's
    on, transcribed or not, and lets go once it ends
  '';
  vm-a-hidden-queue-is-taken-back = ''
    behind a router sharing 8 Mbit/s flow by flow under fq_codel, which delays no probe, the
    one-way controller takes the queue back under it, and an app's one upload gets over half the
    connection beside 32 of the jobs'; without the shaper it gets under a fifth
  '';
  vm-a-microphone-is-noise-cancelled = ''
    a microphone in Joel's PipeWire is offered a second time as "its name, Noise Cancelled",
    heard only while something records from that one and with no model loaded before; recorded
    beside the microphone itself, its noise comes out at least 10 dB quieter and its voice within
    10 dB, and once the recording stops the program says it rests and stops hearing it
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
  vm-boot-says-it-wakes-from-hibernation = ''
    the graphical role, hibernated to a swap file on btrfs inside LUKS and started from its
    image with the passphrase typed at the theme's ask, shows the theme's words for a restore
    on the first, middle and last shots taken while the kernel reads the image; on each
    of two screens of different sizes, shows the theme's empty track once every program is
    frozen, then core's kernel filling it as it reads, its bar exactly on the track, partly
    filled in some shot, never going back and full once the image is read; comes back
    with what it held in memory, and its hook says how long the restore took after the disk
    opened
  '';
  vm-chrome-says-its-open-tabs = ''
    Chromium, standing in for Chrome, installs the open-tabs extension from the local file the
    Chrome module names, unasked, its tabs and nativeMessaging permissions granted and nothing
    disabling it, and the account's Chrome source, finding the host through the file the
    module writes, lists two pages it opened by the host's ids, brings one to the front and
    closes the other through it; and with no extension asked, chrome-recent-sites reads two pages it
    opened from the session file it writes, each with its address, in one window with one in
    front, looked at lately
  '';
  vm-cargo-builds-on-their-own-filesystem = ''
    on a Btrfs disk, cargo's image is made without copy-on-write at the size asked, attached with
    direct IO and mounted ext4 with a folder each account alone reads; what's built in it is kept
    across a detach and attach, a reload leaves a build working in it, and an image past
    repairing is made again
  '';
  vm-checks-go-first = ''
    the daemon sits in a cgroup beside its builds, each build in one of its own handed cpu, io,
    memory and pids; a build's qemu goes from idle to normal at nice -10, every thread and a later
    one, and its build's cgroup is weighed at 1000 for CPU and disk, while a compiler's build beside
    it stays idle at 100; a build running a stand-in systemd-nspawn, with no qemu, is weighed at
    1000 and a service in it goes to normal at nice -10; a build asked for from a quest's slice
    takes the slice's weight and follows it; the daemon's own cgroup carries oomd's mark to pass it
    over; ten restarts of the daemon, vm-first looking every 20 ms, each start and leave the builds
    running, the daemon builds again, a build the daemon is restarted under finishes and its
    client is given the valid path it built, a build asking for a range of user ids gets 65,536, a
    uid-range build that makes a cgroup of its own is refused handing it pids with "Permission
    denied" and the daemon restarts beside it, a finished build that wrote 64 MiB is said with at
    least that and its client's command, one that wrote 64 MiB without waiting for the disk says at
    least that between what it wrote and what it left, a build's sync -f is answered as done while
    its sync runs and root's sync -f outside the daemon works, a build carries a signature by
    machine-1 that its public half alone verifies, the secret half root's at mode 600, a stopped
    quest's build is said with its quest, and a stopped build leaves no process nor cgroup
  '';
  vm-clock-keeps-time = ''
    chrony is told a network's own time server, still listening after a burst of link states, and
    sets the clock from the machine's servers past it when it's silent
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
  vm-desktop-captures = ''
    the desktop takes a screen, a window, a region, a recording and the camera's pictures and
    videos by the keys, and says how long a recording has run
  '';
  vm-desktop-keeps-copies = ''
    the desktop keeps what's copied, text and pictures, past the programs that copied it, Super+V
    gives it back into the window it opened over, and the launcher forgets it all when asked
  '';
  vm-desktop-keeps-pace = ''
    the launcher draws its keys, arrowed or typed, nearly as soon while every core is busy in
    jobs.slice and build.slice as with that load frozen: half the keys, less the VM's software
    swap and each one's wait for its frame to begin, and nineteen in twenty of the middle turn,
    within twice their frozen times and design.md's budget more, timed in
    turns in the same machine so the builder's own load slows both alike; and a step into
    Programs, Chrome's, the reader's, Inkscape's and Neovim's pages, Chrome's Recent and
    Passwords, each holding what Joel's do, shows within twice its frozen time and 50 ms more;
    and the first opening after the shell starts again, People listing 500 as Joel's lists
    hundreds, reaches its frame within twice its frozen time and 50 ms more, and the opening
    after it weighs none of the words file's verbs again
  '';
  vm-desktop-launcher-answers = ''
    the launcher answers from the machine and copies the answer: a MAC's vendor, the battery and
    its charger, heat, the screen's and keyboard's lights, which it sets, the keyboards, the disks
    and what fills them, the network, the queue, the processors and memory, the firmware, and the
    weather and time from where whereabouts says the laptop is
  '';
  vm-desktop-launcher-finds-by-name = ''
    the launcher lists each thing once for everything Joel types, finds a thing by every name it
    had, a collection by its name and emoji in their own place, steps in by a category's word and
    a row's hidden words, a command finishes by Tab, and its field's keys take what's typed away
    and put it back
  '';
  vm-desktop-launcher-draws-still = ''
    the launcher draws only what changes: a list scrolled by hand stays put as it refreshes, a
    burst of changes is taken together, no letter typed after its chord is lost, the pointer never
    moves the selection, and nothing moves unless it changes
  '';
  vm-desktop-launcher-finds-sessions = ''
    the launcher finds pi's sessions and quests and goes to their windows, talks to an orchestrator
    through its inbox, and reopens the sessions a boot took away, each where it sat, in the agents'
    slice
  '';
  vm-desktop-launcher-goes-places = ''
    the launcher's home lists what to browse, its catalogue opens places and does actions, a detail
    inside a thing is found from the root, programs' acts are found by name, every row has a
    Return act, the volume is set to a number, and an opening is traced to the collector
  '';
  vm-desktop-launcher-holds-meetings = ''
    a meeting about to start is found by its name and joined, Now lists today's and counts one
    within the hour down, said at its reminder and as it starts; one further off shows who's asked,
    is answered, written to, cancelled or moved once Joel's sure, its link copied and its attachments
    opened; Google signed out wants a sign-in, the transcripts list each call written down and
    open it in the reader, and Meet joins a call by its code
  '';
  vm-desktop-launcher-keeps-up = ''
    an arrow held at 30 a second at home, in Agents and in Programs, at scales 2 and 1.5, while
    300 sessions and 2,000 people change, makes no rows for a source the place doesn't show,
    none at all while they're frozen, and none looking for where rows live with no Ctrl+]
    pressed; refreshes the list at most four times a second and never on a key's path; and
    composes it no more often than it refreshes, drawing no row afresh while they're frozen;
    and with the launcher shut, 113 sessions moving only their steps cost the shell no sorting
    until it opens
  '';
  vm-desktop-launcher-moves-windows = ''
    the launcher opens kitty and learns it was meant, frames the focused window, switches to and
    moves windows by their titles, Super+Tab goes back to the window before, and Super+Shift shows
    every layer's shortcuts
  '';
  vm-desktop-launcher-reads-mail = ''
    Now counts the mail and the Slack that want Joel, mail opens by its subject in one Mail window,
    is replied to, marked read, archived and its files fetched; Slack's program holds its unread,
    a person is found with the ways to reach them, Docs lists what Joel opened lately, and Search
    Slack ends a search, asking Slack once on Return
  '';
  vm-desktop-launcher-waits-on-joel = ''
    Now leads with what waits on Joel and For You's to-dos hold only what's true, one waiting on a
    file going by itself as the to-do daemon finds it done, a session wanting him waits as the
    session, its notice going to it and going read once it stops waiting; the agents list every
    session from the shell's start and hold still as one changes, and pi's runs are found
  '';
  vm-desktop-locks-and-asks = ''
    the lock screen takes only the right password, lets go when released or told to by logind,
    and holds through a shell's restart; polkit's prompt and the askpass answer and hold
  '';
  vm-desktop-looks = ''
    on the laptop's screen at 1.5 and at 2, the launcher's drawer takes the room beside the card,
    up to 520, and every line of it lies on one of the card's to a hundredth of a pixel: its
    eyebrow on the field's centre, its title on the first row's baseline, and its facts and words
    a whole number of half rows below; and a row's words, home's lists and a found row's note,
    are in the serif a step after its name and on its baseline to half a device pixel; and WARP,
    from the real warp over a stand-in warp-cli, stands in Network connected, degraded and not
    answering, for its gallery to shoot
  '';
  vm-desktop-looks-machine = ''
    on the laptop's screen at 1.5 and at 2, Machine's processor, memory, disk and uptime step
    into the parts the machine daemon reads from the guest, and those parts into their own;
    Machines holds this machine's row first, saying its model and system as its realm's node
    measured them; Heat opens a chip's drawer, the Charger a USB-C port's, Firmware an offered
    update's, its Security Level says what falls short and steps into each attribute, and
    Sleep says where it failed
  '';
  vm-desktop-looks-programs = ''
    on the laptop's screen at 1.5 and at 2, Drive's places and Recent, Spotify and its podcasts,
    what mpv stopped, GIMP's acts and Recent, Neovim's files open in two kitty windows, Chrome's
    tab and latest download, Tuple's Recent, the VMs, kitty's repositories and machines, btop's
    drawer saying it runs in a terminal, Shortcuts over Mail and a Docs tab, Passwords, an entry
    and its copy in the margin, and 1Password's program
    holding Check Every Password after its vaults, each drawn for a person to judge
  '';
  vm-desktop-minds-the-margin = ''
    the margin shows keeping awake, lid shut too, a camera opened, tokens running out, do not
    disturb, For You's tray lit by a critical notice unread, a wait lasting a minute, the muted
    microphone, dictation, what's playing and caps lock, and Now says each and answers the keys
  '';
  vm-desktop-opens-and-sets = ''
    the launcher sets sound and opens its outputs' and microphones' parts, works a program's media,
    sets the power profile, scales displays, joins Wi-Fi and names each connection's addresses
  '';
  vm-desktop-opens-files = ''
    the desktop opens files: a screenshot found by typing, the portal's chooser, its save's name
    edited on the line and a name taken asked once, and Google Drive through it, a Google
    document saying the file it comes as and gx opening a file in its own program, a picture in
    swayimg, where : opens the launcher's command line alone and low, aimed at the picture by its
    name, and :q there closes it, a document in zathura, markdown in the reader, a folder from
    kitty's Recent, and a buffer of a Neovim in kitty from Neovim's page, which lists every file
    each one holds, while the launcher's home keeps its shape
  '';
  vm-desktop-outlives-a-switch = ''
    a switch of the home from the sessions' kitty as it was, with no switch method, onto the one
    kept through a switch, with a changed command line, leaves the same kitty running with the
    same sessions' windows in it, and a switch changing the shell beside a unit 30 s slow to stop
    restarts the shell once that stop is done, with a new shell under 5 s after the old one
  '';
  vm-desktop-reopens-under-orchestrators = ''
    sessions come back, or start, on their orchestrator's workspace by themselves: what
    restore.json kept of two orchestrations goes where the shell kept each window, two stacked in
    one column too, and one its orchestrator hands over again is begun once, and one started for
    another's tree opens with it, in title order after its orchestrator, with Joel's window
    keeping the keys
  '';
  vm-desktop-says-what-happens = ''
    notices come, go and are found again, are signed by the name Joel knows, are acted on by the
    keys and the pointer, bring what they're about forward, and sit inside the window under them,
    and the launcher's drawer of one takes the keys, which move through it as Vim's do
  '';
  vm-desktop-shows-then-rests = ''
    the meters answer the keys and go, a notice shows and goes, the overview draws nothing of the
    shell's, and once nothing shows the shell draws nothing and barely wakes, with For You's
    tray lit too, under a hundredth of a core, and behind the lock as its files are written
  '';
  vm-desktop-starts-clean = ''
    the desktop boots without an ordering cycle and its shell without an error, niri alone keeps
    its priority, programs wear the theme, the wallpaper stands behind the workspaces, and swipes
    and Bluetooth's agent work
  '';
  vm-desktop-takes-calls = ''
    Tuple's daemon starts before the other apps, a call shows in the margin while it lasts, people
    coming and going are said, a call that ended is in Tuple's Recent in Programs to call again,
    a ringing call wants Joel and is kept as missed, a copied link joins, and the call's windows
    find their places
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
  swap-file-grows-only-when-it-may = ''
    a swap file smaller than asked for grows as the machine starts, and one in use, holding a
    hibernation, short of room or big enough stays as it is
  '';
  vm-full-swap-ends-what-filled-it = ''
    full swap ends the cgroup that filled it through systemd-oomd, before the kernel's OOM killer,
    and leaves the rest
  '';
  vm-greeter-signs-in = ''
    the greeter signs in and out, not waiting on the account's setup; greetd leaves its console
    in graphics, and tuigreet, after the greeter falls over, finds it in text again, even after
    a compositor that fell over before taking the screen
  '';
  vm-guide-installs-beside-another-system = ''
    the guide installs a machine with no TPM and no Secure Boot onto a disk laid out as Asahi's
    installer leaves a Mac, confirming only its own partition and running no Secure Boot step:
    it makes the machine's partition in the free space and names Asahi's boot partition for it,
    every other partition the same byte for byte and every other table entry unchanged, Asahi's
    files kept beside systemd-boot; the machine boots by a YubiKey's PIN, its guide with nothing
    to seal, and its disk opens by the recovery words alone
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
    from the boot partition, and gives a root shell inside; and the stick carries Apple's restore
    image whole in a partition of its own, which mounts only for reading, beside the usbmuxd that
    `guide revive` starts
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
    a machine hibernated to a swap file on btrfs inside LUKS, with zswap in front and no resume=,
    comes back from an lz4 image through systemd's initrd as it was, once the TPM and a PIN typed
    at its prompt open the disk, lets the lock go for that one prompt, says how long the restore
    took from the count its initrd kept, and boots anew after; with
    more held under a slice's hard memory floor than the image may hold, it still comes back, the
    floor as it was; with more random bytes than the image may hold, it still comes back, what
    was swapped out for it on the disk and none in zswap's pool, zswap on again; and asked first,
    says a quiet machine's image fits, and with no room exits 1 saying not, plainly
  '';
  vm-idle-sleep-comes-only-on-battery = ''
    a machine left idle on the charger stays awake, and sleeps once the charger is unplugged, even
    as the last of a burst of twenty power supply events; on battery a session still in use keeps
    it awake, and an idle one stays awake while a program holds logind's lock against idling and
    sleeps once it lets go
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
  vm-lid-shuts-with-the-hinge = ''
    logind hears lid-hinge's switch and not the lid's own: a lid shut with the hinge coming down
    sleeps, one shut with the hinge wide open and the light on stays open, and one slammed shut
    sleeps once the light goes dark
  '';
  vm-lid-stays-at-the-desk = ''
    a shut lid keeps running on the charger and sleeps once it's unplugged; on battery it sleeps
    with no monitor, keeps running with one plugged in whose screen is off, holding one lock
    through the card's changes, and sleeps once it's unplugged
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
    stage, and Plymouth hears which; an account's units that ignore SIGTERM hold its manager under
    ten seconds, alone or in a reboot
  '';
  vm-root-builds-only-where-it-trusts = ''
    nix-offload's lines start as their own account with their settings, read the token the
    lender's account leaves them, and give root the lent builder's line the cloud vouches for and
    a standing one's, never a forged host key's, in files the lender's account can't write; with
    nothing lent, the lent line goes and the standing one stays
  '';
  vm-screens-come-and-go = ''
    the launcher, a notice, the margin's marks and dictation's bar come back after the screen goes
    and returns, after a console switch and after a sleep and wake, after which the shell still
    hears a notice, dictation's keys and Keep Awake let go; desk-facts stops naming the screen
    once it's gone and names it again once it's back; and when one of two screens goes, the
    margin and dictation's bar stay on the other and the margin comes back on both
  '';
  vm-screens-keep-places = ''
    niri started with the laptop's lid shut on a dock connects the monitor alone, never the
    laptop's screen, until the lid opens, holds the monitor's first frame, and from before that
    hold takes a switch's settings; keys typed through the virtual keyboard keep the session
    from going idle; a shell starting after a reboot reopens the sessions its list names, each
    where it sat, and never what a boot's end left or on a restart alone; sessions reopened
    while the screen they sat on is gone go to it, in their order, once it comes back; and a
    screen back takes back its workspaces
  '';
  vm-screens-keep-the-launcher = ''
    the launcher opens, finds and runs a row on either of two screens, a Return held doing it
    once, as each goes and comes back, as both go, and after a sleep, its foot still telling the
    time after the wake; Quickshell warns of nothing meanwhile, Qt's own text input and absent
    daemons aside
  '';
  vm-secure-boot-installs = ''
    the whole install works, and the disk opens only for it; the firmware starts the loader by
    an entry of its own, first in BootOrder; shut down with fwupd's firmware
    update waiting, it frees PCR 0, and the boot after opens with the PIN and locks PCR 0 again
  '';
  vm-speakers-are-tuned = ''
    the XPS 14's speakers hear the tuning, headphones don't
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
    tin both ways, the jobs' leaving marked CS1, what Joel waits on at a terminal and an app's pages best effort, its calls, its
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
    refused, a main the health check refuses is switched to and then gone back from, and a switch
    made by hand brings the update-to-main it switched to, which answers the next ask; and a
    machine following its host's word builds main unsigned, root reading the objects someone
    fetched into their checkout and the token asked as someone with none of root's groups, with
    no token builds nothing, and building elsewhere with no builders refuses a piece not marked
    light; a signed commit to a followed input's main alone moves the machine, an unsigned
    one is refused; and on a machine booting through systemd-boot, a main with another kernel
    runs its userland at once, restarting the service it changed, while the booted kernel and
    its modules stay, and the reboot after starts the new kernel
  '';
  vm-webcam-microphones-yield = ''
    a webcam's microphone ranks below a USB microphone's, again once it's plugged back in
  '';
  vm-wifi-holds-on = ''
    nothing that should leave the card on its network takes it off: wifid or networkd or
    wpa_supplicant starting again, another card, a garbled file, a VPN's DNS servers; others'
    routing rules stand, and a network forgotten goes from everywhere
  '';
  vm-wifi-joins = ''
    wifid joins the networks it's given, finds sign-ins and sessions ending, tells the shell what
    it hears in detail, chooses a network the launcher saved first, and trusts one when asked
  '';
  vm-wifi-joins-when-asked = ''
    the shell has wifid sign in, join with a password or an account, leave, save and switch the
    radios; a stuck look is nudged, a silent access point passed, and the card dozes only on
    battery
  '';
  vms-reach-the-launcher = ''
    a workstation with a desktop gives its launcher the VMs, read through the daemon's own virsh,
    under a hidden program the launcher lists only while one exists, and one without the shell
    gets none of it
  '';
  vms-read-as-virsh-says = ''
    virsh's lists and every figure dominfo gives are read, its OS type and security DOI aside and a
    security model of none saying nothing, every name asked of dominfo in one
    batch and read back whole however it's spelled, each figure it left out named with why, memory
    said in decimal units from its bytes and in use only while it's on, each state is said in the
    launcher's own words rather than libvirt's, each state offers its own acts and tells virsh its
    own command, and the VMs are ordered and counted as Programs shows them
  '';
  warp-passes-clippy = ''
    clippy finds nothing in warp or its tests
  '';
  warp-reads-what-warp-cli-says = ''
    warp draws WARP from every answer in its contract exactly as warp.json holds it, keeping a
    reason's own figures, an endpoint given as no address as none and a figure left out as unknown;
    tells no warp-cli, a service not answering or hanging, and a status refused or garbled apart,
    each said in a line with a code of its own and with --json as a record; still reads the rest
    when a part besides the status isn't given; draws each page as its golden one; and connects
    and disconnects only where the organisation doesn't hold the switch, saying when WARP always
    on comes back and warp-cli refusing in its words
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
    clippy finds nothing in whereabouts, which holds where the laptop is and the clock's zone for
    the shell, or its tests
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
    jobs is slowed past half of memory and agents' sessions past 40%, with no hard limit that
    would end their work
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
  yubikey-keys-builds = ''
    the key maker for a Linux machine with no TPM builds, shellcheck and all
  '';
  yubikey-on-every-workstation = ''
    Linux and Mac workstations both have the YubiKey's tools
  '';
  zathura-accepts-its-configuration = ''
    zathura opens a page with the account's settings and stays open without a warning, its bar in
    the monospaced face, and PDFs open in it
  '';
  zathura-command-passes-clippy = ''
    clippy finds nothing in what carries the launcher's command line to zathura, or its tests
  '';
  zathura-escape-closes-no-contents = ''
    zathura's contents close on none of Escape, Ctrl+[ and Ctrl+C, which are unmapped there, and ZZ
    closes them as a window
  '';
  zathura-keys-go-where-they-say = ''
    every key zathura lists lands where it says: J, K and the page keys turn the pages, Home, End,
    gg, G and a count before G go to that page, the arrows scroll as j, k, h and l do, Ctrl+D,
    Ctrl+U, Ctrl+F and Ctrl+B scroll, Tab, Ctrl+O and Ctrl+I go through the jumps, m and ' mark
    and go back, / and the question mark key find on and back and n and N again, Space and the
    question mark key ask the launcher for the keys, the colon asks it for its command line over
    the window, whose pages and zathura's own commands come back over the bus, ]b and [b open the
    next document in its folder and the one before, with a count that many and coming round, q
    closes nothing, and ZZ and Space w close it
  '';
  zathura-next-document-passes-clippy = ''
    clippy finds nothing in zathura's ]b and [b, which open the document beside, or its tests
  '';
  zathura-says-the-document-it-has-open = ''
    zathura says the document it has open, read by its Recent's command as its process and whole
    path, the page it's on counted from one and how many it has, and nothing once ZZ closes it
  '';
  zathura-says-the-files-it-opens = ''
    zathura says each document it's started with to the desktop's list of files opened lately, then
    runs
  '';
  zoom-is-installed-or-opened-on-the-web = ''
    an account with zoom has Zoom's own client on x86_64 Linux, and on ARM Linux, where nixpkgs
    builds none, Zoom's web client as a program that opens Zoom's own links instead
  '';
  zoom-opens-its-links = ''
    each of Zoom's own links joins its meeting in the web client, with its passcode only when the
    passcode is plain, and a link naming no meeting, or nothing, opens the web client's home
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
