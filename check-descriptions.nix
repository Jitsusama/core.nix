# What each check proves, by its name, as a sentence's predicate: checks.nix puts each in its
# check's meta.description, and every-check-says-what-it-proves fails on a check with none or
# one with no check. A check added names what it proves here, in its place by name. Long
# because it's one table, a line or two for each of core's checks.
{
  a-bare-machine-takes-a-modules-own-defaults = ''
    a module's own defaults win over what each bare machine sets for itself: the NixOS one's
    platform, boot loader, disk and state versions, as a hardware file sets its platform, and
    the Mac one's platform, primary user and state versions
  '';
  a-bluetooth-keyboard-may-wake-it = ''
    a machine with Bluetooth lets a USB or PCI controller of Bluetooth's class wake it, so a paired
    keyboard's key does, in a rule udev reads without a complaint
  '';
  a-card-says-how-it-stands-in-one-row = ''
    a card's status line, as the launcher's is, gives its middle hints way first
    as room runs short, the keys list's and Return's staying
  '';
  a-check-copies-what-each-file-imports = ''
    the shell's checks copy every file a root reaches through imports.nix and nothing else, a
    row past the root listing its imports in another order copies the same files, and one left
    empty or dropped, or one past the root's own row naming an import more, is refused,
    copying nothing and naming it, each case taking its file from imports.nix as it is; a
    program's own file importing that file copies what copying it does and nothing else, and
    one importing a file the shell's folder doesn't hold is refused, naming it
  '';
  a-claimed-chord-is-refused = ''
    a chord two layers claim, or one claims twice, is refused
  '';
  a-desktop-wait-says-what-it-last-heard = ''
    a desktop area's wait on what the launcher reports that never passes says the last state it
    heard, one a change passes ends there, one whose first read passes hears nothing more, each
    giving back the state that passed, a test's own assert fails the wait the same way, a test
    that trips keeps its error, noting what it last heard, a line that isn't JSON is noted as
    it came, and a wait whose hearing stops before a state passes fails rather than giving back
    a state its test refused; the reader it waits through, run on a stand-in journal, reads it again
    from the point it's given until the report comes, says what follows a line's first report
    mark, and its time running out fails as never hearing while
    a failed read fails as a stopped reader; and a read for a failure's diagnosis never raises,
    saying a cut-short read, a failed one or what it raised
  '';
  a-failed-boot-says-what-stopped-it = ''
    a failed boot's shell names up to four failed units and their last lines, fitted to a
    terminal's rows with eight left for the shell, in the account's theme with the console's
    marks, and the way out, in time when the journal stalls, why a
    unit's journal gave nothing when it couldn't be read, once which file the journal skipped,
    and nothing when none failed
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
  a-getter-turned-leaves-other-shapes-whole = ''
    in the shell's own Qt, turning a field into a getter in objects whose shapes grow one from
    another leaves every other field of each reading what it was given
  '';
  a-live-site-stands-alone-as-a-folder = ''
    the system page, built on the real web kit and laid out as a folder that stands alone, has
    its page at the root, reading the theme and the kit from its own web/ and kit/ with no strip
    and the page whole; a site that still loads something from outside fails its build, naming
    the file and the address, whether a page's link, image, object, poster, background or
    xlink:href in any case, quoted or not, past a quoted > or after a link's open quote, a
    style's url() or @import in any case, inline or in a stylesheet, a module's import, bare
    or dynamic, or its fetch, the global one's too, spaced, holding the other quote, wrapped
    over lines or after two comments or one's closing line, in its file, the kit's or inline,
    an address at a root, on another host, of data:, built whole as the module runs or added
    to from a climb, or a file the folder lacks, a tag's template taken as written, as does a
    srcset, an import map, an image-set(), a strip whose ends don't pair, share a line or no
    longer read, a site with no index.html, or a folder laid over one already there, which
    stays as it was; a refused build leaves no folder; and a link to another of the sites
    stays, as do a data-src, a query, a folder's index, an address built from a start as a
    module runs, by a template or by adding to it, in its file or inline, a page's new URL()
    and another object's fetch method
  '';
  a-loaded-desktop-counts-what-its-builder-took = ''
    keeps-pace counts, from /proc/stat, the guest's busy ticks and those its builder stole in each
    turn, under load and frozen, and the share they give, how much more of its busy time the guest
    ran frozen, never under one nor past the VM's processors: nothing stolen gives 1, half stolen
    under load alone gives 2, what was stolen frozen weighs against what was under load, every
    tick stolen in both turns gives 1, every tick stolen under load alone the processors, a turn
    with no busy ticks none, idle, iowait and guests' time are never counted busy, ticks land in the
    turn they were counted in and those while the load turned, or after a turn that never
    finished, in neither, each turn keeps every tick it counted, idle with the busy, so its busy
    ones weigh against the guest's own clock, a thread whose flags carry PF_NO_SETAFFINITY, read
    past a name holding parentheses, reads as one the kernel won't let be held, and what a
    subtest says names each turn's ticks and the share, or no share when a turn had no busy ticks;
    and a builder whose frozen loops ran slower than the slowest normal one's stretches the keys
    bar's share of the frozen time, not its fixed ms, by their middle, the lower of two, over that
    normal builder's, never under one nor past one and a half, where loops under load and a
    subtest with no frozen loop stretch none, so b508ac2e's keys fail the bar unstretched and pass
    it stretched, and keys six times as slow as its frozen key fail however slow the builder
  '';
  a-long-name-reads-at-one-pace = ''
    a selected name too long for its row slides at one reading pace, gathering speed from rest and
    settling back to it in at most a fifth of a second at each end, never stepping back; and words
    too long for the field's end give way at a whole word, no separator left before the ellipsis;
    and the field's path shares its room with a folder's path giving way first, down to its least,
    before the other steps share what's left
  '';
  a-look-leads-to-its-cell = ''
    a look's Talk and Go To reach the session that left it, the agents daemon's answer for its
    cell, or a session on its quest, and another quest's only as the one that left a look on no
    quest, an old look naming no cell is led by the one its session goes by, a Go whose session
    works on another quest now says it moved on, and a served look settles only once it opened
  '';
  a-low-charge-is-said-once = ''
    a battery that reports only its charge runs low at its mark and critical at its own, only ever
    deeper until it charges or rises five points past the low mark, so a charge wavering about a
    mark is never said twice
  '';
  a-low-device-is-its-own-notice = ''
    each connected Bluetooth device reporting a battery stands by its own charge, and each running
    low is a thing of its own, keyed by its address, signed by its name and drawn by its kind's
    glyph, wanting Joel now only at the critical mark; an adapter bluetooth-links says stopped
    answering is a thing too, the Adapter's row in Bluetooth, its notice naming bluetooth-again
    for one on PCI and a restart for any other, and saying a device may need pairing again
  '';
  a-mac-refuses-another-accounts-pi-hosts = ''
    a Mac refuses an account other than its primary one that names a pi host's realm, by that
    account's name, and gives it none; a Mac naming no primary account refuses every realm with a
    message saying so, and gives none; and a realm giving a DYLD_ variable is refused, naming the
    realm and only that variable
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
  a-machines-unfree-names-join-cores-on-darwin = ''
    an unfree package a Mac names for itself is allowed beside those core names, and the Mac
    workstation, with every module core gives a Mac, still refuses one nobody names
  '';
  a-machines-unfree-names-join-cores-on-nixos = ''
    an unfree package a NixOS machine names for itself is allowed beside those core names, and
    the NixOS workstation, with every module its role brings, still refuses one nobody names
  '';
  a-macs-sessions-come-back-in-each-realm = ''
    a Mac whose account joins realms runs sessions-come-back as its login starts and each
    minute, on mycelium and every realm it joins, and a Mac in no realm runs none
  '';
  a-notice-comes-forward = ''
    once a notice is opened, its sender's window comes forward, the one used last, unless one of
    its own is in front already or it has none open
  '';
  a-notice-found-is-noted-by-what-was-typed = ''
    a notice found by a search is noted by the line of its words holding what was typed, worked
    out only once it's read, and its card offers its sender's acts but the one that opens it
  '';
  a-capture-says-what-it-took = ''
    a screenshot's or a recording's drawer says where it is and what kind, when it was taken to
    the second, its size, and a picture's dimensions or a recording's length once they're known
  '';
  a-change-mends-only-what-moved = ''
    a change saying only what differs mends the record a daemon's client holds, a field said null
    let go and one said deep mended where it moved inside it, what didn't move kept as it was, and
    a kind's fields are said deep only where its watch asked and the daemon's hello offers them
  '';
  a-piece-says-all-it-knows = ''
    the drawers of Now's own pieces, the disk, the notices waiting, lag, a screen share, a
    recording and do not disturb, say where and what they are, then what moves first, and their
    identifiers last, leaving out what they can't say and what the row's side already says; and
    do not disturb, off, says until when Return would hold notices and what a screen share does
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
  a-repository-link-lands-on-its-row = ''
    a mycelium:repository/ link's key picks the row whose served key is that whole key and no
    other, picks none for a key missing from rows read whole in the watch now running, and
    judged by rows held from an earlier watch, or by none, picks the row to land on as it comes
  '';
  a-row-made-again-keeps-every-field = ''
    a collection's row made again from a row made, as For You makes a session's from Agents', keeps
    every field it is given over 60 shapes grown from one another, and makes its own name ready to
    match from its own name, once, only when it is read
  '';
  a-row-kept-is-never-drawn-again = ''
    the launcher's list keeps a row listed before and now wherever it moves, a key listed twice as
    two rows, and draws afresh only a row new to the list, the same rows taking no step and a row
    falling or rising any way taking one
  '';
  a-hosted-viewer-reopens-where-it-was = ''
    Reopen Them All opens only the viewer of a session pi's host brought back, never its pi,
    quietly where the last boot kept it, and keeps the place of one no host serves yet, opens none
    for one kept nowhere, and holds one bound for the workspace Joel is on until he leaves it; opens
    a viewer and a plain pi kept where their orchestrator isn't at their kept place; and of plain
    pis, brings back held,
    at the walk's pace, those kept nowhere, even one whose folder went, but in a window when no
    boot's places are known
  '';
  a-screen-niri-is-late-to-name-takes-its-workspaces-back = ''
    a monitor plugged back in takes back the workspace niri kept on the laptop's screen, leaving
    the one niri gave back, even when niri never answers the shell's look at the screens as the
    monitor comes, the shell looking again while workspaces wait
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
    put off, then those another machine carried here, newest first, never put off; and Reopen
    Them All's lists given before niri says its screens count what they find gone as one walk
  '';
  a-shut-launcher-makes-no-document-rows = ''
    the launcher's Documents make no row while it's shut, as the clock's minute passes or the
    google daemon tells a row, then hold every document once it opens, made once in the daemon's
    order, and keep them, making none, once it's shut again; and a search, or what's typed inside
    a row, lost as the daemon goes while drawn is asked again once it says hello, still timed from
    its key, lost again left for the next key
  '';
  a-sign-in-notice-is-taken-back-even-late = ''
    a network wanting a sign-in is told of once, its notice staying while it wants one and taken
    back once it doesn't, even a notice arriving only after its network was signed in
  '';
  a-silent-hub-is-driven-as-one-stream = ''
    the XPS 14's kernel drives an MST hub whose first branch didn't answer its link address as a
    single stream until the sink goes away, so the monitor behind it still lights
  '';
  a-socket-asking-for-nothing-holds-a-burst = ''
    a machine with a screen gives a socket that asks for no receive buffer 2 MiB to receive into,
    under a 4 MiB most that one asking can reach
  '';
  a-spawn-kept-in-the-background-opens-no-window = ''
    a fresh session handed over is asked of the harness's spawn decider first: one it keeps in the
    background opens no window, one it keeps out opens told why, one it says nothing of in three seconds is ended and
    opens, one it fails on opens as before, and a saved one opens at once, never asked
  '';
  a-stick-carries-apples-macos-apart = ''
    a stick given Apple's restore image lays it last, as a partition of its own named for its zip,
    with a usbmuxd that skips pairing and a line saying `guide revive`; core's own stick has none
  '';
  a-switch-leaves-timed-runs-be = ''
    a timed oneshot saying nothing of its own keeps its run over a switch, which sd-switch reads
    as keep-old, and its run ends after three hours; its own method, legacy key or TimeoutSec is
    read rather than overruled, another type and an untimed service restart as before, and a
    timer naming its service, or running a template's instance, sets them on that service
  '';
  a-talk-answers-what-its-session-asks = ''
    Talk to It on a session asking shows what it asks whole, with what it takes in a field of
    its own apart from the question, and that alone where it says nothing of what, or only
    spaces; words answer it by a choice's label, else its value, or as its own words where it
    takes them, and read sent, then answered or working, or not delivered and why, kept newest
    first, only those to what it asks while it asks, with any not delivered in the last quarter
    hour, and all of that quarter hour once it asks nothing; one asking what Talk can't answer
    shows only those not delivered
  '';
  a-talks-words-outlive-the-shell = ''
    what's typed to talk to a session is kept by its address with its caret for a week and read
    back as a shell starts; words sent are kept until their talk is delivered or replied to, and
    kept when it fails; a week-old file, or one not written by the shell, gives nothing back
  '';
  a-terminal-sudo-goes-through-run0 = ''
    sudo typed at a NixOS account's zsh, with the sudo module, hands a command or a root shell
    alone to run0, whose polkit prompt takes the key's touch, and every form of sudo's own, such
    as another user, to sudo
  '';
  a-theme-changes-everything = ''
    another theme leaves none of the first's colours behind, gh's markdown style among them, but
    in the web kit's other look, and the search passes over a link it can't follow but never a
    colour it found, a path that isn't there, an empty list or any other error
  '';
  a-things-drawer-says-its-parts = ''
    a thing's drawer says each part as a fact, its acts left out, a holder of figures opened in
    place and a level's bar a stroke; the backlight daemon's files say whether the room sets the
    level and what it's on its way to past a gap too slight to move, the room's light with the lid
    shut or that none is heard, measured to three digits in lx or klx, and every point of its
    curve, dimmest first, kept where the daemon keeps it, its lux every digit in the same units;
    the machine daemon's backlight says its steps, those read back only where they differ, its
    stepping and driver only where the kernel says them, and its sensor its count grouped, its
    lux a count and an offset only where it isn't none; and the facts that identify a thing end
    its drawer, marked, each said once
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
  a-turned-mark-keeps-its-ink = ''
    the theme's down triangle is its triangle drawn a half turn about the ink's middle, wide at
    its top, with every edge of its ink within a device pixel of the upright one's, at 2 and at 1
  '';
  a-want-rings-low = ''
    a machine with pi plays pi's low bell struck twice when a session wants Joel, not the theme's
    high message
  '';
  a-watched-file-comes-back = ''
    the shell's status file hears a status once its folder is made, after the shell starts and again
    after it's removed, and says nothing while it's gone; a view moved to another file, or watching
    again, hears as it should with the one watcher it first made
  '';
  a-yubikey-touch-types-nothing = ''
    a YubiKey's keyboard interface is refused in the running system and in the initrd, so a touch
    types no one-time password at the disk prompt, and udev reads the rule without a complaint
  '';
  agents-cargo-goes-through-nix-offload = ''
    the dev shell's cargo sends an agent's words, from pi in a terminal or a hosted session, each
    kept whole, through nix-offload cargo naming cargo itself, and runs cargo itself for a person,
    one whose session variable is empty, an agent's or a hosted session's on a lent builder, and
    without nix-offload or beside one too old to know cargo
  '';
  agents-lists-spec-once-on-niri = ''
    on the account's whole shell on niri, over the cadence-spec daemon itself, Agents lists Spec
    once, the cursor staying on it as the daemon's answer changes, and a step held in Spec takes
    the rows the daemon serves after a restart; over spend-watch serve, a - out of Now makes
    neither Spec nor Budgets again while what each is made of stands, a change made while Now is
    held shows after, and Budgets goes while spend-watch is down
  '';
  agents-makes-the-launchers-rows = ''
    the agents daemon says how each pi session stands, its acts and what Return does, lays
    sessions under their orchestrators with the waiting folded, and words each row's side, each
    drawer, what matters first, and the sessions together as the launcher's vocabulary says them
  '';
  agents-passes-clippy = ''
    clippy finds nothing in the agents daemon or its tests
  '';
  agents-password-prompts-are-closed = ''
    the windows daemon closes a password prompt an agent's scope opens on niri within a second,
    says so in its journal and tells Joel the scope and folder it came from, once for a second
    prompt from that scope, and closes one already open as it starts again, while his own prompt
    and the scope's other windows stay open; the scope's terminal, opening focused, is put on a
    workspace no screen shows and the window focused before it has the focus again
  '';
  agents-runs-in-each-account = ''
    the agents daemon runs in each account of a machine in a realm, following its node and the
    to-do daemon and reading the launcher's words and the record of switches, its socket
    systemd's, a switch restarting it, and on a machine in no realm follows no node but still
    serves the sessions to reopen
  '';
  agents-work-runs-without-real-time = ''
    the jobs sorter takes real time from work under jobs.service, jobs-work.slice and
    jobs-timed.slice, and from nothing that holds Joel's own terminal work
  '';
  an-agent-raises-no-polkit-prompt = ''
    a workstation's polkit rules refuse a process where agents' work runs (agents.slice,
    jobs.service, jobs-work.slice, jobs-timed.slice, shared.slice or a pi-jobs scope), or one gone,
    whatever would ask Joel for a password but run0 or pkexec, which asks him once for its command
    while his local session is in front, and a unit's verb named in agentsMayRepeat, which is
    granted, leave his terminal and apps to ask, keep a grant, and take only a system unit's whole
    name, never one run0 makes
  '';
  an-account-switch-hears-each-job-end = ''
    every NixOS account's activation switches its units with an sd-switch that reads each job's
    end as it comes, so a switch of more than 64 jobs doesn't hang waiting for a reply
  '';
  a-disks-writeback-throttle-is-the-machines = ''
    each disk a machine names in jitsusama.slices.writebackThrottle has a unit, started at boot,
    by a switch and as the disk appears, with no condition to skip a name that reaches nothing
    and stopped as the disk leaves, that sets its writeback throttle's goal, each its own, and
    gives back what the disk had as it stops; a machine naming none has no such unit, and a name
    that isn't a disk's plain one, or a disk io.cost shares, is refused while another disk beside
    io.cost's is not. Whether the goal reaches the disk is the switch's to show, where a unit
    whose name reaches no throttle fails
  '';
  an-agents-run-stays-in-its-slice = ''
    systemd-run puts an agent's --user unit in the agent's own slice, or in jobs-work.slice when
    the call comes from jobs.service, a pi-spawner scope or a pi host's or conversation's unit,
    or below one, reading only its own options, bundled and shortened as its parser reads them
    by a table held to every option the wrapped systemd takes, hidden ones too, keeping the
    slice for a name or letter no option has, and leaves Joel's own calls, his services in
    jobs.slice, the windows' kitty and the node in agents.slice, a path naming those places
    only inside another's scope, a call naming a slice and a call to the system manager as they
    were; and that the hold refuses, each by the step that says so, a table short of an option
    or reading one as valued that isn't, an option the table can't say, and a systemd-run that
    can't list its options, failing its build, while it holds a stand-in saying them unchanged;
    that the wrapper isn't built without the hold, which is to the systemd it wraps, or for an
    aarch64 machine built here to the builder's own systemd built alike; and that one with a
    patched systemd, one from another source, one built with a flag or an override argument of
    its own, one of no version, or one cross-built from a Mac, is built unheld with no hold to
    read and says so where a held one says nothing, a Mac told only to build on the machine's
    platform and the rest what differs, the flags by name, and that a machine cross-built with
    a patched systemd warns of it
  '';
  an-arrival-is-said-once-each-time = ''
    what comes true by itself is said once, and nothing of what stood as the shell started; a
    piece that tells something new is said again, its own earlier notice taken back first and no
    other's; one whose words alone change is said again in place, by its id, and not once Joel has
    put it away; and one that goes takes back whichever of its notices is up
  '';
  an-emoji-stands-on-its-name-on-niri = ''
    each of Super+E's groups, as niri shows the launcher's rows drawn through Qt's GPU path at 1.5
    and 2, has its emoji's middle within a device pixel of its name's, halfway from the top of its
    capitals to their baseline
  '';
  an-emoji-stands-on-its-names-capitals = ''
    thirteen emoji, the first of each of Emoji's groups as they stood on 19 Nov and four below the
    emoji blocks, drawn through the one mark, have their ink's middle within two device pixels of
    the capitals beside them, at 2, 1.5 and 1, and none of the theme's marks, nor six letters
    beside the pictures, is taken for a picture
  '';
  an-ending-row-arms-before-it-ends = ''
    a first Return or click arms a row that ends the session and a second with no key between ends
    it, however long after; any other key lets it go, and a press doubled with the first does
    nothing
  '';
  an-ipc-reply-arrives-whole = ''
    a Quickshell IPC reply of a million characters reaches `qs ipc call` whole, and a call to a
    function that isn't there ends qs with a failure
  '';
  an-untrusted-client-passes-only-named-env = ''
    the Nix daemon hands a client that isn't trusted only the impure-env variables the machine
    names, and only to fixed-output builds, however the client asks, and logs no value, so a
    private fetch works without trusting the account that asks; the daemon's and the door's
    units carry the names, and nix.conf, which other Nixes read, none
  '';
  an-unasked-switch-never-brings-back-a-touch-alone = ''
    polkit lets a switch to a system the profile holds go unasked, but not to one whose PAM took a
    YubiKey's touch alone, in a service other than polkit's or in polkit's without the check that
    the person is in, nor to a system the profile never held
  '';
  asker-says-only-what-a-process-says = ''
    asker-says reads only the quest and session a process names in its environment, in the
    harness's shape, asking each parent up to the account's manager or one it can't read, and
    never says a secret beside them, in its answer or its words; a process gone says neither
  '';
  asker-says-passes-clippy = ''
    clippy finds nothing in asker-says, which says what a process asking for a password says of
    itself, or its tests
  '';
  background-work-keeps-to-eight-processors = ''
    background.slice declares a quota of eight processors' time, so the account's daemons are held
    to it from every boot
  '';
  backlight-passes-clippy = ''
    clippy finds nothing in the backlight daemon or its tests
  '';
  bat-reads-its-theme = ''
    bat colours code in the account's theme from a cache built once, saying nothing, delta finds
    the theme through it too, and a switch builds no cache of its own
  '';
  batteries-leave-time-to-hibernate = ''
    UPower hibernates a laptop at 5%, before it goes flat, keeping its work
  '';
  bluetooth-again-passes-clippy = ''
    clippy finds nothing in bluetooth-again or its tests
  '';
  bluetooth-again-starts-a-controller = ''
    bluetooth-again finds each Bluetooth controller on PCI by its class, has its driver let go of
    it and take it back, hands one no driver holds to the bus, and says a refusal, or a
    controller that doesn't come back, as a failure
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
  bluetooth-recovers-from-an-unread-exception = ''
    the XPS 14's kernel resets Bluetooth's chip after a hardware exception whose dump it couldn't
    read, rather than leaving it halted until a reboot
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
    adapter's traffic, asks the version once and then an idle adapter every thirty seconds, writes
    nothing new while nothing is connected, and replaces its file whole, readable by everyone; says
    an adapter that's up has stopped after three looks of answering nothing, not one down or gone,
    asks one stopped only every thirty seconds until it answers again, and reads what it sits on
    from sysfs
  '';
  bluetooth-pairs-before-it-connects = ''
    the launcher trusts a Bluetooth device before pairing it, follows it past its link coming up
    until it's paired, untrusts one that fails, and connects one paired before; lists a device's
    parts after its verbs, named first: its battery warning low, its signal and link quality as
    sparks of the reads bluetooth-links would write, its channels, connection, an input device's
    facts held by how it's carried, whether it reconnects by itself and wakes the machine as
    settings whose choices set them, and its address to copy; lists the adapter's radio,
    visibility and pairing as settings, its traffic as a spark and what it is; answers how the
    links are doing; says a device running low at its row's side, and an adapter stopped
    answering at its own, holding first the command to copy that starts it again, and on again
    as that fact goes; lists only devices paired, then Nearby, which has the radio look only while
    it's open and lists the named devices heard, strongest first, each with its signal in dBm and
    as a bar, then counts those without a name, which it lists by address when stepped into; and
    says what BlueZ answered a pairing or connecting that failed, in the launcher's words
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
  boot-stopped-passes-clippy = ''
    clippy finds nothing in what a failed boot's console says or its tests
  '';
  booted-generation-stays-installed = ''
    a switch never removes the generation the machine booted
  '';
  btop-draws-in-the-theme = ''
    btop draws in the theme's colours, not its own
  '';
  btop-moves-on-vim-keys = ''
    btop chooses processes on Vim's j, k, gg, G, Ctrl+d and Ctrl+u, a g alone waiting, and goes
    out of a process's details on -, with Escape letting go of nothing, rather than leaving them to
    nothing or another act
  '';
  btop-names-its-window = ''
    btop names its terminal's window btop while it runs, and gives the title back as it quits
  '';
  btop-quits-on-zz-alone = ''
    btop ends on ZZ and on neither Escape nor q, with a process chosen or not, rather than quitting
    on q or opening its menu on Escape
  '';
  btrfs-reclaims-its-chunks = ''
    tmpfiles turns Btrfs's reclaim on for data and metadata
  '';
  cadence-asks-when-a-target-slips = ''
    cadence asks the session keeping the cycle fast once about each missed target or overdue card,
    gives the net's revert the time its checks take before calling it stuck, keeps a must a
    quest owes open until its owner says it's done, tells the orchestrator above it when that session doesn't answer or is gone, tells an
    orchestrator of a session idle two hours by its log's last write, revives a crashed
    orchestrator while leaving one that was ended, marks each of Joel's axes past its bar
    with its owner, and holds the figures health writes, a boot past 30 s, an unread yes to an
    ask for a secret and a disk past 90% used among them, and tells it once of the net hanging
    and of each test the net quarantined for a merge it couldn't revert, naming the owner's PR,
    and tells a merge's owner once when no Mac compared its Mac checks, saying how to run them,
    and says how far the machines' loads spread over the hour, each machine's share busiest first
    and the minutes each of balance's flags was held, from the newer definition alone
  '';
  cadence-hour-finds-nix-store = ''
    the hour's unit has the Nix of the user's config on its PATH, so it can list the closure of
    the build each service's unit names rather than reading every service as unread
  '';
  cadence-hour-gives-back-the-pages-it-reads = ''
    the hour's unit holds a high mark of 512 MiB, past which the kernel takes back the pages its
    journal and history reads charge, no hard limit, whose kill would lose the card, and a
    half-hour timeout, so a throttled hour can't hold back the next
  '';
  cadence-outlasts-a-switch = ''
    a switch is told to leave each cadence run an installed timer starts be rather than restarting
    it and waiting on its start, which under load outlasts home-manager's wait and sends the whole
    switch back, and each is a oneshot setting a start timeout of its own systemd reads that ends a
    hang now that no switch does, a key it can't read refused as such; each timer's run is found
    by the reading the base module finds it with, the tick, day, review and hour must each be
    among the runs found, a failure names each run and what it lacks, stand-in units each wrong
    one way must each be refused in their own words, and ones right in each way allowed must each
    pass
  '';
  cadence-passes-clippy = ''
    clippy finds nothing in cadence or its tests
  '';
  cadence-reads-the-keys-spend-watch-writes = ''
    each finding key cadence's budget measures copy from spend-watch, and each field of
    budgets.json its budget alarm reads, with its file and the folder spend-watch
    writes it in unless its config names another, from the variable each side reads,
    is still the one spend-watch writes, and is a line of cadence's own rather than of
    a test's fixture; each struct written derives Serialize and none is written by
    hand; and copies renamed as each guard means are refused by that guard
  '';
  cadence-spec-passes-clippy = ''
    clippy finds nothing in cadence-spec or its tests
  '';
  cadence-spec-serves-the-axes = ''
    cadence-spec serves a watch of the launcher's Spec the rows cadence prints, and keeps
    answering with none when cadence fails
  '';
  call-audio-runs-in-real-time = ''
    an app's audio thread can take SCHED_RR 8, below PipeWire's, for at most 200 ms at a time
  '';
  sound-services-are-what-runs-a-pipewire-or-sits-beside-it = ''
    sound-services.nix counts a unit that runs a PipeWire of its own wherever it sits, as one
    command, alone, quoted, a line of several after a reset or with a prefix systemd takes, and
    one PipeWire wants in session.slice by a list or one name, but not one it wants in
    app.slice, a program named like it, one naming PipeWire past its own program, one a later
    reset replaced, one handed to the shell, or a NixOS unit switched off, and a home's only
    for its account, in whatever order it chooses them
  '';
  sound-services-are-read-from-the-machine = ''
    the sound services are chosen by the PipeWire a machine runs, a stand-in's program counted
    where nixpkgs' isn't, and a home's are granted under the login name polkit asks with, where
    it differs from the home's attribute
  '';
  sound-takes-real-time-before-the-seat = ''
    PipeWire's three units, each tuning chain and each service PipeWire wants in session.slice,
    by name in the account's session.slice or app.slice and nothing below them, are granted
    rtkit's real time and high priority before the seat has a session, a home's service for its
    own account alone, PipeWire's grant logged with what it runs and one ps can't read as
    unknown, while a watcher in app.slice, an app's scope, the system's units, another service,
    a near name and another account's own manager asking for the home's service keep rtkit's own
    answer with no grant logged, a rule that throws failing its case, and agents' places are
    still refused
  '';
  call-transcripts-hears-echoed-meetings-as-well-as-it-did = ''
    three AMI meetings laid out as calls, the far end heard again in Joel's microphone 20 dB down
    as speakers with no echo cancelling would, are written down with no more of their words wrong
    than the ceiling the last measurement set
  '';
  call-transcripts-hears-meetings-as-well-as-it-did = ''
    three AMI meetings laid out as calls, the far end through Opus as Meet sends it, are written
    down with no more of their words wrong than the ceiling the last measurement set
  '';
  call-transcripts-names-voices-as-well-as-it-did = ''
    the far ends of three AMI meetings, joined one speaker at a time as if over a Meet, have their
    voices named from who was in as each spoke, with as much of what was named named right, and as
    much of the speech given a name, as the floors the last measurement set
  '';
  call-transcripts-tells-voices-apart-as-well-as-it-did = ''
    the far ends of three AMI meetings laid out as calls are told apart into voices, with as much
    of what was named named right, and of their speech handled as it should be (given a voice,
    or left under Others where one talker holds nearly all), as the floors the last measurement
    set
  '';
  call-transcripts-writes-down-both-sides = ''
    a call's two recorded sides come out as one transcript, cut at each side's pauses, in the order
    they spoke, each turn with its speaker and its time in the call
  '';
  call-transcripts-writes-only-what-was-said = ''
    a call writes down only what was said: a loud noise with no speech in it, and a far end that
    only clicks, breathes, coughs, rings, chimes, plays a chord and hums, give no turn and no
    bracketed sound
  '';
  call-under-load-outlives-a-switch = ''
    a switch leaves call-under-load's hourly run and camera check be, rather than restarting one
    under way and waiting on it
  '';
  call-under-load-passes-clippy = ''
    clippy finds nothing in call-under-load or its tests
  '';
  call-under-load-tells-the-hour = ''
    call-under-load counts a call's frame rates by frame slots, its freezes per ten minutes,
    whether the far end heard its sound, the share it made up, its jitter and packets lost, and
    why when it heard none or counted only part, keeps a run ten minutes from any meeting and
    refuses a stale calendar, takes Chrome's and Slack's waits and time blocked from lag-watch's
    last hour, saying why when the kernel keeps none, sets
    the launcher's openings during the run against the same span before it, writes the hour
    in the shape cadence reads, and sums Joel's own calls' sending threads' waits for a CPU
    into a line the hour reads and no test run is taken for, and checks the camera once
    between 18 and 22 minutes before each meeting, never during one or the quarter hour
    before, reading only a loopback, its frames
    to /dev/null, and asking any other camera only whether it answers, and tells plainly
    when it gives nothing
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
    with nobody logged in, and that it can't tell when a PipeWire won't answer; and that a fresh
    call on another of the person's machines is a call unless asked about here alone, which asks
    no node, a stale one none; that one declaring no call probe, one whose probe said nothing,
    one dated ahead and a node that won't answer, drips, floods past 1 MiB or has a full queue are
    each named and no wait; that every node is asked at once, so two mute ones starve none, one
    slowed to three and a half seconds is still heard, and a mute one beside PipeWire's second look
    still answers within nine seconds; that an account's first 16 nodes are asked and the rest
    named, a linked socket or node folder never asked, at most 64 machines named and those past
    them counted once each, a mistyped flag refused while --verbose isn't, and under --json no
    caution is said in words
  '';
  camera-buffers-never-ask-twice-for-big-pages = ''
    the XPS 14's patched videobuf2 scattered allocator never asks again for a block size that
    failed, and asks for blocks above a page without retrying, keeping full effort for single
    pages, read in the allocator's order: its flags and the largest size set before the loop
    over pieces, each piece's size held to the failed one before it's asked for, and the failed
    size kept right after stepping down from it; each patch line is there once, a second ask
    or a second place keeping the size fails, and upstream's lines gone say the allocator moved
  '';
  camera-guard-keeps-agents-off-the-camera = ''
    camera-guard keeps agents off the camera from a quarter hour before each of Joel's meetings
    until it ends, reading a meeting as call-under-load does, keeps the meetings it last read
    through a calendar that can't be read and across restarts, guards a slice made again at its
    next look, says each change once, and its program refuses a camera's node and no other
  '';
  camera-guard-passes-clippy = ''
    clippy finds nothing in camera-guard or its tests
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
    taking it, it runs nothing and exits 75, and it runs here only with --here; a test main
    quarantines is built alone, its failure blocking nothing unless its entries, read together,
    name subtests and its log names a failure outside them, or none, an excused failure said
    with each failed subtest's owner
  '';
  changed-checks-passes-clippy = ''
    clippy finds nothing in changed-checks or its tests
  '';
  choose-by-ear-walks-its-steps = ''
    the launcher's Choose by Ear follows the noise-cancelled daemon's trial: Record and its count,
    the clips heard blind, Return playing one and Pick a step in, every cleaner and its cost once
    one's picked, a refusal said plainly, and leaving discarding anything but idle
  '';
  chrome-leaves-vimium-out-unasked = ''
    an account whose machine doesn't ask for Vimium gets no Vimium settings and no launcher row
    that needs Vimium, so a managed Chrome's launcher lists no keys it can't take, and it still
    lists Chrome's own for the address, finding and following a link found, bookmarks, caret
    browsing, searching tabs and moving by the screen or to an end
  '';
  chrome-maps-land-in-memory = ''
    the four profile folders Chrome writes through shared maps, its histograms and its GPU and
    Dawn caches, are links into the runtime folder once the account's command runs on a profile
    Chrome already filled, its old files gone and the rest left alone; a second run changes
    nothing, a boot's empty runtime folder is filled again, a lock left by a Chrome that's gone
    doesn't stop it, a Chrome still quitting is waited for, as is one whose maps can't be read,
    one still quitting after half a minute, a histogram mapped, is given up on for every folder
    within that half minute, the run starting over if that Chrome quits during the run's last
    look, as it does when only a BrowserMetrics made again on the disk was kept, the caches
    already in memory, and ending within a minute, keeping it all, if it quits after, or all
    but BrowserMetrics, saying so, if it maps only a cache; with Chrome running, or started
    again as the old one quits, no cache is touched, so none is swapped under it, and a run
    ends without a last look's line, as it does when only a running Chrome's BrowserMetrics
    brought that look, the caches in memory, while BrowserMetrics, which
    Chrome deletes itself as it runs, moves once nothing of Chrome's here maps or holds open a
    file in it, a mapped or open file keeping it on the disk and a deleted one or a neighbour
    folder's not, a Chrome on another host keeping it there and a quitting one waited for,
    and, named after caches a held profile keeps, it moves all the same, and what a session left in
    memory goes unless it's mapped, even while another host's Chrome keeps the folder on the
    disk; the unit starts with the account's manager, before the graphical session, and again
    on each change in the profile's folder, as when Chrome quits and drops its lock, with no
    start limit for Chrome's writes to reach
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
  chrome-session-passes-clippy = ''
    clippy finds nothing in what reads Chrome's open pages from its session file, or its tests
  '';
  chrome-session-reads-what-chrome-has-open = ''
    every tab Chrome's session file holds open is read with its window, whether it's in front and
    when it was looked at, a pruned tab on the page it went back to, none of a closed tab or window
    or an app's, a page with a program of its own by its title alone, a command cut off left for
    the next reading, and a file of another version refused saying which
  '';
  chrome-vimium-takes-the-keys = ''
    Vimium's own parser takes the settings an account writes it, each of the key language's keys
    doing the language's act beside Vimium's own, rather than refusing a line or leaving one to
    nothing, and its own exclusion rules leave it off on GitHub and on each Google page with keys
    of its own, and on everywhere else
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
  claude-statusline-says-how-full-the-context-is = ''
    Claude Code's status line says the context its transcript holds, leaves it out before one is
    written, and names the colours it can't read rather than printing a line
  '';
  claude-wears-the-theme = ''
    Claude Code's theme has the account's text colour and only hex colours, and the mix its tints
    come from is exact
  '';
  a-chord-reads-only-its-own-program = ''
    launcher-say, which niri runs on each of the launcher's chords, is linked statically and asks
    the loader for no library, so a chord reads one file, the one held in memory
  '';
  a-chrome-tab-is-found-by-its-site-and-goes-to-its-window = ''
    Chrome's open pages the programs daemon serves are the launcher's Tabs collection, each named
    by its title and found by its site, ranked as the daemon answers what's typed and matched in
    the shell only where it can't search, and a page's Return goes to the Chrome window it's in
    front of, else Chrome's newest
  '';
  clipboard-history-passes-clippy = ''
    clippy finds nothing in the clipboard history or its tests
  '';
  launcher-say-passes-clippy = ''
    clippy finds nothing in launcher-say, which says a chord to the launcher, or its tests
  '';
  shell-weighs-passes-clippy = ''
    clippy finds nothing in shell-weighs, which says in the journal what the shell weighs, part
    by part, or its tests
  '';
  clock-follows-is-on-the-path = ''
    clock-follows, which says where the clock last moved when typed bare, is on the path of a
    graphical account, where its service moves the clock's zone to follow the laptop
  '';
  clock-follows-passes-clippy = ''
    clippy finds nothing in clock-follows, which moves the clock's zone to where the laptop is,
    or its tests
  '';
  code-health-counts-what-got-messier = ''
    code-health counts a repository's long files and crowded folders without their reasons, its
    suppressions in each language's own way, its lines repeated from elsewhere and its tests in
    quarantine, the lint rules its configs turn off, its files nothing reaches, its folders with
    no note, its VM test lines, and its Terraform not as tofu fmt writes it, what tflint finds
    held to all its own rules and Google's, what trivy finds insecure, what tofu validate refuses or warns of, what has no description or README and
    what no tofu test tests, from a ref's files, and writes the day's figures as cadence reads
    them, and the hour's against each target, the hour before and the owner
  '';
  code-health-crowds-as-core-does = ''
    code-health and core's rule that every crowded folder says why name the same planted folders
    crowded, and the ones expected: a reason past a note's opening, wrapped, or in AGENTS.md
    is read, one joined to a word isn't, and a folder at the limit, past it, nested or of
    folders is counted alike
  '';
  code-health-finds-a-machines-programs-first = ''
    code-health finds a machine's own programs first on its PATH, such as a git that reads a
    repository stock git refuses, with core's git and coreutils after them
  '';
  code-health-passes-clippy = ''
    clippy finds nothing in code-health or its tests
  '';
  code-health-reads-each-folder-it-is-given = ''
    code-health is handed every folder of a larger repository that a repository is given
  '';
  code-health-reads-hourly-against-targets = ''
    code-health reads each hour, handed each repository's targets, its owner and each measure's
    own owner
  '';
  code-site-holds = ''
    the code page typechecks on the kit at its strictest and builds, its tests passing: each
    repository's trees by season then branch, a cursor stepping within the rows, an address
    landing on a place, a repository or a tree in its season, and another site home to a kind found
    by the row claiming it, inside the sites' folders, its homes of that kind alone, and a
    store's weight said in disk units, about only where the daemon reckoned it, with its upkeep,
    the cells at work tying two repositories each tie once, laid out as rows and threads for the
    map, and every name, branch, change title, address and failed check drawn as words, never
    markup
  '';
  code-site-reads-live = ''
    the code page, served by the local sites' server beside a stand-in repos daemon and node, reads
    live in headless Chromium: every repository drawn, one growing once a tree is written and one
    going, a cell at work drawn and linked to its home on the map, broken while the daemon is away
    and live with its repositories again once it's back, stale once the server goes, the web-design
    skill's page checks and its homes check passing at a phone's width and a desk's in both looks, a
    repository's weight and due upkeep said in its row and drawer, its ages told again each minute
    in place with the focused tree kept, today's growth yesterday's once the day turns, no row
    moving as the cursor steps, a reload not shifting as the feeds arrive, a second's burst of
    records drawing the list a few times, a first visit within its budget of files, bytes and
    elements, both visits reaching only the sites' server and the second answered unchanged, the
    map at the page's foot drawing a thread for each cell joining two repositories, walked by its
    keys, opening one as the list does, and handing the keys to the list once its last tie goes,
    and nothing logged
  '';
  collector-keeps-what-it-is-sent = ''
    the collector reads a span it was sent back at its moment and in its trace, and keeps what a
    slice holds in memory and swap, and the pressure on it, as spans and moments
  '';
  collector-passes-clippy = ''
    clippy finds nothing in the collector, which keeps the traces programs send it, or its tests
  '';
  collector-stays-in-the-background = ''
    the collector runs in background.slice, restarts when it fails, and has a runtime folder for
    its socket
  '';
  comments-name-what-exists = ''
    every comment naming a path from a top folder names what's there, as code-health's stale-paths
    measure reads them, but those still waiting on their owners
  '';
  commands-agree = ''
    every command core's crates build keeps docs/commands.md's convention: built on the terminal
    kit, ending and inking only through it, and answering --help --format json as help/1 and
    --version in one line without starting anything; every script a Nix wrapper builds
    exits only with the table's codes, spells no colour and answers --help with its name; and
    each command given it sets its help at 60, 80, 100 and 140 columns with no line past the
    edge, no wrap losing its inset or parting a figure from its unit, and none wider than at 80,
    and sets each view it's given, such as disk-guard's bare run and status from a nearly full
    disk's stand-in, at those widths, exiting 0 with no line past the edge and no wrap losing
    its inset or parting a figure; and each subcommand a command's help lists, at every depth,
    answers --help, -h and help before it with help of its own, not its parent's, and a word
    no subcommand answers to exits 64; and no command's source pads a column to a width by
    hand rather than through the kit's tables; but the points each still waits on
  '';
  console-font-passes-clippy = ''
    clippy finds nothing in the console's font tool or its tests
  '';
  copies-hide-while-the-screen-is-shared = ''
    Copies hide what each copy holds while niri says any cast, a share through the portal or a
    recorder's screencopy, and until niri has said its casts, but not for the shell's own
    recording of one window; what a hidden copy's row is made of (Shared.shown) keeps only its
    kind and when it came, never a glyph or side read off what was copied; and a row made
    showing names the very source that hides it (Shared.made), so Steady.hiddenGone lets a way
    back to it go once that source hides, while one made hidden names none and blanks what its
    row would draw of the copy
  '';
  crate-reach-finds-what-reads-what = ''
    what main moved in a crate a change's crate reads, or one reading it, however far, is
    beside the change, another file of its own crate is too, and crates that only read the same
    third one stay apart
  '';
  crate-reach-passes-clippy = ''
    clippy finds nothing in crate-reach, which merge-net and a gate's replay read to judge what
    main moved beside a change
  '';
  daemon-client-passes-clippy = ''
    clippy finds nothing in daemon-client, the daemon and client the daemons' tests share
  '';
  daemon-client-waits-for-its-daemon-to-listen = ''
    a daemon's tests ask its socket again while it's on the disk but not yet listened on, or gone
    while it's made again, rather than failing at once
  '';
  daemon-serve-passes-clippy = ''
    clippy finds nothing in daemon-serve, the serving side the daemons share
  '';
  daemon-serve-tells-each-watch-what-moved = ''
    a daemon's watch is answered whole, then told only what moved in what it watches, by kind,
    key or place; a paced watch is told all that moved in one line once its pace is up, and
    asked again at once it's told at once what it held back, never everything whole; a
    connection that stops reading is hung up on once its queue fills, while the rest are
    still told; one that asks without reading, slowly or not, or whose pace finds its queue
    full, is shut and its writer ended, as is one whose ask runs past the longest a line may
    hold; each hang-up is said once on standard error, naming
    the connection, its process and how it was being told, while one whose writer ended goes
    unsaid, and lines past what waits to be written are counted rather than waited on; and the
    daemon answers its own asks
  '';
  daemons-agree = ''
    every daemon a graphical machine's account runs keeps docs/daemons.md's convention: its
    socket held by systemd at %t/daemons/<name>.sock, one name for its units and program that ps
    shows whole and first, and daemon-serve answering, but the points each still waits on
  '';
  darwin-role-twice-is-once = ''
    importing a nix-darwin role twice leaves the machine unchanged
  '';
  dates-read-as-before = ''
    in the shell's own Qt, Date.parse reads ECMA's format, Qt's text, ISO and RFC 2822 forms and
    its others as before, gives NaN for strings none reads, and refuses five hundred empty ones
    within 100 ms
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
    fixed, in the journal only when it's run under one, and answers what's wrong now as text and
    as JSON
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
  dictation-tidies-a-take = ''
    the tidying model on the CPU takes a take's filler words and false start out and punctuates
    it, and leaves a take with a negation and a number as it was
  '';
  dictation-tidies-from-data-alone = ''
    the tidying model is IRs and configs alone, fetched by hash, with no code and no chat
    template of the export's
  '';
  dictation-works-it-out = ''
    dictation's spectrogram, transform and vocabulary match their references, words turn into
    Whisper's tokens as GPT-2's merges join them; a held chord's release stops only what it
    started, and a tap's none; a tidy is typed only when it keeps the take's words in order,
    every negation and number among them, and only while the take still waits for it
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
  disk-guard-nix-collects-beside-leaving-clients = ''
    the machine disk guard's Nix collects garbage while programs hand it roots and leave at once,
    where an unpatched Nix 2.34 or 2.35 ends the collector
  '';
  disk-guard-passes-clippy = ''
    clippy finds nothing in the disk guard or its scenarios
  '';
  disk-guard-stays-in-the-background = ''
    the disk guard yields to the account, and a command overrunning its memory never stops it
  '';
  disk-throttles-come-back-as-they-were = ''
    on a stand-in sysfs, writeback-goal refuses a wrong call with 64 before it looks for the
    disk, fails on a name that reaches no throttle, gives back a disk that's gone quietly, and
    gives back what the throttle had before its first goal, or the kernel's default where none
    was recorded, recording a throttle that can't be read as off, failing a goal the throttle
    won't take with only the record that call made gone, and failing a giving back it won't
    take with the record kept; disk-order stands the throttle down beside io.cost and gives
    the kernel's default back as it hands the disk back
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
  each-acts-key-is-free-where-its-pressed = ''
    each act's own key is one letter normal mode doesn't already take, and no row gives one key to
    two of its acts
  '';
  each-list-keeps-its-kinds-order = ''
    each kind of list the launcher shows keeps one order with nothing typed, A to Z only its last
    tie and numbers read by their value, a choice never moving when chosen, and each place that
    lists things sorts by its kind's order with no A to Z made by hand, through Order.sorted, which
    keeps rows alike in the order they came where Qt's own sort shuffles them
  '';
  each-live-page-opens-with-the-strip = ''
    every live page opens with the strip localSite writes, once, straight after its skip link,
    linking home one folder up and saying only that it's an app, and reads the theme from the
    sites' one copy with no kit of its own; and a page carrying a strip of its own, reading no
    theme from that copy, or with its styles reading a kit of their own, fails its build, saying
    which page; and so does a sheet esbuild refuses, warns on or crashes on, or a sheet or a
    script keeping a comment, saying which; and the sheets a site, the kit and the theme build
    reach a page without their comments, and a site's scripts without one marked to stay
  '';
  heads-are-the-kits = ''
    every live page off the waiting list opens with the kit's head as the web-design skill's
    sites.md writes it, read as a browser parses it, raw text and SVG left open included, with
    what's hidden, shut, a popover never shown or styled out by its own style's display,
    visibility or an opacity of nothing, as a browser keeps each declaration, left undrawn and
    what's inert unheard: its one banner, holding none,
    by tag or role, inside no other and in no section by tag or role, its title the first
    heading of any level, each part drawn and heard, an eyebrow of two phrases drawn and heard
    split by the kit's one dot, the title, a lede, the standing line and the quiet row in that
    order and nothing else, only the standing line's polite status live as ARIA reads it and
    nothing around the head live, the quiet row opening with its key line, and the kit's
    head.css linked by its own path in the page's head as an untitled sheet every screen
    applies, with no base anywhere and each skip link, drawn or not, on a line of its own
    before the head; a page still waiting
    passes until it
    keeps to the head, then fails until it's taken off the list, which no page joins; every
    page core's own modules mount on the realm workstation the checks build is judged, a mount
    not built with localSite from a path in the tree, or naming no page but the shared kit,
    failing for the reason each of its rules, run over localSite itself,
    is seen to; Home is the page the home is
    built from; and the gate passes sites.md's own head and names each way a page strays from it
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
  each-places-words-are-a-file-of-their-own = ''
    every thing's words sit in the file of the place it's in, under launcher/words/, and the
    frame holds none, so two changes to different places' words never touch the same file
  '';
  eero-completes-in-zsh = ''
    eero completes in zsh down its tree as help lists it, with no old spelling offered: each node,
    each device by its name or, where another shares it, its address, each profile, what a node or
    profile holds after its name, an act's own words, the old spellings as they were, and --json
    after a view
  '';
  eero-passes-clippy = ''
    clippy finds nothing in eero or its tests
  '';
  eero-reads-what-eero-says = ''
    eero draws the home network from every answer in its contract and a short history exactly as
    home-network.json holds it, its times in UTC and a read not yet made as unknown, and tells eero
    failing, refusing and a session aged past refreshing apart; its daemon reads the network every
    30 seconds while anyone looks and every 5 minutes otherwise and the rest at their own paces,
    keeps the last figures when eero can't be reached, saying since when and why, and answers each
    watch on its socket with the model and tells it every change, a watch ahead being a look; it
    keeps 30 days of what moves, a sample at most every 25 seconds, and a coming, going or roam
    only between reads 10 minutes apart at most, and draws the last hour's series into a model a
    busy hour keeps under 350 kB; it times its own look at the internet, the gateway node only
    from home, and an outage from the first look out to the first back; it tells, as notices, the
    internet out and back, a node offline and back online, a device new to the home network,
    firmware waiting and changing, and an event no other notice says, each once and none before
    eero first answers or on its first answer, and keeps what each said, drawing 30 days of it
    into the model, at most 50, a history from before keeping its rows; it sends the
    conferencing setting as a bare boolean, counted taken only once eero reads it back; it keeps
    or frees a device's address only as asked on its socket, refusing to keep one another device
    or eero holds or one outside the range, or to free one a port forward may reach, judging each
    by eero's list read back, logging each ask whole with who asked before it's done and after,
    and doing none whose ask can't be logged; its socket answers its own account alone, and the
    session is sealed 600 in a folder of 700; every
    view's page matches its golden one, saying when eero isn't answering or the reading is old,
    colouring only on a terminal, each view's JSON is the record's own slice, and a watch draws
    its view again from each model the socket tells it
  '';
  efficient-cores-hold-only-theirs = ''
    efficient-cores, on a stand-in sysfs, holds only the efficient cores' clock ceiling, each
    within its own lowest and highest clock, frees each to its full clock, refuses words it doesn't
    take, and holds nothing on a processor with no efficient cores
  '';
  elsewhere-compiles-for-macos = ''
    elsewhere and its tests compile for macOS, clean, read from Linux
  '';
  elsewhere-passes-clippy = ''
    clippy finds nothing in elsewhere or its tests, on a Mac as on Linux
  '';
  elsewhere-reads-what-a-machine-declares = ''
    the machines and kinds of job a machine repository declares reach elsewhere's settings in the
    words it reads, an account declaring none has it on its path and sends nothing, the ledgers it
    reads are where mycelium keeps the realm's, a realm apart in a folder of its name, and a
    scratch that isn't one hidden folder's name fails the build
  '';
  elsewhere-goes-where-mycelium-places-it = ''
    elsewhere asks mycelium to place a job with its kind's needs and how long it lasts, preferring
    each machine it may send to that already holds some of the job's quest, and when
    mycelium places it on a machine it may send to, sends only its tracked files, never a hidden
    file, key or excluded path, those under a folder a share keeps home included, through the
    quest's handover and runs it by mycelium run, never ssh or rsync, started in the quest's
    scratch and copied into a hidden folder within it, as a far strict fence allows, after its
    kind's setup; it brings back what the kind names, landing each as a plain file within the
    job's folder and never through a link out of it, and ends with the far job's own code, or
    says why what came back fell short, ending 77 when another try meets it again and 75 when it
    may not, the far side's own refusal told from mycelium's; a job placed here, with no quest or
    one mycelium doesn't run far runs here saying why, or under --there ends saying why; a
    program its kind doesn't allow is refused before anything runs, and a command asked wrongly
    says what it takes
  '';
  emoji-keeps-a-lost-search-only-open = ''
    with nothing at the clipboard history's socket, Emoji's rows are drawn only while the launcher
    is open: shut, a search and one typed inside a group that went unanswered are let go; open,
    each is kept, and asked again at the history's hello, the answer to what was typed in the
    group not taken as its key's own a while after the key, so Emoji's rows stay where they are;
    and an emoji's row is keyed by its thing, kept under row: where that claims the launcher's own
    kind, and by its key where it has none
  '';
  ergodox-on-every-workstation = ''
    every workstation can flash the ErgoDox, a Mac as much as Linux
  '';
  evaluates-refuses-a-machine-that-fails = ''
    core.lib.evaluates refuses a bare machine whose assertion fails, as it evaluates, and passes
    the same machine with its assertion holding without building it
  '';
  evaluates-refuses-a-machine-that-warns = ''
    core.lib.evaluates fails on a bare machine with three warnings, one naming a package that
    can't build, and its log names the check and each warning, building nothing a warning names
  '';
  every-assert-says-what-was-wrong = ''
    every Rust assert! and every Python assert of anything but a comparison carries a message
    saying what was expected, but those still waiting for theirs
  '';
  every-card-is-the-one-card = ''
    the launcher and the arrival line's notice take their frame from the one card and stand their
    ink on its content, never drawing a frame, reading the inset or tinting a row's ground
    themselves
  '';
  every-check-says-what-it-proves = ''
    every check says here what it proves, and every description here names a check
  '';
  docs-name-what-exists = ''
    every doc's links and paths name what's there, but the exemptions named
  '';
  du-leaves-guarded-folders-alone = ''
    a Mac's account runs its own du, ahead of any other in its profile, macOS's with the ignore
    file, the data volume and the account's home, and a Linux account none; over a stand-in du,
    a count starting above a guarded folder, from the home folder, Library, the data volume's
    name for it, /, even from inside one, a home in capitals or HOME's own, leaves each
    guarded folder out by its name, once, says so in full and ends 1, HOME gone, unset or not,
    options and their values, joined or apart, read past; a start that is, lies in or passes
    through a guarded folder, named, by a glob's list, in capitals, with ./, // or /., through
    .., by a link, a file through a link, by the data volume's name, or through a home named by
    one, is judged before it's opened, left out and said, du not run when none is left, and a
    start after it like an option reaches du as a path; a link -H, -L or a / after it
    follows is judged by where its links lead, each read as written from the folder really
    holding it, left out when one leads into Documents and left out and said apart past 32
    links or through a folder that can't be entered or isn't there, as is a start typed below
    one, while a start alone missing from a folder that is there, a / after it or not, reaches
    du; a start named - is that folder and CDPATH moves none; only a home's own guarded folders
    are left out by name; -L is taken out and
    said unless -H or -P follows; du's own nonzero end is kept; a working folder gone still
    counts a whole path, and no home found is said; a count above none runs as given and ends
    0, and --help runs no du
  '';
  every-fixed-wait-says-why = ''
    every sleep in a test polls, or says what passes while it waits in a comment or by a named
    constant, but those still waiting to become waits on their events
  '';
  every-folder-says-what-it-holds = ''
    every folder says what goes in it, but those still waiting for their note
  '';
  every-lock-agrees-with-its-manifests = ''
    every Cargo.lock names each dependency its crates' manifests take, and none they dropped, as
    cargo locks them
  '';
  every-ruff-exception-hides-a-finding = ''
    every per-file exception in ruff.toml hides a finding of its rule that ruff would raise without
    it
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
  every-module-meets-the-others = ''
    every home, NixOS and nix-darwin module core offers is brought into an example machine by a
    role, a hardware model, the example or another module, so two that clash fail an example
  '';
  every-piece-in-now-has-a-way-in = ''
    every piece that can show in Now has a way into more, an action Return runs, a drawer
    or a place it opens, but those excused by name with a reason, and one with none fails
  '';
  every-network-row-has-a-drawer = ''
    every row the launcher's Address and Traffic show, each answer as a row too, has a drawer
    saying more than its line, each fact named by the vocabulary and each eyebrow saying where
    and what: a connection's packet size, its link and its hardware, a rate's range over its
    reads and what it carried since startup; Ethernet with no card holds its place, not plugged
    in; Wi-Fi's own row leads with the network joined, its signal and how long; and a
    connection's drawer shows whole or not at all, its facts made unasked and made again in
    place as it changes, that making felt as "networks glancing" so an opening's stall names it
  '';
  address-waits-on-no-route-file = ''
    Address answers from the routing tables as last loaded, reading no file as it answers, even
    while a load is under way; loads them again in the background as the launcher opens,
    telling the launcher once that load changes the way out, and nothing for a load that
    changes only the kernel's use counts; and loads them again as the links change
  '';
  ethernet-waits-on-no-cable-file = ''
    Ethernet answers from its cable's files, its errors, the name servers and the time since
    boot as last loaded, though each has changed since, the uptime counted on by the clock;
    before a cable's files land it says the link as the daemon told it, with none of the
    cable's rows, says it's still reading, and makes no drawer for it, and a lease says its
    server until the uptime lands; asked by a terminal after a gap it reads on until they land
    again; the launcher opening loads them again and is told of an uptime stepped, though not of
    one that agrees, and open is told of one ahead of the count, and of a cable loaded changed
    though none is shown; a cable shown
    loads again each second; a cable the daemon says has changed as it loads is said as it says,
    the load begun before not kept; a card whose load is given up is said as the daemon tells
    it, drawer and all, leaves nothing reading, and isn't loaded again at once, though waiting
    its turn, but a minute on, then twice as long if held up again, to an hour, held though its
    link changes or it's plugged in again, asked again once the clock steps back, and held up
    no more once loaded; a fact is asked on the watch's tick, isn't asked again as it loads,
    keeping its ask's tick, and is given up on the eighth of the watch's ticks after it, about
    two seconds, not the seventh, alone beside facts just asked; one given up leaves nothing
    reading and is held off as a card is, a minute, then twice as long, kept though it lands
    late, and let go once asked again it lands in time; the watch keeps a fact that never lands
    a second and a half, then gives it up; a name server changed makes the drawer again shut
    and tells the launcher open; a card asked alone, and the facts asked alone, are still
    reading; a card given up keeps the view still reading its file, isn't asked again while
    that read hangs, and once it lands late destroys the view, keeps its hold and credits no
    other card with it; a cable file asked is watched and asked once, its watch stopped as it
    lands, and given up in flight by the watch about two seconds on when it never does, its
    view kept and a new one made; each cable file is named once; a card whose hold is over is
    asked for by the answer itself; a card gone is forgotten, though its load
    was under way; a card named __proto__ has its own lease and neighbours; and one named
    constructor no gateway
  '';
  traffic-reads-a-known-interface-once-after-a-gap = ''
    Traffic's answer says the counters, the shaper's status and the uptime as last loaded,
    though each file has changed since, a first rate over the uptime loaded; the shaper's line
    resting is said as measured, holding as held to the shaper's limit, and a state it doesn't
    know as measured, whatever it's called;
    asked after a gap with the launcher closed, it says its figures are from when last read and
    loads them again, reading on until a rate of now has landed, or until the counters fail to
    load, after which it reads on no more; a load stamped ahead of now, as a clock stepped back
    leaves it, is old, and the read after it makes no rate over it; a load asked just after one
    landed loads nothing; in the background, it loads one sysfs file for an interface it
    already knows, its uevent, gap after gap, and none on a load straight after; leaves out an
    interface first seen until it has loaded it whole; and loads all four again, keeping the
    kind it had until they land, then taking the kind they say and telling the answer shown,
    for one made again under its name or whose uevent is empty or gone; a sysfs file that
    never lands is given up about two seconds on, its view kept and another made, and its
    interface isn't asked again until that read lands, which isn't taken for the file under
    way and lets the view go; two hung at once are each kept by name, and the one landed
    asked again; an interface given up is held off a minute, then twice as long each time it's
    given up again, even once its late read lands, until its files land in time; given up as
    its read begins, the next interface is asked through a new view, and the read lands late
    with its text through Traffic's own, letting its view go, as one that fails does; no file
    of the interface under way is asked while its last is in flight; given up with no read in
    flight, no view is kept, but its interface is held off; a hold goes only once every file
    lands, so one whose uevent landed, given up before the rest land, is held twice as long;
    an interface held isn't asked as a load wants it; and interfaces named constructor and
    __proto__ are each read whole, known and described as any other, and one given up in
    flight is held and kept hung by its name, not asked while its read hangs, though its
    hold is over, and asked again once that read lands
  '';
  every-place-is-on-where-its-source-is = ''
    on a machine holding every module and all each reads, every launcher place is on, so a
    place turned on by a unit, option or package that was renamed is caught
  '';
  the-places-on-file-names-each-place-on = ''
    the launcher's places-on.json, which cadence reads its journal by, names each place on, by
    the name it's turned on under, sorted, and no other
  '';
  every-program-wears-its-own-mark = ''
    every program the launcher lists wears a mark of its own, its logo or the theme's mark for it
  '';
  every-pattern-names-a-file = ''
    every path pattern merge-net's must-run.txt, ruff.toml's per-file ignores, treefmt's unread
    kinds past treefmt-nix's own and the ripgrep exclusions of tests/repository's scripts and checks
    give matches a file the repository holds
  '';
  every-why-in-the-foot-has-words = ''
    every why the shell's sessions say a press they can't answer by, and every key the agents'
    list names to its foot through its elsewhere words, has words in the machine vocabulary, so
    none of those presses goes unsaid in the foot, and each why the shell gives has its colour
    chosen, plain or alert, and none other has, the agents' list colouring the foot by it
  '';
  every-window-says-what-it-holds = ''
    the reader on a page and a kitty running the account's shell, opened on the account's niri,
    are each titled by more than their app id or the name of a program running in them
  '';
  every-pixel-mark-stands-level = ''
    every mark the theme draws in pixels, the three empty places' marks and the margin's cat and
    puppy in each pose, is square, its ink touching its top and bottom rows and centred across to
    within a cell, the cat's yarn standing on the sitting cat's ground line in its square instead,
    and the three empty places' marks, the cat's and the puppy's poses it names and the yarn are
    drawn
  '';
  every-cat-pose-reads-as-one-cat = ''
    every pose of the margin's cat, each mark drawn as its sitting's glyph or a stride's or
    meant as its own, has no lit pixel standing alone, a diagonal touching, and lit pixels
    within 15% of sitting's, each stride of the walk the cat's code takes means what its poses
    are picked by and steps one pair of feet with its body still, each pair in its turn over
    the walk, and a leap's rise, tuck and fall are all drawn
  '';
  every-pose-fault-is-refused = ''
    the margin's animals' pose rules refuse the cat's own strides out of order, alone, with a
    pair never stepping or stepping twice running, across the walk's end too, after a pose
    that isn't a stride, undrawn, and none at all, each by its reason; a stride planted with
    its body moved on its top row, the row below, its middle or above its feet, though not one
    with only its tail's tip moved; a foot moved either side of the feet's middle, and on an
    odd width in its middle and last column; a pose with a pixel two cells off, though not one
    joined by a corner alone; and just over 15% fewer or more pixels, on sittings of 37 and 20,
    though not just under or on the line, and on one of 13; and they say a misnamed sitting
    undrawn, and a pose listed but missing, hold a stride no pose lists to the stray and size
    rules, and refuse a stride meant otherwise than its animal's poses are picked by, and
    strides all meant alike but not as asked, though not the same strides all meant so nor
    a sitting meant otherwise, and say an undrawn one undrawn rather than meant otherwise; all
    of it planted on copies of the cat's own drawings; and they find an animal's poses by its
    sitting's and strides' glyphs, the start of its name and what a mark means, though one of
    them is undrawn, and no other mark, not one holding the name inside its own, one meant
    otherwise, nor one drawn as no glyph beside a sitting drawn so
  '';
  every-puppy-pose-reads-as-one-puppy = ''
    every pose of the margin's puppy, each mark named or meant for it or drawn as its sitting's
    glyph or a stride's, has no lit pixel standing alone, a diagonal touching, and lit pixels
    within 15% of its own sitting's, each of its strides means what its poses are picked by
    and steps one pair of feet with its body still, each pair in its turn over the walk, and
    its sitting, bow and strides are drawn; its
    strides are the walk the cat's code takes, each named for the puppy
  '';
  every-script-finds-what-it-imports = ''
    every folder's Python that nix fmt hands mypy passes it when handed to it alone, as a script
    that changed alone is, after failing a planted script that imports from another folder
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
  every-timer-runs-a-written-service = ''
    home.nix's assertions refuse, by name, a timer whose service the account doesn't write and a
    written service named in jitsusama.timedRuns.unwritten; one unwritten is left out, so a
    switch's setting is never a unit of its own
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
    a laptop on the XPS 14, its disk and Secure Boot, with Chrome beside Google's pages, evaluates
  '';
  example-laptop-initrd = ''
    the example laptop's initrd evaluates, its derivation written down so a change that moves it
    runs the VM tests that boot through it first
  '';
  example-nixos = ''
    a NixOS workstation with a screen evaluates
  '';
  expect-run-fails-each-case-that-throws = ''
    a QML test's Expect.run exits 0 when its cases hold, and 1 with a FAIL line naming what was
    thrown when one doesn't, whether an Error, a bare word or null, never hanging
  '';
  files-are-chosen-in-the-launcher-alone = ''
    open-with chooses a file in the shell when it answers, and chooses none when the shell can't be
    asked, never asking GTK's portal
  '';
  find-leaves-guarded-folders-alone = ''
    a Mac's account has its own find, running macOS's with the ignore file and the data volume, and
    with GNU find standing in for macOS's, a search from the home folder, from Library or from above
    it and the data volume's name for it never opens the folders ripgrep's ignore file guards, yet
    lists them where they match, once, as the expression's own action prints, says it left them
    unsearched and finds the rest, a project's folder named like a guarded one included, keeping
    -empty, -exec and -execdir, which would open them, off them and saying it left them out,
    reading a primary's operand as no primary, reading the file's last line unended, searching a
    guarded folder named, started in or named beside its parent; one a -maxdepth keeps find out of
    runs as given, with no note, as does a search find can't prune, -delete's among them; every
    line of the ignore file names a folder find can guard; Linux's account gets no such find
  '';
  firmware-errors-passes-clippy = ''
    clippy finds nothing in firmware-errors or its tests
  '';
  firmware-errors-says-why = ''
    firmware-errors reads the health table's reset source, reason and Intel's reset cause past
    other records and other vendors' entries, weighing the cause on Panther Lake alone, and
    gives nothing, never a panic, for a table cut short or lying about its lengths, saying such
    a table and a missing watchdog as kept naming none, and one it can't read as unreadable,
    never as lost, in its words and its JSON; takes no id shorter than four digits or opening
    with a hyphen, as journalctl's index -1 does; names a
    core's timeout from Intel's own sample Crash Log, padded or not, and says an empty region
    holds no error, a cut one is cut, a Crash Log that won't decode is unreadable and one of
    another kind holds no Crash Log; lets heat, lost power and the power button speak before
    the rest of Intel's cause, whether the reason or the cause's bits say them, never lets the
    cause or its bits mark a reboot the reason or software asked for, and lets a kept error
    speak over an asked one; lists the boots kept beside a table by when they began, else when
    their tables were first written, saying a table or a folder it can't read rather than
    leaving it out or ending the list, and a decoder's panic as a region it couldn't read; runs
    the keeping over stand-ins, which keeps nothing without a table, keeps each for its group
    alone, never rewrites one kept whole, writes a short one again whole or not at all, follows
    the clock's start on a later run, and fails on a table it can't copy once the rest are kept;
    answers JSON of facts and names with every key in every record; and finds a boot by any way
    its id is written, refusing what isn't an id
  '';
  firmware-installs-once-for-each-sure = ''
    Firmware's step asking if Joel's sure asks the machine daemon to install on that device once,
    its row drawn at once as installing, and neither that row nor the step the daemon then serves
    offers a second Return anything to run, so pressing it again only closes the launcher; and
    Firmware answers again once its watch is answered whole, so rows the daemon already held show
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
  for-you-stands-each-under-its-cell = ''
    For You stands what needs Joel now under the cell behind each, the cells by their heaviest
    and what a cell asks leading it, heads each in the words Agents says it in, finds a
    session's own notices by the cell they name, joining only the unread that ask something and
    haven't expired or been withdrawn,
    kept as one list till that moves, and sets Agents above every heading
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
    history, newest first, leaving out any it can't read, offers a look's choices, answered by a
    steer to the session that left it or its quest's and then in History, and gathers what other
    sources give it, as Passwords' one to-do, into what waits, weighed beside the rest, and what's
    done
  '';
  formatting = ''
    everything is formatted and passes the linters
  '';
  gcloud-broker-asks-as-the-named-account = ''
    gcloud-broker is installed only where a machine names a service account, and asks only as it
  '';
  gcloud-broker-mints-and-keeps-its-tokens = ''
    gcloud-broker, against a stand-in for gcloud, prints a token minted as the account alone,
    keeps it where only the account reads it, mints one per audience, and says a refusal
  '';
  gcloud-broker-passes-clippy = ''
    clippy finds nothing in gcloud-broker or its tests
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
  github-broker-mints-a-narrow-token = ''
    github-broker, against a stand-in for GitHub, signs a narrow token with the App's key, asks
    once while it lasts, says why GitHub refused, and answers before the App is made
  '';
  github-broker-passes-clippy = ''
    clippy finds nothing in github-broker or its tests
  '';
  go-to-brings-a-windowless-session-forward = ''
    Go To on a session with no window here, one pi's host runs or one on another machine, opens
    its window away from Joel's workspace and focuses it once niri lists it, and only once
  '';
  google-answers-on-systemds-socket = ''
    google serves the daemon interface on %t/daemons/google.sock, a socket systemd holds, both
    starting at login without waiting on a file, and reads pi's sign-in from the keyring
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
    can't be reached, tells the shell each meeting's times, answer, the one link it's joined by,
    how many are on it besides Joel, each address once, and how its series repeats, asking each series only every five minutes and keeping its rule while
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
    asks are, on the socket systemd hands it; and Calendar's, Documents' and Mail's rows and
    drawers, read in the account's own vocabulary each apart, so one lacking a word serves none
    and says so while the others serve on, say no word but the vocabulary's, a meeting's, a file's
    or a mail's own and the clock's, sizes to three figures, and a documents watch hears its rows
    and a drawer watch only the drawer a change touched, each place's rows spaced 1024 apart, one
    row moving told alone, midway, as the rest keep their order; and google-people watches for
    people the moment google's socket is made, past the wait it would otherwise back off to
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
  health-gives-back-the-pages-it-reads = ''
    health's unit holds a high mark of 256 MiB, past which the kernel takes back the pages its
    start's replay reads, and no hard limit, whose kill would lose what health holds unwritten
  '';
  health-passes-clippy = ''
    clippy finds nothing in health or its tests
  '';
  health-tallies-the-days-warnings = ''
    health reads each journal line for the thing that said it and whose a crash was, reads a Mac's
    unified log line alike, a fault as critical and an app's program as a program, counts a
    line's repeats as one signature, counts a device dropped, reset or correcting its errors though
    the kernel says it only as information, times a boot to a ready launcher without the password's
    wait and a shutdown to its boot's last entry, counts once a lid reading shut while the laptop's
    screen is on and its keys typed, keeps the pain-points detector's ranking and says a pi go by
    where its time went and a Chrome go by its reloads or way back, counts a tool call
    that failed, was refused or was made twice running and Joel's words that sound pained, reading
    each session's log on from where it stopped and never twice, keeps what pi and the shell hold
    by their copies' ages and the day's most through a restart, counts an agent whose presence says
    busy or listening though its process is gone and keeps the day's deaths after their presences
    go, counts the asks for a secret shown, said yes to unread and asked again though said only as
    information, keeps each disk's least free space and gravest band through a restart as the disk
    guards say them and counts once a guard that stopped beating, each other machine whose calls
    go unheard, those past the most named as one, and a mycelium node here that won't answer while
    it stays so, only a machine heard on a call or none mending, keeps a day by kind
    and writes it with the figures cadence holds it to every half minute however busy the
    journals, and shows the three worst pains in plain words, pi's as pi's, then a line a kind,
    the gravest first; it times a launcher ready by today's line, each
    launcher part and pi's modelled typing against their budgets, the keys' p99 exactly from every
    key the launcher's trials kept once each though the shell writes them again, saying how many
    it's over, counts the searches typed and those left with nothing picked without what was
    sought, keeps each
    program's openings and ends as the windows daemon timed them and each daemon's ready as it said it as ours, holds
    pi's start to its own budget apart from the daemons', times a service's restart from its
    stopping to its ready line, the shell's ready or else its start, leaving a start alone, a stop
    long before and a ready long after, reads a service's stopping and start from systemd and not a
    scope's, weighs each group's processor time by the hour from its cgroup, counting none from
    before its first look or a restart of it, names pi's sessions and their commands for the
    quest a presence or their slice says and the rest for their slice or service, finds the
    account's manager above its own service, tells Joel's crashes from ours, counts no refused
    `sudo -n true` probe as an error, and says each figure's side; and on systemd's own
    journalctl, over a journal journal-remote wrote, each follower reads its line of two boots
    and follows on into the second, and a day whose clock was stepped back is replayed in the
    order it was written, each history ending at the line it's given as the journal's last, and
    the journal's last line is the one written last, by journald's number
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
  home-answers-its-keys = ''
    the local sites' home, built from three recorded sites and served with the kit and the theme's
    web kit the sites share, answers its keys in headless Chromium: j, k and the arrows step its
    cursor and hold at the ends, g twice but not once and G reach the first and the last, Return
    opens the site under the cursor, ? and F1 show its keys and Escape hides them, its shown keys
    answer a tap, an arrow on the theme switch chooses a look and leaves the cursor, the look holds
    to the next visit, its mark takes each look's own tint from the kit, a document under a hero
    answers a tap on its keys, and the web-design skill's page checks pass at a phone's width and a
    desk's in both looks, with nothing logged
  '';
  home-builds-from-its-rows = ''
    the local sites' home lists each site from its row in sites.json, the settled apart and every
    name escaped, with a row Nix declares for a live site standing in for the record's at its path
    or going first, says every row it lists in rows.json as written, but for a recorded row's claim
    on a kind a declared row is home to, which it leaves out and says, in its log and in rows.json
    with the declared row that has the kind, strips each page that asks after its skip link, names
    each document loading a script file of its own rather than the kit's, loads the kit's keys and
    switch and the theme and its fonts from the sites' shared copy, takes away the scripts it used
    to copy, draws its mark in each look's tint or the other look's where one is missing, takes its
    grounds and tints from the theme it's handed rather than a kit/ among the documents, says a
    theme that isn't there and fails on one that isn't a stylesheet, leaves out and says a tint or
    ground that isn't a colour, leaves out and says a recorded row whose path leads off the sites
    however a browser would read it and fails on a declared one before writing anything, keeps the
    last build for a record holding NaN, an infinity or a lone surrogate, and changes nothing on a
    second run
  '';
  homes-probe-reads-as-the-reader = ''
    the web-design skill's homes probe, in headless Chromium over small sites of its own, learns a
    pattern's names from the page's links on the sites alone, names each row and record the system
    reader drops, names each claim the home says it took while sites.json still makes it, keeps a
    kind's first claim, passes a page that links none of a pattern's things, fails with no row home
    to anything, and reads a shortened name's tail as no mention
  '';
  house-report-advises-by-the-file-its-key-names = ''
    a Mac's key job, its own lines run over sample keys with its Mac tools stood in for, says to
    make a key anew whichever refusal it says, by the owner, mode and access list of the file the
    key's name comes to, and not of a key others may only run or reach by lesser rights, one it
    copied, or a name that comes to no file
  '';
  house-report-hands-a-mac-its-key-safely = ''
    a Mac given a key runs house-report signing with activation's copy of it, readable by nobody's
    account alone in a folder of root's, ahead of the address; activation copies the key only once
    every folder on its real path is root's and written or taken by no other account, by its mode
    or an access list, each tested whole within the walk, reading it in that real folder, set once,
    as a plain file root's and read by no other account, through a name of each run's own, its text
    naming no chown or chmod and no install or mv but its own, however spelled; a key another
    account owns, may read or write, or may read, write, take or change the rights of by an access
    list, is said, last and once, to need making anew on both machines, not a new owner or mode,
    decided past the refusals by the file its name comes to, every link followed, whichever
    refusal is said, and one other accounts may only run or reach by lesser rights isn't;
    and the build refuses a Mac's key outside /etc, /private/etc or /var/root, or spelled through
    or ending in dots
  '';
  house-report-is-told-an-address = ''
    a Linux machine runs house-report as a service of its own account that writes nothing, started
    again when it ends but not after its usage exit, and is told only an address as the shaper's
    house reads one, IPv4 alone, sent to with the shaper's port: a name, a malformed address, an
    IPv4 one written as IPv6, or any IPv6 one, which a report would leave from a temporary
    address no house names, is refused as the machine builds
  '';
  house-report-knows-which-lists-leak-a-key = ''
    the pattern a Mac's key job tests an access list by, as the job's text sets it, takes a line
    granting another account read, write, append, writesecurity or chown, alone or among other
    rights, as letting it know the key, and not one granting only lesser rights or denying any
  '';
  house-report-passes-clippy = ''
    clippy finds nothing in house-report or its tests
  '';
  house-report-runs-as-a-macs-daemon = ''
    a Mac runs house-report as a launchd daemon as nobody, from its start and again whenever it
    ends, once the store is mounted, at a standard priority, logging to a file in /var/log made
    nobody's before launchd starts it, and sending to the address it's told with the shaper's port
  '';
  house-report-says-what-the-link-sent = ''
    house-report's tests pass: a report's text is the bytes then the ms, its total starts at
    nothing and grows by what the counter counts, a counter made again counts from nothing, an
    uplink that moves counts on from the new one's count, the way reports go is said first and
    each time its interface or address moves, this machine's own address is sent to by its
    loopback from a loopback address, and an interface's count is read from its statistics alone
  '';
  house-report-takes-up-a-macs-swapped-key = ''
    a Mac given a key copies it again, as root, by the same text activation runs, whenever the key
    or its folder changes and each minute, so a key swapped while the Mac runs is taken without a
    switch, and one launchd missed within a minute; a switch says a refusal each time and forgets
    what the job said, kept beside the copy's folder, and any once kept in it, and the job says it
    in its log once until it changes
  '';
  house-report-takes-up-a-swapped-key = ''
    a Linux machine given a key hands it to house-report as a credential read by --key, and
    restarts it whenever the key file changes, leaving one someone stopped stopped, with no start
    limit on either unit, so a key written in several steps never leaves the reporter down
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
    nothing once ZZ closes it, clearing what a killed one left
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
  ir-camera-relays-grey-frames = ''
    ir-camera keeps each sample's highest eight bits, reads and writes lines their stride apart,
    and refuses a short frame or a format it can't use
  '';
  jobs-broker-passes-clippy = ''
    clippy finds nothing in the jobs broker or its tests
  '';
  jobs-broker-runs-on-its-socket = ''
    the jobs broker runs on %t/daemons/jobs-broker.sock, shown by its name, and a switch restarts it
  '';
  jobs-broker-scopes-only-agents-work = ''
    the jobs broker makes a scope only into agents' own work, for a process already an agent's and
    named by the kernel, refuses a status of anything not a unit's name, and knows no stop or start
  '';
  jobs-sorter-knows-pi-units-by-their-names = ''
    each pi host's unit and each conversation's that host.nix renders for a realm, on a machine
    with the sorter, is named exactly as the sorter joins it: the name it is told to find that
    kind by, then the realm and kind, and a conversation's slot after an @
  '';
  jobs-sorter-passes-clippy = ''
    clippy finds nothing in the jobs sorter or its tests
  '';
  jobs-sorter-sorts-by-behaviour = ''
    the sorter moves a group busy two seconds running, never the shell's or pi's, sends a scope
    started from jobs back there unless it sets limits, and weighs a window low only while every
    pi in it is marked as one nobody is at, waking at once when a mark goes; with tiers, work
    and pis are ended lowest tier first, every tier's work before any pi, and a pi Joel was at
    within half an hour or that orchestrates is spared as the account's
  '';
  joels-programs-ask-short-slices = ''
    the sorter is told that Chrome and Slack, and no other programs, ask for a short slice, and
    only in his programs' slice
  '';
  joels-work-goes-before-agents = ''
    the sorter moves a terminal's work inside jobs, above pi's work and below timed work
  '';
  journalctl-words-skips-as-the-summary-reads = ''
    the journalctl a failed boot's summary runs, in the initrd as after it, words each kind of
    journal file it skips, and the hint on its own line under one, word for word as the summary
    reads them
  '';
  json-strings-at-their-length = ''
    in the shell's own Qt, 20,000 distinct string values of 248 characters parsed grow its memory
    by under 17 MB, where grown a character at a time they took about 21, and what's parsed reads
    as before, escapes, escaped quotes and empty values alike, an unterminated value refused
  '';
  json-strings-kept-once = ''
    in the shell's own Qt, 20,000 equal string values parsed grow its memory by under 1 MB, where
    a string apiece took about 7, and what's parsed reads as before, a value dropped by a repeated
    key whole when its twin comes after the parse has collected
  '';
  keeping-awake-holds-the-lid-only-where-it-sleeps = ''
    keeping awake holds the lid only where a shut lid would suspend, on battery or on the charger,
    and tells the shell whether it does
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
  key-makers-asked-wrongly-make-nothing = ''
    yubikey-keys and se-keys answer --help and refuse any word with 64, making nothing in an empty
    home
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
  kitty-tells-what-it-has-open = ''
    kitty's watcher, run in kitty's own Python, tells the programs daemon each change to its tabs,
    its windows and the variables a program in one says, and that it's loaded, as one ask naming
    kitty's list and its own process, passes over a resize, and with no daemon there says so once and holds kitty up for nothing
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
    drawer and ssh, orders the repositories under ~/src by zoxide's last visit, and says how many
    agents' panes its row for them holds
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
    records the microphone, and in an idle window nothing, says the most an app's thread was
    blocked in the kernel, its hidden tabs' counted from where they rested, and none where the
    kernel keeps none, reads a stand-in's named thread from the kernel, sums each generation by
    its load, and answers now and report as text for Joel or as JSON named by its schema
  '';
  last-seconds-keeps-its-ring = ''
    last-seconds reads back each record as it was written and never a torn, flipped or zeroed
    one, wraps its ring past its slots, puts seconds in a row on different pages so a garbled
    page loses one in nineteen, refuses a second recorder, and goes on after a restart's newest
    record; finds each source of a stand-in machine by its name rather than its number, again
    when found late, its fans never through the driver that holds every processor where another
    gives them, moving off it once one loads; reads each counter as what grew a second, however
    far apart it was read, an energy counter that started again as nothing and one that wrapped
    within ten seconds across its range; reads the firmware every fifth second and after each
    quick read, saying which records did, and signs the battery's current by what it says; and
    says a boot cut short after its newest second, asleep when it never woke, and the running
    one quiet when its recorder stopped writing; says no growth in the first record after a
    sleep; and the recorder refuses no seconds, a boot id no folder the reader lists could hold,
    and a stand-in's rings among the machine's own, ends a rest at once when its end has come,
    and lets in signals held back through a second that overran, a sleep and its wake together
    read as awake; run whole and signalled as the sleep's hook does, records the next sleep after
    a wake with no sleep before it and after a sleep let go once sixty reads' time had passed,
    not before and not past half as long again; and asking for one boot answers that boot alone
  '';
  last-seconds-passes-clippy = ''
    clippy finds nothing in last-seconds or its tests
  '';
  last-seconds-sleeps-wait-for-the-record = ''
    last-seconds' sleep hook, run against stand-ins of systemctl, setpriv and the reader, lets a
    sleep go once a record newer than the newest before its signal says asleep, never on an
    older sleep's, gives up two seconds on, kills a look that hangs past six tenths, tells the
    recorder it woke after, and lets a sleep go at once with no recorder running
  '';
  lately-follows-each-source = ''
    Now's Lately counts what's unread from the mail, Slack and the desktop's other notices,
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
  lid-docked-holds-the-lid-only-with-a-screen = ''
    lid-docked counts a monitor on any connector that isn't disconnected, its screen off or not,
    and holds the lid only while one is there
  '';
  lid-docked-passes-clippy = ''
    clippy finds nothing in lid-docked or its tests
  '';
  linux-builder-collects-only-what-it-is-asked = ''
    the Linux VM's collection, run for a collecting key, collects all that's dead, or at most the
    GiB asked, or nothing for free, prints the store's free bytes, and refuses anything else
  '';
  linux-builder-is-offered-only-with-room = ''
    the Mac's Nix reads its builders from the room job's file, which a job every 15 seconds,
    given the set floor, hold and log in order, a log newsyslog moves aside, keeps clear of the
    builder below the floor, as at 34% free, on a page swapped out, on a swap-out count that
    started again, with the room or its
    own record unread or that record from later than now, or when a step fails, each starting
    the hold, a shortage a failed write couldn't record among them, and saying why and naming
    the log that says when; it gives the line back only after the hold, at 35% and 600 s but not
    595 s, saying each change once in one line opening with its UTC time, a failure once, and
    that it no longer fails at the look after
  '';
  linux-builder-gives-up-on-a-guest-that-stops-answering = ''
    the Mac's ssh to the Linux VM waits five minutes for a guest to greet it and, once in, gives
    up after four asks 15 seconds apart go unanswered, each setting said once in the entry in any
    case, indent or with =, so a build sent to a guest that stopped answering fails and frees its
    locks; it reads the entry's settings, not ssh's acts or any file ssh reads before it
  '';
  linux-builder-lets-collecting-keys-only-collect = ''
    a Mac's Linux VM lets a collecting key in only restricted to its collection, never plainly as
    a builder, and is changed from the cached guest so it keeps room
  '';
  linux-builder-offers-the-features-it-is-given = ''
    a Mac's Linux builder offers benchmark and big-parallel and any feature it's given, on the Mac's
    line and in the guest's Nix, and with none given offers only its own
  '';
  linux-builder-leaves-the-macs-port-53-alone = ''
    a changed Linux builder guest's network serves it no DNS from the Mac, through a vzvm patched
    to leave the NAT's DNS proxy out, and the guest asks resolvers of its own
  '';
  linux-builder-keeps-its-network-where-it-is-set = ''
    a changed Linux builder guest's network is the one its option sets, 192.168.239.0/24 unless
    given another, handed to vmnet by its first address, the Mac's, through a vzvm patched to
    pass it on; a network that isn't private, has an octet past 255 or a host bit set is refused
    at build, one set on an unchanged guest is warned of, and a guest given nothing keeps its
    own; it pins the address handed, not what vmnet makes of it
  '';
  linux-builder-reuses-its-network = ''
    a changed Linux builder guest's vzvm patches apply in the module's order, with no fuzz, to the
    source the Mac builds, and the patched source keeps the network it makes and, past the macOS
    26 guard, hands a kept one back before vmnet is asked for another, unless a guest lost its
    attachment to it, which lets it go, and gives every guest of a process one hardware address;
    it compiles nothing, and whether vmnet takes the network again is shown only on a Mac
  '';
  linux-builder-keeps-a-changed-guests-time = ''
    a Mac's Linux VM given keys or features steps its clock with chrony at any time, past a second,
    in place of timesyncd, and one given nothing keeps the cached guest's clock
  '';
  linux-builder-keeps-a-serving-mac-awake = ''
    a Mac's Linux VM runs under caffeinate -s, so the Mac stays awake on AC power while it serves
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
  local-name-comes-back = ''
    a Mac macOS renamed gets its declared local name back, said once with what it was, at most
    once in five minutes by the time its stamp holds, a time still to come counting as old; a
    name already right is left unwritten with its stamp kept, a warning beside it not read as
    part of it, an unset one is set, one that can't be read is left alone, its stamp kept, and
    is said again five minutes on, one read again a moment on isn't said, and a name it
    couldn't read or set is said, and a set tried, once in five minutes
  '';
  local-name-answers-on-its-link = ''
    the name answers the first wired or Wi-Fi link's address, past a tunnel or a bridge and
    never from the IPv6 list, on that link alone and as a host of its own; a moved address,
    another link or none ends it and its answer, waited for; with no link, or a scutil that
    fails, it waits, saying which once, and a scutil that fails while it answers changes
    nothing; it says the name offered, not answering, before dns-sd says anything; and an
    answer that ends by itself ends it, saying so
  '';
  local-name-is-kept-by-a-job = ''
    a Mac with a local name runs two root jobs, each labelled as nix-darwin labels it, under
    the Mac's own bash, by that name and not the host name, finding their tools only in
    macOS's own folders and saying what they do in their logs, given nothing else: keep.sh,
    which sets the name back, from its start, whenever macOS writes its network settings and
    once a minute, and answer.sh, which answers it, kept running; a Mac with none runs
    neither; the module adds, changes or turns off no launchd plist but those two, nor any
    agent of the account's, launchd's environment or a file in /etc but the rotation of the
    two logs, as held; each plist is on, is linked as its own file, is the text its options
    lower to, is written from that text and opens no socket; keep.sh names, in any
    case, none of dns-sd, launchctl, systemsetup's remote switches, Remote Management's kickstart or the
    sharing command, and answer.sh only dns-sd, in one line answering the name as a host of
    its own on one link, since any other could advertise for the host
  '';
  local-sites-kit-holds = ''
    the kit every live local site is built on typechecks at its strictest and its tests pass: a
    daemon's records held as its watch tells them and each answer said whole or a change, a feed's
    watch saying connecting, live, broken or stale and since when, every line it can't read
    reported, a document's reader stepping from part to part, every step held at its list's ends,
    and g pressed twice going to the top
  '';
  local-sites-passes-clippy = ''
    clippy finds nothing in the local sites' server or its tests
  '';
  local-sites-refuses-renamed-look-acts = ''
    local-sites' home module refuses a machine whose launcher renames one of its acts on a look,
    in place or by a later entry keyed by a list of one set or a set words.json writes as the id
    by its outPath or __toString, or by a whole entry so written, reading the words as words.json
    writes them, or in a words.json a machine writes itself, and refuses that file given as a
    source it can't read, though never one made from its text, core's or a machine's; it names
    only each act's old and new word though another word is renamed beside them, even to no name,
    and takes core's own words and ids Words.qml keys as no act: objects, lists of several or
    none, whichever end holds the act, and a path in the store
  '';
  local-sites-rows-homes-read-as-readers-do = ''
    the rules a switch asserts over declared local site rows name each kind two sites are home to,
    each pattern the homes' readers drop, every byte they refuse included, and each kind a site
    gives more than one pattern, which the readers pass over, naming each once, and let a row give
    its own kind both by name and by pattern, or one pattern again however it's written
  '';
  local-sites-serves-its-sites = ''
    the local sites' server serves the documents and a mount, answers a file the browser holds by
    its tag alone and sends one written since again, even at the same size and time, refuses a
    stranger's host name and a stranger's page, has two pages share one watch on a daemon with
    each told every change trimmed to the feed's fields, refuses an act before it reaches the
    daemon, and says a missing daemon broken with why
  '';
  locking-asked-wrongly-does-nothing = ''
    lock-screen, lock-for-sleep and swayidle-away answer --help and refuse an unknown word with
    64, asking nothing of the shell, niri or swayidle
  '';
  lookups-say-a-name-that-fails-or-takes-long = ''
    a Mac's account times its names each minute through the system's resolver and each server
    named, at Standard, given the slow mark, names and servers in order, and a round over
    stand-ins says a failed lookup, one out of time, one ignoring its TERM killed five seconds
    on, one killed by anything else, unable to run, failing on its own or ending otherwise
    saying so, an address beside a nonzero end not believed, a server giving no IPv4 address (a
    CNAME or nothing alone, as a name with only an IPv6 address gets, an AAAA line turned down,
    or a dig with no reply, said as that) or a slow lookup from its second round running, each kind
    counted apart so a failure then one cold answer says nothing, one failing and lagging by
    turns from its third, and a busy mDNSResponder, by its path or its name, from its fifth, as
    errors opening with name-lookups,
    each well again once, and nothing while all is well; a state file's blank line, a word for
    a count or a zero before a count's other digits is read past, never run, rather than ending
    the round or dropping checks from it; a round that ends early, before any name or at its
    save, says so as an error; and one stopped by a TERM mid-lookup says nothing
  '';
  looks-probe-finds-what-a-deploy-lost = ''
    the web-design skill's looks probe, in headless Chromium over small sites of its own, lists a
    page's stylesheet, face, type, colours, masthead, mark, drawings, images and scripts as drawn,
    names each one lost when a deploy leaves its words in a plain shell, names a stylesheet, a face
    or an image that answers 404 broken, keeps what was broken before as still rather than failing,
    lists art a stylesheet paints on the body too, an unused face, a canvas and a lazy image, keeps
    a rebuilt asset's name past a hex, base64 or base32 hash and a page's assets past its move to a
    folder, names a dropped page's 404, takes no button's icon for the mark, says a loss named as
    meant with why, loses nothing to a change of words or labels, measures drawings in shut folds,
    an exclusive group's too, and leaves them shut, fails a page that draws nothing, and refuses a
    before answer or meant losses set wrong
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
    it runs, restarted by a switch, and handed pw-cli, by which it finds who reads a camera
    through PipeWire
  '';
  machines-remake-only-the-machine-that-changed = ''
    the launcher's Machines makes again only the row of the machine a node change names, keeping
    the others, and none while the launcher doesn't want it
  '';
  a-vpn-service-keeps-its-memory = ''
    a VPN's service named for it never swaps, keeps a hard memory floor that system.slice
    claims once for each, and carries the weight that puts it ahead of the other system services
    where the disk shares by weight; with none named system.slice claims no floor; a packaged,
    scripted, required, socket-started or timer-started service is taken, and a name with
    .service, one no service has or one the slices module sets itself is refused
  '';
  machines-yield-to-what-the-person-works-in = ''
    system virtual machines are weighed as a build is, and the account's containers and session
    machines are idle
  '';
  maps-keep-their-meaning = ''
    in the shell's own Qt, a Map or a Set finds keys equal by value and only those, objects by
    who they are, in the order they came through deletes, clears and walks that change it, and
    every one of forty thousand keys it holds
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
    won't apply with its breaker as owner, listing it by the subtests that failed, a listed
    test's new ones in an entry of their own under their own owner, reverts
    nothing for one quarantined at the round's head or main's newest, nor for a run failing
    only in subtests main lists, but searches one failing in another, starts nothing while
    the machine is held, says
    which changes run their VM tests before merging, a file a test names passing on what it
    names unless it builds a crate, a Rust file changed only in comments left out, and which
    moved read from a run that passed at a clean head
    rather than evaluated again, and lets a head merge whose same change
    passed before a rebase that moved none of its files and no Rust crate its crates are in, read
    or are read by, and takes a net quiet 20 minutes, running nothing, every thread on a lock,
    as hung, and asks whether a landing is still wanted with the checkout given back
  '';
  merge-net-outlives-a-switch = ''
    a switch leaves the net and main's keeper running, each to end between its rounds or keeps,
    and the keeper runs the very build it follows, from a link of its own outside the profile
  '';
  merge-net-passes-clippy = ''
    clippy finds nothing in merge-net or its tests
  '';
  model-relay-carries-requests-whole = ''
    model-relay, against an HTTP/2 origin of its tests' own, forwards a request's method, path,
    headers and body as they came, a body of many pieces whole, refuses any other host, carries many
    requests at once on a few connections, streams an answer as it comes, opens again a connection
    the origin lets go, sends a request the origin reset before its answer, or left out of a goodbye
    saying nothing was wrong or a protocol error, once more on another connection, one already open,
    even beside one kept, or one opened for a burst of them together, whose failure to open each
    hears at once, and sends pi direct with its own 421 when reset or left out again or lost, when
    its connection was kept with none other open or when none would open, but says a 502 when a
    reset, first or again, said to calm down, retiring its connection, sends on no body too large
    or left unfinished, says another origin's 421 once the body is read, even a body too large,
    counts a body pi left as its cancel, never a 421, tells a body past its deadline as late,
    passes a TE of trailers as h2 sends it, says its counts as it leaves, answers broken off among
    them, both those a fallen connection broke off and those the origin did, counts on each
    connection the requests pi gave up on and no answer read whole or broken off, tells one a
    fallen connection lost from a reset, the origin hearing a lost one once, says an origin out of
    reach as a 502 or a 504 without queueing, says an origin whose name can't be looked up as
    that, doesn't ask for a connection that won't open on every request,
    forgets a failure once the origin is back, sends pi direct with a 421 when it can't
    trust the origin, answers beside an answer longer than its idle time, leaves once idle even
    while refusing other hosts, held open by a kept connection or knocked on as it goes, finishes
    each answer it streams when stopped, sending what comes meanwhile direct with the 421 of one
    leaving, past the ten seconds a relay leaving idle takes them, gives those a few seconds once
    its answers end, even one pi says nothing on, cuts its answers when its drain time runs out,
    not when those few seconds do, or at a second stop, ends what it sends direct when stopped as
    it leaves idle, at once if stopped again, says what it waited on and its counts once more, last,
    once what it cut has ended, even with its counts moved meanwhile, its cuts apart from pi's
    cancels, counts a request still waiting on its first connection as cut when stopped and as
    pi's cancel when pi leaves it, and one whose body is still being read as cut, by a drain or by
    a stop as it leaves idle, refuses a drain of none and each flag it no longer takes, telling
    them by their words, keeps its socket to its account, from a second
    relay and off a file or a link that isn't a socket, and logs nothing of a header or a body, a
    failure included; and its own cases hold a ping from behind an upload, with little held
    unsent in its sockets and short frames, retire a connection by its number alone, sparing one
    opened since, and at most once a minute for a reset, have the kernel probe by count alone
    each connection still open as a relay finds the machine slept, however it was retired, and
    stop once done, one marked but not named too, uncounted, but not one let go, nor hold that
    one open, nor stop a later sleep's probes though ending their own, and say an error with its
    causes, cut to a line
  '';
  model-relay-passes-clippy = ''
    clippy finds nothing in model-relay or its tests
  '';
  model-relay-runs-in-background-slice = ''
    model-relay is installed only where a machine names an origin, listens where only its account
    may ask, removes its socket when stopped, runs with its socket in background.slice, forwards
    to the origin named alone, outlasts a switch, stops waiting on its answers in time to leave
    before its stop timeout, keeps more connections than its default and no more than it takes,
    and tells every pi where to find it
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
  mycelium-node-takes-its-flags = ''
    the mycelium core locks takes every flag a workstation's node is started with, each with a
    value where its line gives one and none where it doesn't, so a flag can't reach the node's
    service before the mycelium that reads it
  '';
  mycelium-surfaces-stay-even = ''
    every act Joel takes on his cells is on the mycelium command, a pi session, the map and the
    launcher, or owed by a cell by a time, or waived with a reason: a file said to have it holds
    its words and the locked mycelium's help says a command's, and an act the launcher's session
    row or the map's cell drawer lists is one of the table's; a clean act passing, and a surface
    left out, two standings, a missing file or word, a help that fails, an owing with no cell,
    time or why, a waiver with no reason, an act naming no way to it, an act a surface lists that
    the table doesn't, no longer lists or maps to no act, and a list read as holding none, each
    failing first
  '';
  mycelium-completes-its-teams-in-zsh = ''
    mycelium swarm completes in zsh with its acts and the teams the line's realm's node names, a
    team after an act, an act after a team, and nothing for the words said to one
  '';
  mycelium-links-what-a-machine-names = ''
    a workstation in a realm runs mycelium's node there, dialling each machine it names, over SSH
    or through a provider's program, and asking each provider's program it names what it could
    make, as a user service on Linux signing in with the agents' key,
    or the key its realm names, through that key's agent rather than one the account's SSH config
    names, lending that key to no machine and adding no key file to its agent, and an agent on a Mac whose sessions sign commits with that key,
    finding the system's ssh before the account's programs; the node may hold 65536 open files
    on Linux, under a hard 524288, systemd's own default, and 10240 on a Mac, naming no hard limit;
    the home realm's node writes how it would ease the machine, naming each machine lent for
    agents it may ease onto, by a name the flag can't misread, asking the harness to park
    sessions and making its carries only where told to, on Linux and a Mac alike, and a further
    realm's doesn't; one naming none runs it dialling nothing, and one in no realm has neither a
    realm nor a node
  '';
  mycelium-carry-guards-match-its-lock = ''
    the mycelium core locks has a line of code matching each of carry-guards.nix's patterns, so
    a lock moving to one that wrote either guard in another form fails here rather than every
    machine on core's lock ceasing to carry
  '';
  mycelium-carries-only-with-its-guards = ''
    a workstation's node is told to carry only where the mycelium it is built from has a line of
    code, not a line comment, for each guard that keeps the person's and the leading sessions,
    each tried alone and with neither file there, and a node built from one writing both in
    forms the patterns accept carries; where one is missing the switch warns once, in the
    system's warnings on Linux and a Mac, naming that mycelium and only the files lacking one
    and how, rather than refusing the machine, and a machine that doesn't carry or runs no node
    is never warned
  '';
  mycelium-watches-its-balance = ''
    a machine in a realm reads each minute how evenly its home realm's machines carry their work,
    keeping each reading in mycelium's state, in the agents' slice on Linux and as an agent on a
    Mac; a further realm's balance isn't read, nor any on a machine in no realm
  '';
  mycelium-shows-the-builders = ''
    the home realm's node on Linux is given nix-offload's standing, beside and lent lists of Nix
    builders and the folder each lender's health is in, so it shows them beside the machines; a
    further realm's node and a Mac's are given none
  '';
  mycelium-signs-a-macs-links-with-the-enclave = ''
    a Mac's node signs its links in with the agents' key from the Secure Enclave, through that
    key's own library and no agent, so the login session's agent never asks for an approval;
    a Mac without that key's module, and Linux's node even when its signer names such options,
    are given none
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
    the attach; a machine naming none authorizes nothing of mycelium's, a realm or machine name
    opening with a hyphen, which mycelium would read as a flag, is refused, and so is a key of two
    lines, whose second would be authorized unpinned, while a key's last newline is dropped
  '';
  mycelium-serves-agents-of-another-account = ''
    a workstation naming the account its agents run as gives its home realm's node a socket in
    /run/mycelium answering that account alone, which NixOS makes at boot for the node's account
    alone to write; a further realm's node and a machine naming none serve no such socket, and a
    Mac naming one is refused
  '';
  mycelium-takes-its-link-key-s-words = ''
    the words a link key pins mycelium to, read from the line a machine's system writes, are ones
    the locked mycelium takes: run as sshd runs them for a login, with no command asked, they
    parse whole and reach the action, which refuses the login and keeps the refusal naming the
    machine the key is from, so a renamed or newly required action or flag fails the check rather
    than the first link or attach
  '';
  mycelium-map-check-answers-soon = ''
    mycelium-map-check says what its last reading found and when, with the day if not today, then
    that it's looking again, before a stand-in for its browser test opens; it keeps each answer
    whole for the next run, says nothing on the way as JSON or of a kept file it can't read, and
    hands pytest a test alone in a folder of its own rather than one in the Nix store
  '';
  mycelium-runs-among-the-sessions = ''
    on Linux, every realm's node runs in the agents' slice, beside Joel's programs rather than among
    them, and keeps what it swaps out compressed in memory, never writing it to disk
  '';
  mycelium-site-holds = ''
    the mycelium map typechecks on the kit at its strictest and builds, its tests passing: the
    networks and cells as the command prints them, machines' cells left out, two cells sharing a
    title kept apart, each machine's room and reach flattened, only the latest messages' senders
    and groups, a cut title's ellipsis as three full stops, and each machine's share of the work
    from the balance's latest reading, none when it is past three minutes old, has no time or is
    not served, what a receptor fed a cell, kept for ten minutes and up to a minute ahead, the
    map drawing again when the first balance or feeding ends, who started each cell from its
    own record on any machine, else its forage, none where neither says and none for a cell
    started by itself or by nobody, a cell two machines list kept once as the copy started
    latest with either copy's networks, its drawing, reach and starter all read from that copy,
    reach judging it by both copies' networks in either order, a cell with no name the map
    can't draw found unreachable and said as words, with its machine, rather than lost,
    a network the node sets within none growing out of the one whose cells started or split
    more of its cells than any other, a tie going to starts and then by name, where the node
    sets a network standing, no start from inside it counting, of two that started each other's
    cells the stronger the trunk and no branch looping back round, every network's choices
    settled strongest first so a choice left over from a loop waits behind stronger ones, a
    cell's forage saying what it split from, else the first forage listing it, for a cell with
    a forage here, growing a branch where no start is said or the start grows none or would
    loop, counting toward the trunk as a start does and counted apart from the starts, a cell
    started and split from two networks counting toward each, the drawer saying a branch's
    starts and splits each out of its cells, what each key asks of
    the map, its tree, its find line, the
    page and the theme switch, chords, typing and a key named after an object's own asking
    nothing, the tree's networks and branches by name under a machine or a network and the
    reading's order at the top, their cells worst first, the rows only open branches show and
    where each tree key goes from a row, edges included, the builders the node reads kept as
    builders, never cells, and rooted after the machines, what each cell, network, machine and
    builder is in words, counts, sums and labels included, each time said against the reading's
    own with
    its day only off the reading's and a lease's time left, and how each set of cells and each
    machine stands and why, worst first, its form, its share and the balance's flags, what each
    set of cells, receptor and cell is doing and how much of the talk asked or found something,
    what each spent today, most first, ten to a list and the rest counted and summed, what
    waits on each cell and what holds stand, oldest first, said as the page names each, a
    question to a cell said as an answer it owes, a network counting the answers owed rather
    than the cells owing them, and a cell drawer's acts in order, Go to It first, and its
    question whole, paragraphs and lines kept
  '';
  mycelium-site-reads-live = ''
    the mycelium map, served by the local sites' server beside a stand-in node through the node feed
    as home.nix declares it for a machine in a realm, reads live in headless Chromium: the realm
    drawn and counted, its networks, a branch and lines across them in one window that opens a cell
    clicked, a cell moving and one going, two cells sharing a title moving each owing an
    answer, the legend saying a ring is an answer owed and never one waiting on him, broken
    while the node is away with the realm
    kept drawn and live once it's back, stale once the server goes with the realm kept, a record's
    words drawn as words and never as markup, what each spent today listed most first in Now, a
    machine's drawer and a network's, a network two machines feed listing each with the cells it
    draws there, its branch's counted in, what waits on a cell and what holds stand said as the
    node
    judges them, every cell drawn a step out from the whole map and folded only once too small to
    read, every line thinned as the map steps out past its own size, the talk between two networks
    drawn as one line as warm as its warmest pair until it or a cell it touches opens, a receptor's
    feeding of a cell drawn with both drawers saying it and leaving once its ten minutes end or the
    node drops it, a network grown out of another as its branch once a forage says a cell of the
    other started one of its own, its hub laid out again as a branch, both drawers saying so and
    the branch going with the forage, grown alike from a forage saying only what the cell split
    from, both drawers saying it split and neither that it was started, and grown across
    machines once a cell's own record names
    its starter on another, going once it stops, the copy drawn and the branch changing together
    while two machines list the cell, the builders standing under the machines in
    rounded boxes,
    dashed while taking no builds, their words read unopened, opened by link and let go by m,
    an asking cell's drawer saying its question whole and leading with Go to It, which g follows,
    a key of each
    kind keys.ts decides answering (the keys dialog, find and a letter typed into it, n and N,
    Escape in its order, the walk, the machines, Now and Escape closing it, the view zoomed in
    and out about its middle, panned a step across and down and fitted whole again, -
    climbing from a cell to its network, the
    tree stepping, closing, opening, going in and out of a branch and opening a row, and the
    theme switch wrapping),
    the web-design skill's page checks and the map's own (each opened thing in view, its drawer
    saying what it is, no more names than it's engaged with) passing at a phone's width and a
    desk's in both looks, a first visit within its budget of files, bytes and elements, both
    visits reaching only the sites' server and the second answered unchanged, and nothing logged
  '';
  mycelium-serves-the-harness-files = ''
    each realm's node serves the harness's files of that realm, its open and ended quests, the
    sessions to restore, its runs of subagents and reviews, what its sessions landed, and the talks
    in its steer inboxes,
    from the realm's own folders, and the home realm's alone the machine's waiting, verdicts and
    queue, the queue's jobs and the queue whole from the login's runtime folder, the void's
    answers whole, a line each, and the balance service's latest reading whole, each keyed as
    the launcher reads it, a wait with no id by its title; and each node, on Linux or a Mac,
    restarts when what it serves changes, since it reads that only as it starts
  '';
  mycelium-says-how-its-machine-stands = ''
    each of a machine's mycelium nodes, in its home realm or a further one, is given the system
    daemon's socket, whose words its machine record carries: Linux's in the runtime folder, a
    Mac's in the account's state folder, where its launchd agent serves; a machine naming no
    socket gives none
  '';
  mycelium-carries-the-realms-kit = ''
    each of a machine's mycelium nodes, in its home realm or a further one, is given its realm's
    kit last among its words, one folder in the store holding the tools every machine shares and
    whatever the machine adds for that realm alone, which each session and run the node starts
    finds first on its path, a curl speaking wss:// among them, leaving git and gh to the account,
    which may wrap its own; a machine emptying its kit gives none
  '';
  mycelium-kit-builds-here = ''
    the tools of the kit a machine of the system running the checks is given build there and hold
    curl, fd, jq, mycelium, rg and websocat, its curl speaking wss:// over OpenSSL 3, so a tool
    one system lacks fails on that system's checks before a machine of it switches
  '';
  mycelium-declares-what-work-needs = ''
    a machine declaring what work begun in some of its folders, or naming some words, needs has
    it written where mycelium reads it, in mycelium's words: a cloud need alone and every other
    as its kind keyed to its value, as `mycelium sprout --need` takes them; nothing is written
    where nothing is said. A need mycelium can't read or no machine could say, a rule naming no
    need or no folder and no word, a folder that would match nothing or everything (from ~ or $,
    with an empty, . or .. part, or space at an end) and an empty word are refused. The need
    kinds are the locked mycelium's, read by a scan that fails on what it can't read, and once
    the lock reads these rules, their field names are its own
  '';
  mycelium-declares-what-a-machine-says = ''
    a machine declaring what mycelium's node can't measure (its reach, a probe saying whether a
    call is on there through a-call-is-on alone, what a held session outlives, whether work
    costs money, its taints, its agents by realm, the fences its agent sessions run in, whether
    pi fences each command itself) has it written where mycelium reads it, in mycelium's words;
    a fence it doesn't say is left out, pi_fences_itself is written only when a Mac says so,
    beside its fences and their launcher, and refused on Linux, the doors a fence accepts for
    another machine are written only when named and refused for a machine no realm could name,
    a fenced or strict one says its system's launcher for an agent's run, which runs pi's strict
    profile, and an unfenced one none, no fence says a launcher when agents' runs are turned
    off, a machine of either system declaring every fence evaluates, a machine kept lively
    says its ceiling, a memory_full of 0 included, and one not kept lively none, the locked
    mycelium reads that ceiling as kept_lively, an Option<Lively> whose cpu is a u32,
    memory_available an Option<u64> and, from the lock that first has it, memory_full an
    Option<u32>, read by a scan that fails on what it can't read, with nothing over Lively
    refusing a field it doesn't know, one declaring nothing has no file, and agents in a realm
    it doesn't join are refused
  '';
  mycelium-lends-from-the-store = ''
    a NixOS machine that lends evaluates, and its home realm mycelium node is given what it lends
    the worker machines it made as that file in the Nix store, rather than one in the account's
    config folder, its fields exactly those the locked mycelium's lender parses the file into, by
    name and type, from structs that derive Deserialize with only deny_unknown_fields over them
    and no field attribute but default, which a scan shown to refuse eleven other layouts reads,
    and the lender joins a relative upstream onto its runtime folder; the budget kinds the lend
    takes, and the one it writes where none is said, are the lowercased variants of the lender's
    Kind and the one marked default, read from an enum that derives Deserialize with one lowercase
    rename over it and plain variants, by a scan shown to refuse ten other layouts; a lend
    leaving a machine's also and a budget's kind unsaid is written with them filled in as the
    lender takes them, beside a forge budget a machine names; a lend capping no machine's dollars
    and pricing no model is written without them, which mycelium's own tests show its lender
    reads as lending no model,
    its budget's prices among the fields the scan reads as defaulted, a scan shown to read as
    defaulted only a field whose own line above, past its doc lines, is the attribute, and warned
    of rather than refused, naming which it lacks, as is a models budget naming no spend-watch
    budget or pricing none, and a machine lending models only beside a forge, each still meeting
    every assertion; a lend whose every models budget is priced and watched and whose machines
    are capped is warned of by neither warning, nor is a machine lending only a forge or its
    forge budget; a price leaving its cache's prices unset is written without them, and a model
    with any of its four prices at nothing is refused by the option's type; a
    budget with no key is written with
    none, a further realm's node and a machine lending nothing are given no lend, a Mac's lend is
    refused by a message naming it and its node given none, plain prefixes and dotted paths and
    key packages are taken, and a lend with no realm, naming a budget it lacks, with a prefix that
    is empty, starts another, either way round, or is named twice, a key that is empty or whose
    program is outside the store, climbs out of it with .. or is named as text no package keeps
    alive, a whole upstream or one with a dot part, a host with a scheme, a header holding a
    space, a budget of a kind the lender lacks, or a path of / or /* alone or holding .. is
    refused, each by its own message or the option's type
  '';
  mycelium-node-reads-its-declaration-from-the-store = ''
    each realm's mycelium node, on NixOS and nix-darwin, is given what its machine declares as a
    file in the Nix store, which only root writes, holding the same words as the account's config
    folder, so a link there can't widen how its agents' commands run; a machine declaring nothing
    names no file
  '';
  mycelium-node-reads-its-stimuli-from-the-store = ''
    each realm's mycelium node, on NixOS and nix-darwin, is given the person's table of which
    receptors say when more is asked of them as a file in the Nix store, which only root writes,
    since the node refuses a table any other account could change; a machine naming no source
    names no file, so every source stays silent, and a source mycelium has no receptor for is
    refused
  '';
  mycelium-table-names-mycelium-s-own-sources = ''
    the sources a machine's stimuli table may name are exactly the ones the locked mycelium's
    receptors read, as its own source names them, so a renamed source can't make the node refuse
    the whole table and silence every source with nothing failing
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
  net-passes-clippy = ''
    clippy finds nothing in net or its tests
  '';
  net-reads-the-home-network = ''
    a real modem's pages and eero's golden record collect into net's golden record, the modem
    first with every channel and its log placed in time by its own clock, the gateway eero one
    device with the router and access point roles and its cable to the modem, and a source that
    doesn't answer is said so while the others' devices are kept
  '';
  network-site-holds = ''
    the network page typechecks on the kit at its strictest and builds, its tests passing: nothing
    drawn until every kind of net's records is told, concerns worst first by net's rank rather
    than the order a watch hears them, a record it can't read named rather than drawn, the
    floors laid out top first with each device on its own eero, and a device's command naming it
    by a word that finds it alone and pastes as it reads, quoted as the terminal kit quotes, and
    the opposite arrow retracing the step just walked only while the map is drawn the same way
  '';
  network-site-reads-live = ''
    the network page, served by the local sites' server beside a stand-in net and node, reads live
    in headless Chromium: every device drawn in its tree and on its map, a name with markup in it
    drawn as words, one going with its link, a machine mycelium runs on saying its cells as the
    node moves and linked to its place on the mycelium map, broken while net is away and live with
    its devices read again once it's back, stale once the server goes and keeping what it drew,
    every device and wire on its map within a phone's width with nothing to scroll sideways and
    each zoom button 44 px inside its set, a bigger house with two switches, a third eero and a
    full floor drawn the same way at a phone's width and a laptop's, the map drawn across on a
    desk, stacked once the window narrows to a phone's and across again once it widens back,
    the device the keys are on in view once the map narrows to a small phone's and once it
    widens to a short desk's, neither flip jumping back to the worst concern, stacked in a window
    whose room falls short of across only once its floors' names count, a first visit within its
    budget and a second answered unchanged, the web-design skill's page checks passing at a
    phone's width and a desk's in both looks, and nothing logged but what a test causes
  '';
  network-revive-looks-once-the-network-settles = ''
    network-revive looks at the network once a burst of changes goes quiet, or anyway if it never
    does, and reads each unit's check and whether it reloads, refusing a setting it doesn't know
  '';
  network-revive-passes-clippy = ''
    clippy finds nothing in network-revive or its tests
  '';
  night-shift-eases-its-bars-only-while-joel-is-away = ''
    a night job easing its bars while Joel is away tells he is from swayidle-away's mark, rhythm's
    minutes and a call on this machine, their paths left to systemd to expand, gives each eased bar
    and the wait before one, and is refused where it eases a slice it has no stop for, where an
    eased bar is under its own, or where nothing in the account keeps the mark
  '';
  night-shift-passes-clippy = ''
    clippy finds nothing in night-shift or its tests
  '';
  night-shift-runs-only-at-night = ''
    a declared night job is tried only at night and never caught up on by day, starts only within
    its hours however it was started (one on tries of its own only within hours it names, and
    hours holding no minute are refused), runs behind the account's work with the limits it names
    on the job's own cgroup, not its watcher's, stops by the apps', the desktop's and agents'
    slices, watches the disk only when given an io bar, escapes its command for systemd, eases no
    bar while Joel is away unless given one to ease, and an account declaring none gets no unit
    but still has night-shift on its path, for a heavy job's turn
  '';
  night-shift-stops-for-joel = ''
    night-shift runs a job once a night on a quiet machine, starts nothing outside its hours, even
    once a wait for the turn outlasts them, nor on a busy machine or beside another, stops one for
    a watched slice's stall or the machine's past its backstop and kills what ignores TERM, starts
    nothing and stops what runs past its io bar, lets a job stalling only itself run on, runs a
    job with limits in a cgroup apart from its watcher or not at all, and fails with a failing job
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
  niri-draws-a-fixed-surface-once = ''
    the session's niri, nested in the sandbox, draws a bottom layer surface its unit names once on
    its screen, where it is, all through a workspace switch caught partway
  '';
  niri-fixes-the-cat-to-its-screen = ''
    niri's unit names the margin cat's layer namespace among the surfaces drawn once on their
    screen
  '';
  niri-hears-the-copilot-key = ''
    the Copilot key types, in the keymap niri builds, the key its dictation bind names
  '';
  niri-is-asked-for-full-range = ''
    niri's unit asks for full-range RGB on a screen whose EDID takes it, by the name its patch reads
  '';
  niri-is-asked-for-panel-saturation = ''
    niri's unit asks for a laptop panel's saturation only where a machine sets one, by the name its patch reads
  '';
  niri-is-held-as-the-windows-daemon-tells-it = ''
    The shell's niri holds what the windows daemon tells in niri's own shape, and says a window
    opened, closed or moved, a cast or a screenshot, as niri's events did, with a window used or a
    workspace activated making no list again
  '';
  niri-keeps-a-blur-ready = ''
    the session's niri, nested in the sandbox, takes back the textures a launcher-named layer's blur
    let go while its blur region was empty, making none when it opens again, and takes back the set
    it gave up on growing when it goes back to its first size, and the grown set when it grows
    again, but not for a size of the grown width at another height or of the grown area on its
    side; of fourteen sizes nearly filling the screen, it takes back one three others followed,
    and takes it back whole again once it is the oldest the pool holds whole, so the pool holds
    at least about 63 MiB and an effect takes back the set kept for its size before keeping the
    one it gave up, and the pool never says it keeps more than 64 MiB once it lets go, so it
    holds less than about 68 MiB, and its let-go lines name the walk's first size among those it
    lets go past its cap, never the second, which it keeps
  '';
  niri-keeps-secret-copies-from-sandboxes = ''
    the session's niri, nested in the sandbox, offers a sandbox's focused window no copy a password
    manager marked secret, in either selection, while ordinary text still reaches it and Joel's own
    window is offered the secret and reads it
  '';
  niri-lets-x11-in-only-with-its-cookie = ''
    the session's niri, nested in the sandbox, starts an Xwayland that lets in an X11 client holding
    its cookie and refuses one without it
  '';
  niri-moving-the-keys-makes-no-list-again = ''
    The shell's niri moves the keys, to a window or to a workspace's other window, and keeps a
    window used, without making any list again, and still lists again a window retitled, opened or
    closed
  '';
  niri-names-each-screen-once = ''
    niri's config names each screen in one output block, the panel's holding srgb-gamut, and refuses a
    second block for a screen, which niri would ignore
  '';
  niri-never-shows-its-own-screens = ''
    no niri key opens its screenshot screen, its key list or a quit that asks, and the launcher's
    Log Out quits without asking
  '';
  niri-opens-sandboxed-windows-behind = ''
    the session's niri, nested in the sandbox, opens a sandbox's windows on a workspace no screen
    shows without moving the focus, even with an activation token from the focused window or while
    the focused workspace is the empty one at the end, and Joel still goes to one by its id
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
  niri-renders-by-the-deadline-up-to-60-hz = ''
    niri is built with both render deadline patches, and its unit asks for the deadline and keeps
    it to screens of 60 Hz and slower, by the names those patches read
  '';
  niri-says-what-the-gpu-gave-it = ''
    the session's niri lets smithay say which GPU priority niri got, keeping the rest of niri's own
    log filter
  '';
  niri-shares-the-launcher-but-no-password = ''
    no layer rule in the account's niri keeps the launcher out of a shared screen, and the last
    rule saying of polkit, askpass and the notices keeps each out of every capture, a recording's
    screencopy as much as a share
  '';
  niri-starts-able-to-go-first = ''
    the session's niri starts through its wrapper, which holds CAP_SYS_NICE
  '';
  niri-tells-a-resize-at-once = ''
    the session's niri, nested in the sandbox, reports a window's resize over IPC within half a
    frame of handling the commit taking it, by niri's own clock, saying a resize moves the tile
    and how long ago it began
  '';
  a-mac-signs-what-it-builds = ''
    a Mac's Nix is given a line to sign with its own key, made before /etc under the name given
    or the Mac's own, and a hook that signs what each build ends with
  '';
  a-mac-signs-what-its-builder-built = ''
    the hook a Mac's Nix runs after each build signs every path it names with the Mac's key,
    asks nothing of none, and fails no build when the signing fails
  '';
  a-stopped-kill-joins-its-timer = ''
    the machine's Nix lives through a stop that comes while it waits on a child it killed with a
    timer behind the signal, as the daemon's process for a connection kills its build hook
  '';
  nix-builds-yield-to-the-person = ''
    Nix's builds run at a lower weight than the person's programs
  '';
  nix-builds-only-in-its-sandbox = ''
    a NixOS workstation's Nix daemon is set to sandbox every build, never relaxed, and to fail
    rather than build bare when no sandbox can be had, with no later line in its config saying
    otherwise
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
    takes it, else tells changed-checks to run here, and after a run gives the other lent
    builder the checks and reads its base kept, whole and never over its own; a change whose
    base no lent builder keeps is checked replayed onto the newest main one does, saying so,
    never when it holds a merge, a commit doesn't apply or main moved a file beside it or one
    every check reads, though its own files may move onto a newer main, and onto an older
    where ready reads their lines apart, and runs again on its own base should the replay fail; and `relay`
    dials a far side again
    while it closes before saying its first line, passes that line and everything after
    both ways, and hangs up once, without dialling again, on one that says nothing in time,
    and ends once its far side has answered no caller for long enough, never while idle; and
    `lines` copies into root's files only the lender's lines whose URI, whole, key and host key
    a standing builder's settings give, or a lent builder's instance does, named by prefix and
    zone and its host key asked of the cloud once, each line of eight fields and the rest plain;
    and a test main quarantines is built alone, its failure blocking nothing unless its entries,
    held as one naming each owner, name subtests and the failed tests its log names aren't all
    among them, none named included
  '';
  nix-offload-lenders-outlive-a-switch = ''
    a switch that changes a lender's unit reloads it rather than restarting it, and a lender
    that ends superseded is started again from the new unit; every VM lender, and no other,
    warms each builder it stands, by default with a check core still has
  '';
  nix-offload-lines-reach-root-alone = ''
    with nix-offload's lines set, root's Nix reads its builders only from the folder their own
    account writes, the service runs as it with its settings, which carry each standing
    builder's systems and most builds at once, and the lender's account needn't be trusted;
    unset, root reads the lender's own and its account must be
  '';
  nix-offload-passes-clippy = ''
    clippy finds nothing in nix-offload or its tests
  '';
  nix-offload-root-outwaits-the-relay = ''
    root's ssh waits for a lent builder's first line longer than the builder's relay goes on dialling
    its tunnel, so a tunnel slow to open costs a build a wait rather than its only builder
  '';
  nix-offload-standing-door-trusts-no-one = ''
    the lender's door to the standing builders, run as its socket unit runs it, answers as the
    Nix daemon and takes its caller as untrusted, names root's list of them through the daemon,
    and only the lender's account may open it, in a folder only root writes
  '';
  nixos-role-twice-is-once = ''
    importing a NixOS role twice leaves the machine unchanged
  '';
  no-name-says-a-replaced-word = ''
    no check's name and no jitsusama.* option's says a name docs/glossary.md replaces, but the
    other senses and the renames still waiting that tests/repository/names-kept.nix keeps, each printed with
    its word, and the check refuses a stand-in name breaking each of its rules
  '';
  no-words-leave-their-room = ''
    in the shell's own Qt, words whose ink runs past the box they're laid in, past the card, or
    onto another's are each named with their fault, and words inside all three, meeting at an
    edge, held in a field, or past by a glyph's bearing are not
  '';
  noise-cancelled-cleans-a-noisy-clip = ''
    a person's voice recorded over a room's noise comes out of each noise-cancelled cleaner, on
    the CPU, with the noise past it as much quieter as the cleaner promises (10 dB for
    DeepFilterNet, 6 for its gentle one and RNNoise), the voice within 6 dB of how it was
    spoken, and the gentle one keeping more of the room than the full one
  '';
  noise-cancelled-keeps-the-voice-it-cleans = ''
    every noise-cancelled cleaner, as the DNSMOS scorer hears it, makes a voice in a room's noise
    and in pink noise sound better overall, and leaves the voice mark no more than 0.3 under the
    noisy one's, so a cleaner that cuts holes in the voice is refused
  '';
  noise-cancelled-scores-a-clean-voice-better = ''
    the DNSMOS scorer beside Joel's ear, on the CPU, marks a voice, the same voice over a room's
    noise, and that mix cleaned by DeepFilterNet out of 5, and marks the voice alone at least
    half a mark quieter in the background and better overall than with the noise
  '';
  noise-cancelled-scores-with-the-trials-models = ''
    noise-cancelled's service is given the same models folder as Choose by Ear's trial, holding
    the scorer beside the cleaner, so each call can be scored after it ends
  '';
  noise-cancelled-switches-to-the-new-one = ''
    a switch restarts noise-cancelled after it reloads the units, so the microphones offered are
    never the old program's
  '';
  noise-cancelled-reaches-no-network = ''
    the always-on noise cancelling, which holds each call's first speech to score it, may open
    no socket but a local one
  '';
  noise-cancelled-trial-reaches-no-network = ''
    the service behind Choose by Ear, which holds a few seconds of Joel's voice, may open no
    socket but a local one
  '';
  noise-cancelled-trial-waits-out-a-call = ''
    Choose by Ear's refusals hold a-call-is-on whatever a machine adds to them, so it never
    records during a call
  '';
  nothing-names-its-own-look = ''
    no module but the theme's names a colour or a face
  '';
  nothing-spells-a-mark = ''
    no module but the theme's spells a mark's code point instead of naming it
  '';
  notices-holds-what-the-bus-says = ''
    the notices daemon judges each notice arriving by its sender's level and Quiet, keeps a thousand
    with none waiting dropped, tells a watch each as one record, says an act on the bus with the
    token first, never takes the name from another server, and keeps its history and ids over a
    restart; each notice's rows list its card's acts in its order, in three places each worded for
    its list, signed as the desktop knows its sender; and a notice read where it came from closes,
    saying where it was read, while one read here stays open; and one from a conversation with one
    open takes its place, folding it, read, and saying how many came in all; and a chat
    program's notice from a channel Joel hears every message of is asked of its daemon, a
    reply, an error, silence or no daemon judging it to him and raising it, unless read
  '';
  notices-passes-clippy = ''
    clippy finds nothing in the notices daemon or its tests
  '';
  notices-stand-on-the-cards-columns = ''
    a notice drawn on the one card, as the arrival line draws it, holds its ink the inset from
    the frame on every side, at 2 and at 1.5, with its mark centred on the title's capitals and
    its text column a mark's column past the inset
  '';
  notices-starts-in-each-account = ''
    each graphical account's session starts the notices daemon from its socket, the bus starts it
    for a notice sent while it's down, it restarts with no start limit, and no QML of the shell
    serves the notifications name besides; it reads the shell's levels, Quiet's file, its socket,
    the launcher's words and each program's mark
  '';
  explain-it-asks-only-what-a-cell-is = ''
    Explain It asks the realm's node only what one session is, by an address of capitals and
    digits, as JSON lines, and tells the quick model only its title, repository, machine,
    standing, doing, since and plan's latest and next, each flattened to one short line, never
    its id fields, quest, folders, trees, spend or networks; it says an unknown session apart
    from a node that can't say, and shows an answer only while it's the act's, fresh and whole
  '';
  explain-it-shows-its-own-answer = ''
    Explain It reads each session it's asked of from a stand-in node on its own, never taking one
    row's answer for another's, tells an unknown session from a node that can't say, gives up on
    a node that never answers, and shows its tool-less pi's answer only while it's its own
  '';
  now-lists-by-group = ''
    Now lists what wants Joel first, then what reaches other people, what's playing and the modes
    left on, by name within each, and a group it doesn't know last
  '';
  now-says-what-ails-the-home-network = ''
    Now's Home Network piece says the internet down while this machine finds it out from home,
    and otherwise only what eero says ails, worst first: the internet, the provider, the nodes'
    network and each node off line, and nothing while eero is out of reach
  '';
  now-says-what-ails-the-work-network = ''
    Now's Work Network piece says the link to work down, since when where the warp daemon saw it
    drop, degraded only once the daemon judges it held a minute, with how long, WARP's figures,
    the hour's spells and what lies behind it, connecting for long or WARP not answering, and
    nothing while it's well, Joel took it down himself or it's only coming up
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
  onepassword-gives-its-manager-each-release = ''
    1Password's manager for Passwords is given each op:// reference a secret ask may release, each
    after its own --release, and no other manager's reference
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
    in the moment it's turned on; started with a flag it doesn't know or one with no value, it
    refuses as usage and says nothing on its line
  '';
  onepassword-manager-takes-its-command = ''
    1Password's manager, run with the very command Passwords is given to start it, releases
    included, starts and says how it stands rather than refusing a word of it
  '';
  onepassword-offers-its-quick-acts = ''
    the account's 1Password entry offers Quick Access and then Lock 1Password, through switches the
    installed 1Password takes
  '';
  onepassword-opens-its-window-when-its-unit-starts-again = ''
    1Password opened from its entry runs once while its unit stays up, and once more when the unit
    restarted under the opening, as when the app dies making its window
  '';
  onepassword-outlasts-a-switch = ''
    a switch leaves the running 1Password be rather than restarting it, which would lock it
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
  only-polkit-may-take-a-yubikey-touch = ''
    a machine whose login names pam_u2f is refused as it's built, since a touch there lets someone
    in with no password, and a workstation with no key isn't
  '';
  open-with-passes-clippy = ''
    clippy finds nothing in the portal's chooser or its tests
  '';
  opened-programs-get-scopes-of-their-own = ''
    a program gio starts from its desktop entry gets a scope of its own in app.slice, named for
    the entry, or starts as it is with no systemd to reach, and a graphical machine names the
    wrapper that does it for GLib
  '';
  pain-points-names-what-slowed-him = ''
    pain-points counts what each launcher opening lost Joel, waiting past 400 ms to usable or past
    a frame's 100 ms, typing his way to a pick past his own median for it, or leaving a search
    still typed with nothing, not one he cleared to read the lists; joins an opening that came straight back to a failed search or the same row,
    ranks the day's worst, and keeps a pick's kind without what was copied, named or opened by path;
    reads pi's logs from where it stopped, or anew when another build kept them, counting only
    what Joel typed up to the call that hands
    his turn back, pi's queue and preparing past a second, a tool holding the session past pi's
    minute, a provider's error, a stopped turn or a message said again, and after a nudge only
    its first reply, never the model's thinking or the work he asked for, at most
    two minutes a go, and keeps nothing he typed or an agent wrote in the day file, its state or
    its line; and reads Chrome's history from its bytes in memory, asking only when each visit
    began and how, to count reloads a minute apart, leaving out a page reloading itself, and
    going back within seconds, keeping no page; and pain-points hear keeps what Joel said that
    hurt, or a turn that stalled him, with his words in its own 0700 state for a fortnight, and
    asks the session's own cell for a fix time, once, never the void or a cell the routes say
    never to, and one the void heard or an ended session's the least-held live holder of its
    subject through that subject's hub, by the session's purpose or his words, kept as a need,
    and the fallback only when no hub takes it, sending nothing without them, and exits failed
    when it couldn't route
  '';
  pain-points-passes-clippy = ''
    clippy finds nothing in pain-points or its tests
  '';
  passwords-completes-in-zsh = ''
    passwords completes in zsh with keys, help and keys' --json, and none of the commands that
    reached an entry or a secret before the daemon answered the shell alone
  '';
  passwords-copier-keeps-a-secret-in-memory-on-niri = ''
    on a nested niri, Passwords' copier, given a secret on its standard input as the daemon gives
    it, pastes it whole on Joel's display, offered marked secret, makes nothing under TMPDIR,
    runs nothing from PATH, typed or not, can't be read through /proc and holds as much of it
    in locked memory as the secret is long, says it's ready only once it can't be read and held
    once Joel's display has the copy, one paste then pasting it whole, pastes whole and keeps
    locked a secret that outgrows its first page, pastes whole a secret of exactly its cap of a
    MiB, refuses a standard input it can't read or one a byte past that cap, never saying it
    held either, and refuses to say ready or held for a copy that isn't sensitive
  '';
  passwords-holds-no-secret = ''
    the passwords daemon holds every entry each password manager lists without a secret, even one
    given with its secrets on, answers over its standard input and output as the shell runs it, a
    held list or get without asking a manager, gives a secret only to the ask for it, never to a
    watch, or to the clipboard marked sensitive and cleared when its time is up, writes it to the
    copier only once the copier says it's ready and answers it copied only once the copier says it
    holds it, answering a copy the copier refuses, before it's ready, partway through or once
    it has it all, with the copier's first line of reason, read no further than 4 KiB, or its
    own where the copier says none, ends the copy it holds as it ends itself, killed or not,
    refuses a copy before reading any secret while no copier is named, types a password only
    into the niri window it was asked for once the launcher has let go of the keys and while
    that window still has them, refusing a second while one is under way, never into one an
    agent's process is behind, above or below (pi, pi-spawner,
    mycelium, the hosted runtime, or the agents' or jobs' slices), nor a kitty whose foreground
    runs one or a multiplexer or that can't say, and writes one journal line for each type
    naming no secret, can't be read through /proc by the
    account's other processes nor write a core dump, says nothing of its own on the shell's pipe
    whether it ends well, fails or panics, ends once whatever ran it closes its end,
    refuses to hand a one-time code back, reading none, gives a one-time code's detail as its
    code and never its seed, says a manager signed out and gives no secret from it, tells a watch of entries as they arrive, and refuses an ask or a kind it doesn't know;
    serves each entry's row and drawer keyed by its place, in the account's own words and with no
    secret, rows spaced 1024 apart and one row moving told alone, a row with nothing to copy or
    open reading its details, and refuses rows where the words didn't read;
    audits every password only when Joel asks, refusing otherwise and for a manager signed out,
    counting the entries that share one and matching each hash's suffix against a stand-in Have I
    Been Pwned's range, padding counting nothing, sending it five characters alone and saying no
    secret in an answer or a log;
    keeps what an audit found in the state folder for Joel alone, holding no secret nor any hash,
    dresses the entries with it again once the daemon starts again, holds since when trouble was
    found until a check finds none, and tells a watch each change;
    answers a secret ask only for a reference listed to release, drawing it as a row of its own
    and refusing one not listed unseen, and with none listed draws none; releases the value only
    on Joel's yes on the ask's own key, for what was drawn and once it was shown long enough,
    refuses on a no, refuses a program outside the root-owned folders, and runs the asked command
    alone in a stand-in bubblewrap with the value masked in what it prints;
    and no passwords command reaches an entry or a secret, each refused as asked wrongly with
    nothing on stdout and nothing typed after it said back, a bare run and the help saying
    passwords are in the launcher, and the help fitting 60 columns with the exits it ends with
  '';
  passwords-lists-1passwords-ssh-keys = ''
    passwords keys lists the keys an SSH agent holds at 1Password's socket, each by its type, size,
    fingerprint, comment and public half, as a table under a head counting them, drawn whole into
    a pipe, and as JSON naming passwords.keys/1, no row for an agent holding none, and says how
    to turn the agent on when none listens, against a stand-in agent alone
  '';
  passwords-passes-clippy = ''
    clippy finds nothing in the passwords daemon or its tests
  '';
  passwords-runs-each-password-manager = ''
    on a machine with 1Password, the launcher's Passwords place runs the passwords daemon as the
    shell's own child with 1Password's manager through the system's op, with no unit and no
    socket, names no copier, stays off until agents are fenced, would type through wtype and ask
    kitty through kitten once on, and knows 1Password's program
  '';
  pause-all-agents-asks-once-on-a-second-return = ''
    in a nested niri, Pause All Agents is found by typing, arms on a first Return asking nothing,
    asks the safe word's daemon once on a second for a pause of every agent, and says in the foot
    what stopped, or a refusal in the vocabulary's words rather than the daemon's
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
    everyone watched, says a get nothing answered in time isn't ready, serves someone a source
    says the same of as it did before it was started again, and refuses an ask or a kind it
    doesn't know
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
    one tool, on pi's own plan and quick model by default, or through pi-claude on the Claude plan,
    and with no tools at all when started with --no-launcher
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
  pi-browses-in-chromium = ''
    a pi started from a shell on Linux is told to browse in Chromium, which no Chrome policy
    enrols, unless the machine names its own browser, which wins; a Mac is told of none
  '';
  pi-claude-keeps-to-the-plan = ''
    pi-claude starts pi with Claude Code's provider, on Sonnet unless a model is asked for, off the
    mesh, signed in to the plan's own folder and with nothing that would bill an API instead
  '';
  pi-harness-takes-its-settings = ''
    the harness's config takes a machine's declared settings at a switch, on Linux and a Mac, keeps
    every key already there, is left alone where none are declared, and a bad file doesn't stop it
  '';
  pi-keeps-closed-logs-compressed = ''
    pi's closed session logs are rewritten compressed each night in background.slice, catching up a
    night missed while off, over pi's sessions folder only where there is one, with no I/O class
    of their own and three hours before their clock ends them, and never on a Mac
  '';
  pi-keeps-its-own-settings = ''
    pi's settings take core's at a switch and keep pi's own keys; a file holding them isn't
    rewritten, a missing one is made, and one not JSON is refused untouched
  '';
  pi-keys-models-a-key-s-waits = ''
    pi-keys times a thread's bursts from its schedstat and stat, keeping the solid ones a key would
    wait out, models keys only over the time Joel was at a pi window, each waiting at niri, kitty
    and pi for the burst it lands in and then for the screen's next frame, counts a late key and
    the thread that lost it a frame, reads niri's focused window and frame, and finds the window's
    scope and pi as the jobs-sorter finds them, a bare class in the shared kitty finding none,
    and reads what a sample ran beside: the CPU's waiting, the load, and the pi and package
    versions a pi runs, keeping nothing else of its environment, and models the keys again by the
    install each window's pi loaded, so installs compare under one load
  '';
  pi-keys-outlasts-a-switch = ''
    a switch leaves a running pi-keys sample be rather than restarting it and waiting on its start,
    which outlasts home-manager's wait and sends the whole switch back
  '';
  pi-keys-passes-clippy = ''
    clippy finds nothing in pi-keys or its tests
  '';
  pi-carry-is-on-the-path = ''
    pi-carry, the harness's program mycelium carry runs to carry a quest to another machine and put it
    back, is on the path of every account with pi, run by the store's node
  '';
  pi-compiles-in-its-state = ''
    a pi started from a shell, on Linux and on a Mac, keeps node's compiled-code cache in the harness's
    state rather than the temporary folder, and the user manager names none, so pi hosts stay ones the
    node takes the word of
  '';
  pi-keeps-its-folder-private = ''
    pi's own folder and its session logs' folder are the account's alone on every machine with pi, so a
    token a tool printed into a log isn't readable by another account
  '';
  pi-leaves-out-the-built-in-codemode = ''
    pi's settings on Linux and on a Mac turn off the built-in codemode extension the harness's own
    codemode replaces, so pi starts without warning of it
  '';
  pi-loads-its-packages = ''
    pi's package list is the harness and neovim.pi on every machine, a package a machine adds comes
    after them rather than in their place, and pi is given each git package as its current link
  '';
  pi-log-compression-passes-over-other-file-systems = ''
    the nightly rewrite of pi's logs ends at once off btrfs, saying there's nothing to compress,
    rather than failing
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
    every five minutes the harness notes which windows here are joined to a session another machine
    holds, so the next boot can offer them back, in background.slice with the store's node, skipped
    where the harness isn't installed, and never on a Mac
  '';
  pi-state-moves-before-any-pi-starts = ''
    the harness's state that every pi reads as it starts, the mesh's presence, leases and runs, the
    quests' session records, the run ledgers, the work tool's records of its trees and a further
    realm's own presence, leases, runs and session records, is each given a subvolume of its own
    as the account's manager starts, before every pi host of every realm, the node, a further
    realm's own node and the folder's other writers, each by name, and with no host named too,
    while nothing else of the harness's moves, its worktrees and every realm's mesh messages
    included
  '';
  pi-sweeps-work-trees-in-a-unit-of-its-own = ''
    ended pi sessions' work trees are swept by one oneshot unit of their own at the top of each
    hour, catching up an hour missed while off, among agents' commands in jobs-work.slice, with
    the store's node, the account's git, its hosts' folders, provider modules as a machine gave
    them, only the agents' SSH proxy its hosts get and none of the variables a pi host's unit
    clears, nor bash's start-up variables, rather than on a host's thread; left running through
    a switch for up to two hours, started only once it has waited up to 15 minutes for the
    machine to have room, reading when the last sweep ended, a wait whose failure still starts
    it, made to give memory back past 6 GiB with no hard limit, its own, its
    slices' or its account manager's, to kill it at, skipped where the harness isn't installed,
    and never on a Mac
  '';
  pi-sweep-waits-for-room = ''
    the hourly sweep's room check lets it start at once when memory and I/O stalled little over
    five minutes and a quarter of memory is free, and when either stalled past its mark or memory
    is short waits, saying why, then lets it start as soon as the stall clears or its wait runs
    out, never skipping the hour; one whose last sweep ended too long ago for a wait to end
    before the hosts take over, or left no stamp, lets it start at once; one that can't read the
    machine, or reads a value that isn't a number, lets it start at once, naming what it
    couldn't read, and runs nothing it read
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
  pi-packages-keeps-its-own-store = ''
    pi's packages install their dependencies into a pnpm store of their own, and keep pnpm's
    metadata cache and node-gyp's headers, under the folder the agent fence holds read-only,
    never the account's one store or ~/.cache, which fenced commands write, as pnpm itself says of
    its store and cache
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
    commits waits for the host to end or continue it, once, across recovery; words the host's agent
    queued show as pending, never queued on the agent again, until a dequeue clears them
  '';
  pi-hosted-builds-boundary-context-when-read = ''
    pi built with the host's patches, applied exactly, builds a turn's boundary context only when a
    handler reads it, once a handler and afresh for the next, and builds entries a handler returns at
    once to check them, dropping ones that don't fit as before, where unpatched it built before the
    handlers and after each, read or not
  '';
  pi-hosted-freezes-context-without-a-set = ''
    pi built with the host's patches, applied exactly, freezes a context entry it reads and everything
    in it without putting any of its objects in a weak set, where unpatched every object went into one
    set for the process's life
  '';
  pi-hosted-keeps-the-patches-it-applied = ''
    pi built with the host's patches keeps a copy of every patch in host-patches under
    share/pi-host/patches, by its name and reading as the patch does, and nothing else, so
    cadence can tell from a running host's runtime which patches it loads
  '';
  pi-hosted-runs-its-own-turns-through-pi = ''
    pi built with the host's patches, applied exactly, runs work the host's agent starts itself as
    one of its own runs: busy from its start, before_agent_start told it carries on, the same prompt
    and sections a prompt's run sends with no change recorded, a handler's message held for the next
    prompt, and one agent_settled at its end; work inside a run already going joins it. After a
    restart that changed the sections, the run's first request carries them, sent once across its
    turns, a message sent as it starts lands after its first turn's tool results, and one held by a
    start that fails, or by work that fails or is aborted, waits for the next turn
  '';
  pi-hosted-loads-where-a-link-points = ''
    pi built with the host's patches, applied exactly, loads a linked extension through the file
    its link points at now, so once the link moves, a reload loads the new copy and its own helpers,
    still known by the link, whether the copy it first loaded is gone or still there
  '';
  pi-hosted-keeps-each-folders-extensions = ''
    pi built with the host's patches, applied exactly, keeps the extensions it loaded for each of
    the sixteen folders loaded most recently, two loading at once included, while a conversation
    there holds them, so another opening there reuses its modules, no two folders share one, a
    folder whose last conversation ended loads afresh and holds no place, and a reload loads afresh
  '';
  pi-hosted-shares-only-keyed-packages = ''
    pi built with the host's patches, applied exactly, evaluates a package declaring its per-session
    state keyed once across folders and keeps it once their conversations end, while a package
    without the declaration, one declaring something other than true and an extension in no package
    keep a copy for each folder, so a pairing made in one folder never reaches another; a reload,
    and a moved link, load afresh, and a load begun before a reload caches nothing
  '';
  pi-hosted-shares-keyed-modules = ''
    pi built with the host's patches, applied exactly, evaluates each module a package declaring
    its per-session state keyed reaches once for all of its extensions, while another keyed package
    and each extension of an unkeyed one evaluate their own, and puts none in Node's module cache;
    a reload or a moved link reads the files afresh, a load begun before a reload hands a later
    load nothing, loads take turns, a reload or a moved link lets go of loads queued behind one
    that never ends, which evaluate afresh what the package finished, keeping none it let go of
    for the copy it moved to, and a turn held too long lets loads go on without it, sharing only
    what the package finished and keeping every factory, until it ends and loads wait again;
    a failed load's retry evaluates afresh every module, however deep, that the failed load
    evaluated and none another extension finished, and is shared after; an import made as an
    extension runs is given only finished modules, so one that threw, or whose throw its load
    caught, is evaluated afresh by its own import or require again, a sibling's and a later load,
    a load and a late import racing each evaluate their own, an import during its own load shares
    what that load finished, is given part-way what it is still evaluating and fails with it, as
    does what a failed load's modules import after it, however deep, two late imports of one
    module at once share one evaluation, and a builtin, a data URL, JSON and a module jiti leaves
    to Node are jiti's own, evaluated no second time
  '';
  pi-hosted-keeps-each-conversations-keys = ''
    pi built with the host's patches, applied exactly, marks a model runtime that reads keys and
    headers through an auth context, gives each runtime the key, provider headers, model headers
    and OAuth headers its own context holds, never falls back to the process for one its context
    lacks, refuses header variables where it can't read the context, and reads the process for a
    runtime given none, as ordinary pi does
  '';
  pi-hosted-starts-conversations-lightly = ''
    pi built with the host's patches, applied exactly, runs an MCP server's header command without
    blocking and once for the connections opened within 30 seconds, runs one that resolved nothing
    again and once its cache is cleared, finds a tool on the path once and looks again for one
    missing, and hands a provider the store's own copy of its catalogue without a second clone
  '';
  pi-hosted-loads-extensions-lightly = ''
    pi built with the host's patches, applied exactly, sorts its alias map at most once however
    many modules two folders' extensions load, resolves pi-ai's longer and shorter specifiers as
    jiti's own order does, and evaluates each extension's modules afresh for each folder and after
    a reload
  '';
  pi-hosted-yields-between-extensions = ''
    pi built with the host's patches, applied exactly, gives the event loop a turn before it
    imports each extension and before it runs each factory, and still loads every extension in
    the order it was given
  '';
  pi-hosted-names-handlers-that-hold = ''
    pi built with the host's patches, applied exactly, writes one line to stderr for each
    extension whose handlers for one session event, or resources_discover, took 250 ms or
    more on its clock, and none at 249, each timed from its own start, its path quoted on one
    line, for session_start, a cancelling event, once, and resources_discover alike, a
    throwing handler too, two slow extensions on a line each, and none for a turn's streamed
    events though their handlers ran; a cancel still returns when stderr's write throws
  '';
  pi-hosted-screen-reaches-its-viewers = ''
    pi built with the host's patches, applied exactly, copies (a fullscreen selection too)
    through a hosted screen's viewers' terminals, sends a viewer who joins late the images
    already on the screen, and edits a draft, an extension's panel's too, in the external
    editor of the viewer that asked through its host, keeping the draft and saying why when
    the edit fails and refusing it in place when the host can't, and asks its host before a
    quit, at once, running on when the host says it doesn't hold
  '';
  pi-hosted-draws-before-it-binds = ''
    pi built with the host's patches, applied exactly, draws a hosted screen's conversation
    before its extensions' session start settles, then only what the bind added, or all of it
    again where the bind did more than add or added beside a call awaiting its result; a
    throwing early draw takes back only what it added and leaves the history unrepeated, and
    the conversation is drawn after the bind; it hears the agent only once they are bound,
    while plain pi still binds before it draws
  '';
  pi-hosted-screen-stays-in-its-session = ''
    pi built with the host's patches, applied exactly, ends only a hosted screen when it quits
    or crashes, suspends only the viewer that asked when its host can and refuses otherwise,
    binds nothing after quitting during startup, sends no more of its first words once it
    quits, builds nothing an extension asks of it once stopped and tells every screen of a
    theme change, while ordinary pi keeps its process exits
  '';
  pi-conversations-run-apart = ''
    each realm-kind pi host gets the template pi-conversation-KEY@.service its conversations'
    own processes start from, run by the host's program with --child for the host's key, its
    socket in the slot systemd names under the host's runtime folder, with the host's proof,
    node, frame flag and SSH agent, and the host's environment and the browser its account
    names in place of Chromium where the machine names one, each of those two with its % as
    written, but a plain pi's pools, its realm's conversation heap, its inspector shut and the
    variables that load code cleared, glibc's
    trim threshold fixed at 128 KiB beside its realm's own allocator tuning, unless the realm
    names its own threshold there or in glibc's older variable, whichever of the two neither
    the realm nor the trim sets dropped from the unit, the realm's tuning kept for its hosts too, in
    agents.slice slowed past 12 GB, no core kept, failing rather than skipped when the
    program is missing, never restarted or watched by systemd, forgotten once ended, given 60 s to park
    and told so, left running through a switch, its host's restart and the kernel killing one
    of its commands for memory, and a Mac gets none
  '';
  pi-conversations-read-says-what-it-read = ''
    what the pi host VM test's failures read from the machine is said on one line, blank lines
    dropped, an empty read and a failed one saying so with its exit, one cut short by its
    timeout naming its own seconds, and every character a terminal would act on but a newline
    written as its escape within its line, readable words kept as they are
  '';
  pi-conversations-set-heap-growth = ''
    a realm's conversationHeapGrowingPercent reaches each of its conversations' own processes,
    attended and background, as V8's --heap-growing-percent after their heap and before the
    harness's program, with the build proving the realm's node takes it, while its hosts and
    every process of a realm that names none keep V8's own growth, and 10 and 100 percent are
    taken while 9 and 101 are refused outright
  '';
  pi-conversations-refuse-a-node-without-growth = ''
    a realm whose node refuses --heap-growing-percent as a bad option fails the build its
    conversations' unit names, saying the realm, the percent, the node and what to do, as node 22
    and the tree's own node refuse an option they don't know, one whose node fails another way,
    never ends, ignores being told to end, is killed before its limit or can't be run, a package's
    folder among them, fails saying which and what to do, node 22, which pi is built with, takes
    it and the rendered unit files name that proof, and a machine cross-built for another platform
    has the proof stand unproven and warns of it unless its realm accepts that, where one built on
    its own platform doesn't
  '';
  pi-conversations-take-heap-growth = ''
    node 22, which pi is built with, and the tree's own node, each started with the word a realm's
    conversation is given for a growth of 70 percent, grow every heap limit they trace by a
    factor of 1.7, more than V8's slow modes ever give, while the same program without it grows
    by a factor of V8's own
  '';
  pi-hosts-start-with-the-session = ''
    each realm a machine names gets an attended and a background pi host, started with the
    account's session where the harness is installed, run by the host's pi and its own node, its
    inspector shut to SIGUSR1 and opened or undone by none of its options, started plainly or
    through env, with just the eight variables that load code into it, as
    pi-hosts-refuse-code-loading-variables names them, and the TPM agent's cleared, ready once it
    says so, in agents.slice with its realm's environment as written, a % and a quote in it
    included, and its folders first on its PATH, left
    stopped after refusing what it was given and left running through a switch and through the
    kernel killing one of its commands for memory, watched by a two-minute watchdog whose abort is followed by a kill 10 s
    on, given 20 minutes to start and 150 s to park its conversations
    when stopped and told that limit, its main process signalled first and what is left ended
    with it, keeping no core, given no conversation's allocator tuning, sixteen codemode workers and
    sixteen libuv threads for its conversations to share, Chromium as its browser, the account's
    home realm, node's heap limit set to its realm's heapMb,
    --experimental-async-context-frame after --disable-sigusr1 only where its realm asks for V8's
    frame, and pi's SSH agent proxy as their agent wherever the proxy runs, never given the TPM agent's own variable,
    started at most three times in ten minutes and said to be down once left down, never at a
    restart, a machine naming none gets none, and a Mac gets no systemd unit
  '';
  pi-hosts-ignore-sigusr1 = ''
    node 22, which pi is built with, and this tree's own node, each given the --disable-sigusr1 a
    pi host's unit gives it, outlive SIGUSR1 with their inspectors shut, beside a node without it
    whose inspector the same signal opens
  '';
  pi-hosts-keep-async-context = ''
    node 22, which pi is built with, started with --disable-sigusr1 and
    --experimental-async-context-frame as a host whose realm sets asyncContextFrame is, keeps fifty
    concurrent AsyncLocalStorage values across awaits, timers and promises others settle, while
    this tree's own node 24 refuses that flag and ends at once
  '';
  pi-hosts-restart-waits-on-calls = ''
    a pi host's unit names the probes its own restart waits out, as JSON the host reads: a call
    being on, always first, then what the machine adds, each with its words and its command
  '';
  pi-hosts-refuse-an-unproven-runtime = ''
    a realm's pi reaches its hosts only with the proof that pnpm test:host passed on the runtime
    it is in; a proof of another runtime, or one that isn't a proof, fails evaluation, naming the
    realm and how to prove it
  '';
  pi-hosts-refuse-code-loading-variables = ''
    a realm that gives its pi hosts any of NODE_OPTIONS, NODE_PATH, NODE_COMPILE_CACHE,
    LD_PRELOAD, LD_AUDIT, LD_LIBRARY_PATH, OPENSSL_CONF or OPENSSL_MODULES, which their unit
    clears so none opens the host's inspector or runs code as the host, fails evaluation naming
    the realm and only the ones it gave, joined as written and spoken of as it or each, then
    names each of the eight again where it says how they load code, the options a conversation's
    own node takes instead, and that a command needing one sets it itself, rather than having
    them dropped unseen, while a variable that loads nothing passes beside them unnamed
  '';
  pi-rings-low = ''
    the account's ding rings loud for seconds, as the harness's done chime plays it, and the
    shell's want sound rings loud, both under the theme message's pitch
  '';
  pi-rings-when-a-run-is-done = ''
    pi's done chime is told a ding and the store's pw-play, and no voice or want sound, on Linux,
    with Do Not Disturb's file holding it while it's on, and a Mac is told nothing, ringing with
    its own
  '';
  pineapple-github-starts-with-the-sign-in = ''
    the reader of GitHub for the safe word starts at login, reads Joel's GitHub sign-in through gh,
    comes back after a switch and a failure, runs in the background slice, reads as its own name
    in ps and runs live
  '';
  pineapple-judges-each-saying = ''
    every saying in sayings.json is judged the safe word, a resume, an ask to stop or a near miss as
    the table says, and reaches what its words and its place name, or everything, said so; and
    Joel's own command resumes a stop only out of pi's reach, marking it rather than removing it;
    and its daemon serves only live stops to the launcher, taking a press only from Joel's shell;
    and his stop all, refused under pi, freezes agents' work and timers here but never his own,
    and parks every session and refuses new ones on each machine running one, until a resume
    undoes exactly that
  '';
  pineapple-passes-clippy = ''
    clippy finds nothing in pineapple, the judge of the safe word that stops Joel's agents
  '';
  pineapple-reads-each-site-named = ''
    each other site a machine names for the safe word is read by its own unit, searching the
    repositories named for what Joel opened with that site's sign-in, in shadow until the machine
    says it's live; none is read unnamed, and a name GitHub's or Slack's reader has is refused
  '';
  pineapple-serves-the-launcher-from-its-socket = ''
    the daemon showing the safe word's stops in the launcher starts at login on its socket, where
    the launcher looks, knocks with notify-send and reads its words from the launcher's vocabulary,
    with a pause plan naming the agents' timers and keeping the compile cache
  '';
  pineapple-slack-goes-live-through-a-gateway = ''
    a machine saying its tool gateway's reader is live runs it as the Slack reader for the safe
    word, live, given the gateway and its sign-in rather than pi's keyring, its URL's percent
    signs reaching systemd as written, still taking its row in For You with it as it stops, with
    no shadow reader beside it
  '';
  pineapple-slack-hears-a-gateway-in-shadow = ''
    a machine naming a tool gateway hears Slack through it with a second reader for the safe word,
    started as Slack's is but only ever in shadow, leaving the environment its sign-in reads as it
    was, its URL's percent signs reaching systemd as written; a gateway reached over plain http is
    refused, the keyring's reader stays pineapple-slack beside it, and a machine naming none has
    no such reader
  '';
  pineapple-slack-starts-with-the-sign-in = ''
    the reader of Slack for the safe word starts at login, reads pi's Slack sign-in from the
    keyring, comes back after a switch and a failure, runs in the background slice, reads as
    its own name in ps, runs live and takes its row in For You with it as it stops
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
  pi-serves-the-launchers-agent-actions = ''
    the harness serves the launcher's AI actions on daemons/pi-verbs.sock, held by systemd for the
    login alone and named pi-verbs as daemons are, restarted by a switch but not after it rests,
    in background.slice with the store's node through pi-verbs on the path, skipped where the
    harness isn't installed, and never on a Mac
  '';
  pi-tells-where-as-the-prompt-does = ''
    pi's status line finds the folder by the prompt's roots, asks the prompt's git-engine, and marks
    git with the theme's glyphs
  '';
  pi-wears-the-theme = ''
    pi's text, strings and deepest thinking are the account's text, string and alert colours, and
    every colour it names is hex
  '';
  pixels-probe-holds-every-pixel = ''
    the web-design skill's pixels probe, and its words and names probes beside it, in headless
    Chromium at a phone's width, names a page wider than its window and what reaches past it
    while passing a part in its own scroller, a part arriving over the words that shifts the
    page past 0.1 while passing one whose box was kept, text under 12 px, scaled or
    transformed, a drawing's label under 12 px unless marked data-enlarges and under 9 px even
    then, as the drawing's scale sets it, while passing words for the ear alone or a style
    hides, and a control whose name doesn't start with the words it shows or that shows a mark
    alone, labelled or not, while passing a keycap named for its key, a keycap its line of keys
    says the work of, and a picture named by its label, and words under 4.5:1 on the ground
    under them, or 3:1 at 24 px or bold at 18.66, faded as the browser composites each
    opacity, on a dark page's canvas, its style's or its head's, with each covering or tiled
    gradient layer at its worst colour between its stops but for a hard one, and a placeholder
    or a style's words in their own fill, giving the faintest and how many more, and words on
    a picture, through a filter, a blend, a mask or a stroke, drawn by a shadow alone, or over
    too many grounds named
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
    root-owned file, only past the gate saying the person is in, with its helper let reach the key,
    and no other login does; with none named, polkit asks for the password alone
  '';
  polkits-prompt-says-what-runs = ''
    polkit's prompt names the command asking, its store paths without their hashes and a long one
    cut at its middle, shown as written, and the account run0 or pkexec would run it as, and the
    journal says each ask as it begins and how it ended, after how long and with what keys
  '';
  processor-telemetry-passes-clippy = ''
    clippy finds nothing in the processor telemetry reader or its tests
  '';
  processor-telemetry-reads-each-region = ''
    processor-telemetry finds each telemetry region by its GUID and no other processor's, names
    each cluster by its CPUs, reads a counter across its wrap, and says which part kept the package
    out of its deep idle states longest
  '';
  programs-holds-whats-installed = ''
    the programs daemon says hello on the socket systemd hands it, says an entry whole from the
    nearest data folder with each file kind's globs and description, says an entry changed again
    and one removed as gone, reads a program's list as its watch begins and as its file changes and
    not while nobody watches, says a list whose command fails as empty and why, reads a list of
    folders itself with no command, one not there holding nothing, and again as a file comes in
    one, reads a folder named by its key as its watch begins and as a file comes in it, one not
    there holding nothing, serves the repositories under a folder four folders deep, past no
    hidden one, each with its page on GitHub, and one's files as git lists them, hidden ones but
    not what it ignores, each within its folder, as each is asked, and the repositories again as
    one is cloned or its git writes while watched, reads when zoxide's database saw each
    folder, ssh's config and known hosts, and the ssh lines of zsh's history as each is asked and
    again as its file is written while watched, no other file beside it, reads what a program
    has open as it's asked, and a list its program tells of again only once told it changed, or as
    each ask comes while it names a process that never told, asks what a file shows of one
    file-preview it keeps, again only once the file changed, says a file that file-preview doesn't
    answer within its deadline as not answering, stopping it and what it waits on, and goes on to
    the next, serves Programs' rows open first, counting the windows the windows daemon says each
    has open, Joel's apart from the agents', and going to his latest, and again as those change,
    serves kitty's open list without the panes the agents daemon names, again as it tells,
    serves Chrome's open pages as rows of their own, named by their titles and said by their
    sites, and again as Chrome writes its session, finds Programs' rows for what's typed by
    name, keyword, comment and what their binds do, best first with the letters matched, finds
    what was taken by name, newest first, past Recent's nine and no more than asked, with the rows
    it never served and each served whole, unplaced, once it's ahead, counts the changes programs
    tell of in its health, follows a data folder through each link a switch
    replaces, and refuses an ask, a kind or a place to search it doesn't know
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
  quests-makes-the-launchers-rows = ''
    every quest open and ended is a row in Quests' order, saying its kind, its priority or the day
    it ended, and the session holding it, with how many are open and driving, and a change tells a
    watch only the rows that read differently
  '';
  quests-passes-clippy = ''
    clippy finds nothing in the quests daemon or its tests
  '';
  quests-runs-in-each-account = ''
    the quests daemon runs in each account of a machine in a realm, following its node, its socket
    systemd's and restarted by a switch, and in none without a realm
  '';
  quickshell-allocates-from-one-arena = ''
    the account's Quickshell unit holds jemalloc to one arena, rather than one for each thread
  '';
  quickshell-compiles-every-file = ''
    Qt's own compiler compiles every file of the account's shell and the greeter without refusing or
    crashing
  '';
  quickshell-draws-each-window-on-one-context = ''
    the account's Quickshell unit tells Qt to make no decorations context beside each window's own,
    by a name the Qt it runs on still holds whole
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
    a switch that changes a file the shell reads leaves it running, and every switch starts the
    companion that hands it over onto its files once the launcher is idle
  '';
  quickshell-takes-its-shape-from-the-theme = ''
    every corner and border in the shell and in each module's surface is the theme's
  '';
  quickshell-writes-no-perf-map = ''
    the account's Quickshell unit sets Qt no perf map of the JavaScript it compiles, and unsets one
    the login's environment would give it
  '';
  reader-builds-again-only-for-the-pages-keys = ''
    a new act in the key language for lists leaves the reader's build as it was, and one for pages
    changes it
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
    read as Neovim reads it, gO's outline, the window's title from the document's title or else
    its file's name, code in a table's column counted by the stretches Typst breaks it into, math
    in a cell measured as its words, a place or a mark landing as far through its block at another
    width and on a line of its own block, kept marks waiting on the first screen for their block's
    height and written back as read, landed again from what was kept unless the file was edited,
    one kept without a height taking its block's, a set saying the file was edited only when its
    words changed, the first width from the window kept, and the first screen setting the block
    after a long one
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
  ready-rows-keeps-each-rows-place = ''
    a row moved among a daemon's ready rows takes an order midway and the rest keep theirs, a
    making is told only what reads differently and what went, names go A to Z with case alike
    and numbers by their worth, each figure is said as numbers.json's cases say, and every text
    matches every query, scored and marked, as fuzzy.json, made by the shell's Fuzzy.js, says
  '';
  ready-rows-passes-clippy = ''
    clippy finds nothing in ready-rows, the form every daemon serves the launcher's rows in
  '';
  receptor-say-gives-mycelium-words-and-numbers-alone = ''
    a receptor is said by running mycelium's absolute path with fixed words, a realm of
    mycelium's letters and counts held to the most it takes, with any words its caller adds
    put after its own and given as built; said off, each count is none; and only the account
    or login declared, ASCII case aside, is taken as the source's own
  '';
  receptor-say-passes-clippy = ''
    clippy finds nothing in receptor-say, how the daemons say mycelium's receptors
  '';
  record-window-passes-clippy = ''
    clippy finds nothing in record-window or its tests
  '';
  releases-wait-for-the-passwords-place = ''
    a release listed while the launcher's Passwords place is off leaves Asks out of the launcher,
    and the machine is warned that releases need the place on; once it is on, Asks is drawn over
    the place's daemon with the clock its yes is timed by, and nothing warns
  '';
  reopening-finds-each-screen = ''
    Reopen Them All puts each session back on its own screen, found by its maker, model and serial
    rather than its connector, and on the one it was kept on of two alike, a gone screen's
    workspaces after the one in use's and then to it, each to its rank, once it's back, and each
    window that reopens itself by its program and title, beside the windows kept with it, under
    the one kept above it in its column, and a window already where it goes left there; and those
    that orchestrate are begun first, the next session only while the machine queue says the
    memory and the processors have room and the disk isn't stalling, a queue unread or gone quiet
    holding nothing back; and a Reopen Them All list counts what it finds gone in the walk's tally
    while the walk may still find one of that tally's folders gone, else in one of its own, as
    once only a window given up on or the harness's sessions keep the walk, each session once by
    the id it was asked by, whichever list queued it, and told again as a list joins a tally
    already told of one gone
  '';
  repos-is-told-every-realms-owners = ''
    the refresh's unit gives repos every realm's declared owners as one word that systemd passes
    whole, on a machine declaring two realms that list one owner in both, and runs a repos that
    sets the declared login outright, so what a refresh keeps as asked of him is his
  '';
  repos-passes-clippy = ''
    clippy finds nothing in repos or its tests
  '';
  repos-reads-the-disk = ''
    checkouts and trees laid out as git lays them out, a reftable's included, read as one
    repository for each remote with each checkout's trees and heads, its trunk and when it was
    last written, a tree whose folder is gone said so, each checkout's store weighed as git's
    auto-gc samples it and due as its config sets, over stores laid out by hand and one git wrote,
    every view of them fitting each width, and the GitHub receptor said with its counts, each
    change asking review as owner/name#N of a declared owner and the realm's owners, none of
    them while a read failed, and the daemon's homes for Code, every repository and present tree
    at its Code address by key whatever the reading's order, written whole and readable by the
    server through no link left in its way, not again while the file holds them, put right once
    another writer changes or removes them or puts a link or a pipe there, which never holds the
    daemon, and tried again after a failure it warns of whenever its reason changes, naming the
    hidden file where that failed, and says it's over, the running daemon writing them after it's
    ready and again as a tree is cut, serving on through a failure, and writing none and saying
    nothing of them when none are named
  '';
  resolved-is-set-per-network = ''
    a laptop leaves resolved's global DNS settings as they ship
  '';
  rhythm-learns-when-joel-stops = ''
    rhythm finds each night's stop at the last busy minute before four hours' sleep, never at
    dinner, lunch or a stray key, makes four weeks' middle usual with each weekday's own habit,
    the latest nights weighing most so a change shows within a week, guesses midnight and says so
    with under a week of nights unless the four weeks before a break hold enough, keeps a year of
    nights past the minutes and, once they span two months and an hour and a half of daylight,
    leans tonight's stop with the day's length by at most an hour, reads only the times Joel
    typed from zsh's history and pi's logs, each seeded night stopping at its last, and says it
    in plain words and its profile file; and it suggests a walk only in daylight after a long
    stretch at the machine, an hour and a half apart, at most three a day and one in poor
    weather, saying the weather and the sunset
  '';
  rhythm-passes-clippy = ''
    clippy finds nothing in rhythm or its tests
  '';
  run-subtests-keeps-what-is-named = ''
    run-subtests runs a VM test whole with pytest picking the tests named, refusing a word pytest
    would read as more and a script that doesn't hand its tests to pytest, refuses a check that
    isn't a VM test rather than letting the quarantine call it a pass, and
    lets a test's entries naming subtests excuse only a run whose every failed test they name
    between them, saying each one's owner
  '';
  rust-programs-pass-clippy = ''
    clippy finds nothing in the single-file programs or their tests
  '';
  sccache-shares-builds-across-worktrees = ''
    sccache's server runs among the jobs, started as its unit starts it, and two workspaces in
    folders of their own build one dependency through what an account's shells and user manager
    are given, finding that server's socket, and the second's compile of it comes from the cache
  '';
  screen-picture-follows-its-screen = ''
    the wallpaper's picture is read once at its screen's pixels, let go when its screen goes and
    read once again when it comes back, as screens come and go, and Quickshell warns of nothing,
    no binding loop among it
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
  sd-switch-passes-its-tests = ''
    the patched sd-switch builds and passes its own tests
  '';
  se-keys-builds-for-any-name = ''
    the Mac's key maker builds, shellcheck and all, for a name with an apostrophe
  '';
  searches-leave-other-programs-data-alone = ''
    a Mac's home folder holds an ignore file, and ripgrep and fd searching from it or from Library,
    run there or given its path, never open other programs' data, the cloud's folders or the
    Desktop, Documents and Downloads folders macOS asked about, while still finding the rest, a
    project's folder named like a guarded one included, and searching a guarded folder named or
    started in; Linux holds no such file
  '';
  secret-asks-keeps-the-item-to-its-yes = ''
    a yes releases the one item to the checker alone, which says its length and never the value;
    an ask for another item reads nothing and draws no row; a yes for a line that changed since it
    was drawn is refused and drawn again; a yes while locked reads nothing; a yes past the ask's
    wait ends it as unanswered and tells its watchers; and a no refuses
  '';
  secret-asks-passes-clippy = ''
    clippy finds nothing in the secret-asks daemon or its tests
  '';
  secret-asks-runs-with-its-item = ''
    the secret-asks daemon runs on its socket with the item the machine names, read through op's
    wrapper, and a switch restarts it
  '';
  session-backup-passes-clippy = ''
    clippy finds nothing in session-backup or its tests
  '';
  session-backup-sends-each-log-once = ''
    session-backup only adds to its one folder, sealed copies that open, carrying on where a
    stopped run left off, and removes an old log from the machine only once Drive still holds
    its copy
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
    slice above them carries one hard and one soft floor, the same at each; session.slice and
    app.slice keep what they swap out compressed in memory, never writing it to disk
  '';
  sessions-come-back-after-a-reboot = ''
    after a new boot or console login, sessions-come-back starts each pi session held before it
    again on the machine, in its own folder, as the pi it ran as with --session and its id, once;
    one that ended while the machine ran, or whose pi died, stays ended; a hold it can't read
    plainly is passed over and said once; the machine is the one most sessions name; a node
    silent past its limit is stopped with all it started; a record it can't read is set aside
    and said, one kept more than four days before brings nothing back, and one is kept before
    any start; one whose folder isn't there or that the node refuses is tried again fifteen
    times, one it gave no answer for waits five minutes first, and a new boot gives each its
    tries again; a realm's .off file keeps nothing and brings nothing back, then or after; and a
    run beside another leaves the realm to it
  '';
  sessions-come-back-passes-clippy = ''
    clippy finds nothing in sessions-come-back or its tests
  '';
  sessions-sit-beside-the-programs = ''
    agents' sessions have a slice beside Joel's programs, which outweigh it ten times over on the
    processor and the disk, and the sorter watches both
  '';
  agents-work-keeps-off-the-performance-cores = ''
    a machine naming its efficient cores holds agents' work and builds to them, not Joel's own
    terminal or timed work or background, handing the account's manager the cpuset controller to
    do it; one naming none holds nothing, nor does the laptop's hardware without the slices
  '';
  a-unit-opens-as-many-files-as-a-shell = ''
    a unit the account's manager starts, a transient one included, may open as many files as a
    login shell unless it sets its own limit, as mycelium and quickshell do: by default its soft
    limit is its hard one, and the hard one stays at the manager's 524288
  '';
  a-probe-waits-beside-the-programs = ''
    the sorter's probe runs in a slice of its own at Joel's programs' weight from login, so its
    pressure reads what the rest of the machine makes one of them wait
  '';
  shared-work-ranks-with-the-sessions = ''
    no slice inside the account holds a disk target, and work other sessions wait on has a
    slice beside jobs, below Joel's programs on the processor, with a memory ceiling, named in
    SHARED_SLICE
  '';
  shaper-finds-the-rate = ''
    the shaper reads sets, routes and queues from what the kernel prints; counts what every link to
    the same router carried, and what a follower's queues sent, dropped and held, on its leader's
    line; replays recorded days as one stay per network across midnight, each
    second's carried rate from the counters and each direction's own delay voted from its
    reflectors, and names a cut with no queue of its own, a rate held low on an idle connection and
    one starved under a full one; the one-way controller cuts a direction only once its own delay
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
  shaper-hears-a-keyed-house-from-any-address = ''
    a keyed house naming no address, whose reports its key alone says are its, opens the port the
    shaper hears them on to every private IPv4 address, in one rule naming no other, and builds,
    while a house naming neither an address nor a key is refused as the machine builds
  '';
  shaper-hears-only-the-house = ''
    a network's house opens the port the shaper hears its reports on, the one src/house.rs names,
    to the house's addresses alone, one written alone or a list, over IPv4 or IPv6 as each is
    written: the input rules are the houses' three alone, and no port, range or trusted interface
    opens it to all; and a house's address, alone or in a list, is held to well-formed IPv4 or
    IPv6, an IPv4 one written as IPv6, mapped or compatible, refused in each spelling tried
  '';
  shaper-refuses-a-house-key-it-can-t-read = ''
    a house's key file that the shaper's service can't read, under /root, /home, /tmp or /var/tmp,
    or spelled through doubled slashes or dot folders, is refused as the machine builds, and so are
    two houses naming one address under two keys, while a key under /var/lib, and a keyed house
    beside a keyless one naming the same address, are taken
  '';
  shaper-passes-clippy = ''
    clippy finds nothing in the shaper or its tests
  '';
  shaper-starts-when-asked = ''
    the shaper starts at boot only on a machine that names its controller, so a machine that
    imports the module and never asked stays unshaped, while its recorder starts either way
  '';
  shaper-stays-through-a-switch = ''
    a switch starts the shaper and its recorder again in place, never stopping them first, so the
    queues keep their counts and the record has no hole around the switch
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
  sign-in-program-never-waits-for-good = ''
    a sign-in program's token is taken trimmed, slow, ending after its output or once a hold
    writing it is let go; one failing, printing nothing, missing, printing what isn't text or
    past 64 KiB is none; one hanging, lingering or printing for good runs over within its
    patience, killed with what it started
  '';
  sign-in-program-passes-clippy = ''
    clippy finds nothing in sign-in-program, which runs a reader's sign-in, or its tests
  '';
  slack-finds-who-it-knows = ''
    slack-people says who Slack knows by an email address with their time zone, presence and
    status, on a grid by a search their profile confirms, and one it knows nobody by; finds each
    once with the harness's sign-in, keeping who it found across a restart, and asks how they
    are each time, only ever reading; finds their conversation with Joel in plain words, only
    what's in a direct message with them, newest last; asks ahead only after a step-in, finding
    their conversation once and never how they are; handed the launcher's words, says their
    drawer's facts, Write in Slack and their conversation's row and drawer ready, or as a
    fact that one it knows nobody by is not in Slack; and asks nobody once Slack refuses the
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
    Slack too busy apart from signed out or out of reach; and says whether a channel's notice
    was a post there or a thread's reply, by its words and time in the channel's history, and
    that it knows no channel by a name not yet held
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
    slack-unreads starts at login, reads pi's sign-in from the keyring, restarts as the new one after a switch,
    is restarted when its watchdog hears nothing, is handed the launcher's vocabulary for its
    rows, and takes its ask in For You with it however it stops,
    while its socket, systemd's and held only once pi has signed in, stays
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
    the launcher's Sound lists the rows the sound daemon serves, in its order, each stepping into
    its device's parts, made once as wanted and again only as its record changes, and never
    listens to a headset's microphone while it plays music; a device's drawer
    says where it is and which way it goes, its state, its volume in decibels, its parts, how it
    keeps time while it runs (dropouts warned past none, load to three significant digits from
    its busy time over the cycle or warned as over 100% where it didn't finish, a follower in
    its driver's cycle), its format, the kernel's facts, and its priority and identifiers last,
    never whether it's muted nor its level; and channels at one level are balanced, a stereo
    pair's otherwise named without front; a device's row served anew remakes only its own row;
    and the sound daemon down, its one row's Return starts it again
  '';
  sound-hears-pipewires-devices = ''
    the sound daemon makes each output and input PipeWire says a device and leaves out a program's
    stream, says a volume as the shell shows it, keeps it through a mute alone, marks chosen the
    ones the default metadata names, lets go of a node gone, reads each channel's volume and the
    mute from a Props, and with no sound server says PipeWire isn't answering, answers a list and
    a watch with no devices rather than waiting, and refuses an ask or a kind it doesn't know
  '';
  sound-passes-clippy = ''
    clippy finds nothing in the sound daemon or its tests
  '';
  sound-runs-beside-the-shell = ''
    the shell brings the sound daemon, on the daemons' socket, which systemd holds whether or not
    it runs, restarted by a switch
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
  spend-watch-counts-each-budget-apart = ''
    spend-watch counts each cap from its period's start against its own budget's spend, names a
    burning hour's session, and tells a finding once, never a cap that flickers
  '';
  spend-watch-passes-clippy = ''
    clippy finds nothing in spend-watch or its tests
  '';
  spend-watch-stays-in-the-background = ''
    the AI spend watch yields to the account, comes back when it fails, and leaves its status behind
    when it stops
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
  spotify-player-speaks-vims-keys = ''
    spotify-player takes the key language's Ctrl+e, Ctrl+y and Ctrl+o beside its own Vim keys
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
  spotify-speaks-vims-keys = ''
    Spotify's page scrolls on Vim's j, k, Ctrl+e, Ctrl+y, Ctrl+d, Ctrl+u, Ctrl+f, Ctrl+b, gg and G
    as Vim scrolls, goes back and forward a page on Ctrl+o and Ctrl+i, and opens its filter on /,
    leaving Escape, Alt, a field and a dialog to Spotify
  '';
  ssh-tpm-agent-refuses-agents = ''
    the TPM's SSH agent refuses a process where agents' work runs, keeps no core of its PIN, and
    its source, beside the gate, passes each case the gate must decide; each slice the gate names
    is one a machine with a screen makes, awaited is the one service the example laptop puts in its
    awaited slice, zsh's own text leaves a shell the agent it was given, and bash's points the
    login's services at the TPM's
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
  subject-hubs-passes-clippy = ''
    clippy finds nothing in subject-hubs, the subjects' hubs and needs cadence and pain-points
    share, or its tests
  '';
  subject-hubs-reads-hubs-and-keeps-needs = ''
    the hubs a mycelium node lists are read from the line with the list's id, one or several a
    subject, none from a node that holds no hubs and an error from any other refusal, a record
    that can't be read or a subject that isn't a plain name left out; the narrowest subject
    whose every word is said, of a live hub, takes a thing first; and each need put to a hub is
    kept once in its UTC day's file of the mesh's needs, in the shape the harness keeps its own
  '';
  subtests-read-no-word-as-an-option = ''
    the build of a VM test's subtests hands run-subtests the check and its words past --, in
    order, so neither is read as an option
  '';
  sun-passes-clippy = ''
    clippy finds nothing in sun, the sun's reckoning the calendars and night light share, or its
    tests
  '';
  surface-clock-passes-clippy = ''
    clippy finds nothing in surface-clock or its tests
  '';
  svgtext-probe-measures-a-drawings-words = ''
    the web-design skill's drawing words probe, in headless Chromium over small pages of its own,
    names light words on a shape whose fill fell back to paper in the dark look, as the forage
    research page's diagram did on 15 November 2026, faint words on the page alone and dark words
    faded by their group, holds its bar of 7:1 from both sides, passes words on a shape filled with
    a colour the page defines and strong words, and names a drawing kept in an image 65 px wide
    while passing one 64 px wide
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
  system-accounts-start-no-persons-services = ''
    every user unit core starts by itself in every account's manager says whose it is, so a
    system account's lingering manager starts none of a person's services
  '';
  system-judges-each-part = ''
    the whole system said from what each part's owner answers, mycelium, net and repos, each
    part good, watch or bad by its own rules and the whole by its worst, an owner that fails
    leaving its part not read with why, and every line fitting each width
  '';
  system-passes-clippy = ''
    clippy finds nothing in system or its tests
  '';
  system-reads-its-machines-realm = ''
    the system daemon's unit names the realm its account's sessions are in, on Linux and on a Mac
    running nodes for two realms, where mycelium refuses a command naming none, and on a machine in
    no realm names none
  '';
  system-serves-where-its-node-reads = ''
    the system daemon serves where its machine's mycelium node reads how it stands: on Linux
    the socket systemd holds in the runtime folder, and on a Mac a launchd agent, in the
    background and started again, listening in the account's state folder and finding the
    account's commands as its login shell does
  '';
  system-site-holds = ''
    the system page typechecks on the kit at its strictest and builds, its tests passing: the
    parts in the summary's order with their machines, a part or machine not told said rather than
    drawn, the whole said worst first, and a record's words reaching the page as text only
  '';
  system-site-reads-live = ''
    the system page, served by the local sites' server beside a stand-in system daemon, reads live
    in headless Chromium: every part drawn and the whole counted, a part turning and a machine
    going, broken while the daemon is away and live once it's back, stale once the server goes, the
    web-design skill's page checks passing at a phone's width and a desk's in both looks, a look
    for a machine with a spot machine's long name leaving its words at least two fifths of the
    list at both widths and none past the window, a machine's look saying the daemon's why under
    its words with its one act opening that machine's launcher row on a thumb-sized target at both
    widths, a good part read from its owner's kept answer
    never listed as worth a look and its note said where it belongs, a first visit within its
    budget of files, bytes and elements, both visits reaching only the sites' server and the second
    answered unchanged, and nothing logged
  '';
  forage-feed-passes-what-its-page-reads = ''
    the node's feed, as a machine in a realm declares it, passes every field the forage page reads
    off a forage record, so nothing the page draws goes missing live
  '';
  forage-site-holds = ''
    the forage page typechecks on the kit at its strictest and builds, its tests passing: each
    cell's forage under the cell that started it, even in a ring, plans first with their steps
    added up, how many cells have no purpose written, what each quest waits on its person for, a
    record it can't read said, not dropped, and a floor past 48 cells drawn around the open cell
    with the cells it started and every cell each grew from, kept on its cell through records,
    and a floor wider for its gaps drawing its words no smaller than site.css's cut allows, which
    grows them by as much
  '';
  forage-site-reads-live = ''
    the forage page, served by the local sites' server beside a stand-in node, reads live in
    headless Chromium: every cell drawn in order, a started or split cell under the one it grew
    from, what waits on its person and the plan furthest to go said in its lede, each cell's plans,
    quest, needs and split in its panel and a need as a dotted arc, its keys stepping the cursor, a
    purpose written followed and drawn as words though it holds markup, a cell a purpose takes out
    of the view of cells without purpose taking the cursor with it, a wait answered no longer
    said, a cell going and the crest drawn again once every cell has, broken while the node is
    away with its cells kept and live with them read again once it's back, stale once the server
    goes with its cells kept, a first visit holding still at a phone's width, a desk's and a
    wide desk's in a room for its floor, drawing its machine's name at the wide desk's and no
    word under 12 px, and a long link, open in its panel, never
    widening the page, a first visit within its budget and a second answered unchanged, the
    web-design skill's page checks passing at a phone's
    width and a desk's in both looks, nothing logged but what a test causes, and on 60 cells the
    floor following the cursor to a booked start past the 48th, drawn solid, not drawn again on a
    step it holds, kept on its cells through a find, one matching nothing keeping the cursor and
    the floor through its end, a record on it drawn with it kept, drawn again on a step off it
    over the list the record left, and during a find matching nothing a link naming the cursor's
    cell in another form bringing its row back and that cell ending leaving a floor drawn around
    another where it was
  '';
  murmurs-site-holds = ''
    the murmurs page typechecks on the kit at its strictest and builds, its tests passing: murmurs
    newest first under their quests' titles, ties without the writer's own, how one settled, the
    sentence's turns, the quote cut on a word, and a record it can't read said, not dropped
  '';
  murmurs-site-reads-live = ''
    the murmurs page, served by the local sites' server beside a stand-in node through the node feed
    as home.nix declares it for a machine in a realm, reads live in headless Chromium: every murmur
    drawn newest first and counted, one said, settled or gone, a quiet node said in a line with no
    empty list or bare sky, again once its last murmur goes and never while a record goes unread,
    a node the server never reached said unheard and never quiet, in a second tab too,
    one it can't read said while the rest draw, broken while the node is away with the murmurs
    kept and live once it's back, stale once the server goes with them kept, a murmur's words
    drawn as words and never as markup, j, k, g, G and ? answering, a tap on a key its head shows
    doing what typing it does, the web-design skill's page checks passing at a phone's width and a
    desk's in both looks, a first visit within its budget of files, bytes and elements, both visits
    reaching only the sites' server and the second answered unchanged, a tab whose renderer dies
    before its first reading opened once more and said, and nothing logged
  '';
  start-new-work-asks-only-a-sprout = ''
    Start New Work asks the realm's node only a sprout of the trimmed words or link, after --,
    with -o json and never --join, in a whole folder from the open quests' repositories, home in
    any spelling and repeats left out; it says why when it asks nothing, hears both streams before
    saying it started on the node's machine, else the node's or sprout's why; and the words a
    mycelium: link offers are read again as one plain line of at most 1000 characters, its session
    kept only where one here has that address, named by its quest, the foot saying a page offered
    it; a slow start is said still coming up a minute after its answer, then told by name when
    it's up or lost within a quarter hour, in the foot only while Joel still waits on its act; and
    the same words given the same act within the hour say the time they started rather than
    starting again, and a start's wait is counted to the second
  '';
  start-new-work-finds-places = ''
    Start New Work lists the open quests' folders, then the places the realm's node names, each
    folder once and only in plain words, what the node calls a zone started by its name, a quest's
    folder that is one included, and the rest by their folder, as is a zone two checkouts share;
    and a place the node says isn't checked out started by name all the same; and Wheres.startAt
    gives a row picked before the places land the start it says once they have, found by its own
    folder alone, else the start it was made with; and a node's 2000 places, as long as Joel's,
    are read and weighed in under 50 ms of the shell's thread
  '';
  start-new-work-goes-to-the-cloud = ''
    Cloud.js, behind Start New Work's In the Cloud, reads the offers the agents daemon passes on,
    dropping them where the node can't take the cloud's need, its price the cheapest in one
    currency, and a new price or shape kept as news; asks again once they're stale, or at a
    Return while none is offered; lists only the places another machine knows by name, a zone's
    path or a checkout's last step, each once; reads a link's on: past its name; and asks a
    sprout that needs the cloud, or names a machine by one plain word, refusing a place or a
    machine it couldn't say and the cloud while it offers nothing, saying apart that nothing is
    heard yet, why its last ask came back, or that its providers offer none
  '';
  starts-stopped-lifts-only-on-joels-return = ''
    Starts Stopped's Return steps in among its acts wherever the launcher drops its drawer's step,
    and Let Them Start asks only on Joel's own Return, never a walk from outside or with no
    launcher to say
  '';
  carry-back-asks-the-node-only-a-carry = ''
    Carry Back asks the realm's node only a carry of a quest's id to a machine's name, rebuilt from
    the act's parts so nothing else it holds is sent, and says it done once the carry ended
    carried, else the node's own why, that its why couldn't be read, or that no answer came; a
    carried row reads as carrying back under the key ReadyRows made it with, its thing or else its
    record's key, kept under row: where its thing claims a kind of the launcher's own
  '';
  talk-to-opens-only-on-whom-it-names = ''
    the launcher's talkTo opens a talk on the first word it's given alone: a session's address in
    either case, the void's on its line, a network told apart as having no one to talk to, and
    anything else as nobody, and a network is named by the first word alone
  '';
  mycelium-links-open-in-the-launcher = ''
    xdg-open and gio send mycelium: links to mycelium-link's entry, registered for them in the
    profile's applications so the portal opens them without asking, which asks the launcher to talk
    to the cell or show the network a link names, by its name alone, a notice's whole cell link
    by its address wherever it says the cell runs, and new work a sprout link offers with its words
    decoded and its cell only as an address, each after a fixed word, on the machine it names
    through newWorkOn, and asks nothing for a link naming none of them, an address not in
    capitals, a name qs could take for a flag, a sprout's words missing or said twice, or a second
    link within a few seconds, and calls only functions the launcher's IpcHandler has, each
    taking as many arguments as it passes
  '';
  mycelium-link-passes-clippy = ''
    clippy finds nothing in mycelium-link, which opens a mycelium: link in the launcher, or its
    tests
  '';
  terminal-kit-gives-every-command-one-shape = ''
    the terminal kit takes the shared flags out of a command line and leaves the rest in order,
    inks by --color, NO_COLOR, FORCE_COLOR and the terminal in that order, lays rows out giving way
    by whole items, reads help examples first, says a failure in three parts with its exit code,
    makes a panic a bug, and reads a local time and writes one back as Eastern time's clocks say
    them, summer's hour included
  '';
  terminal-kit-passes-clippy = ''
    clippy finds nothing in terminal-kit, the library every command takes its shape from
  '';
  the-apple-silicon-stick-evaluates = ''
    the stick for an Apple silicon Mac, aarch64-linux on Asahi's kernel and booted as BOOTAA64.EFI,
    evaluates
  '';
  the-ask-row-reads-as-for-you = ''
    the live sites' ask row reads a look as the launcher's For You does, against the launcher's own
    scripts run in Node: over looks either side of each rule and of every kind and state, which it
    reads, which wait, what each offers and what follows each, and when it lapses as Looks.js and
    Now.qml's lookOffers and followsOf give them, what the second form leads with (its parts
    ended by each stop, closing mark and mark that is neither, none or two, a stop inside, empty,
    padded or missing) and its act as
    LookForm.js gives them, the first form's question and what follows doing nothing as Looks.js
    gives them, and what it links as Now.qml's drawer names its kinds, each kind at a
    web address; the launcher's
    act names as Now.qml and core's joined words give them, the last of an id's winning; and the
    ids local-sites' home module guards as Now.qml lists them
  '';
  the-ask-row-reads-each-character-as-for-you = ''
    the live sites' ask row reads every character, and judges it as posing as one of the
    launcher's acts, as LookForm.js does: within, leading and ending a name, once and twice,
    beside a space and after a stop, and in a run after the first; and ends what a look says
    as LookForm.js does whatever character follows its words, a stop or a closing mark
  '';
  the-arrival-line-shows-what-still-waits = ''
    the arrival line shows the notice the notices daemon serves as in front, only its critical one
    while notices are held or over the talk line, none while the launcher has the notices, and
    counts those behind it still waiting; what's playing shows for its moment over a notice that
    can wait but never a critical one, a notice stays its sender's time or five seconds, ten with
    something to do, and an artist's name reads as written; a message to Joel knocks once for a
    burst, never while notices are held or with no sound set, and his safe word knocks even while
    held
  '';
  the-asks-say-how-long-each-was-shown = ''
    a yes says how long since its line's frame reached the screen this opening, counted from the
    first and again from a changed line's, and says no time for a line never presented, for the
    line as it was or for another ask's, so the daemon refuses it unread; a line is timed from the
    first frame after a key of Joel's that moves over the rows, typing or a key read with the
    Return after it starting the time again and Return never starting it; while
    the launcher is walked from outside a yes is stamped as nothing to send; a no is sent as served
  '';
  the-asks-say-how-long-each-was-shown-over-passwords = ''
    over Passwords' connection, a first Return on an ask only arms, a yes says how long since a
    frame presented its line selected, noted again as the launcher reopens on it, a changed line
    says no time until presented, every opening being from outside, a line is timed only after
    Joel's key moves over the rows they reached by his Escape, never on the find, from a step
    into a place, nor over rows a landing Return, the command line or an ask from outside left,
    Release It from its verbs counting from its line, typing or an ask from outside starts the
    time again and lets a frame asked for go, each case reads a yes it sent, a withheld yes says
    why, the entries told over the connection leave the asks' rows be, a yes walked from
    outside the launcher is never sent, and while the screen is shared or niri hasn't said its
    casts each ask's row names only that it's hidden, offers no act, times no frame and counts
    nothing once shown again until his next move, For You's row hidden too, a yes from a row
    made before the share is never sent and is told as hidden, the share withholding it or, before
    niri has said, not knowing, a row held as it becomes from just before the share is held
    again hidden and as itself once the share ends, another source's hold left be, what's told
    and unread forgotten as the share begins, two asks hidden at once told apart by nothing but
    their keys and when each was asked, and the shell's own recording of one window hides
    nothing
  '';
  the-asks-place-releases-through-its-daemon = ''
    the launcher's Asks lists what the secret-asks daemon serves, For You holds each waiting ask,
    a first Return arms it and a second releases it, r refuses, a yes while locked is refused in
    the daemon's words, Refuse It refuses, and a yes walked from outside is never sent and its
    walk is told it was refused
  '';
  the-askpass-outlives-its-shell = ''
    the askpass asks a restarted shell again, never asks twice what's answered, and refuses about 20
    s after the shell goes unless it comes back; it answers --help and refuses anything but one
    prompt with 64, asking nothing; it decides every case in ssh-tpm-agent's gate-cases.txt as the
    gate does, by the caller's cgroup and each parent's, refuses nix-offload's serving units, a
    cgroup it can't read and a gate it can't read, and says where a refused asker ran
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
  the-camera-takes-its-loopback = ''
    the XPS 14's camera service takes the loopback of its name already there, held or not, adds one
    only when there's none, deletes the others of its name where nothing holds them, and of
    several keeps one a program holds
  '';
  the-camera-takes-the-one-plugged-in = ''
    the camera program takes a camera on a removable USB port over the machine's own while it's
    plugged in, never a built-in one on a fixed port, and the machine's own with none
  '';
  the-cat-is-drawn-in-whole-pixels = ''
    a pixel mark's ink is laid in runs along its rows, each cell a whole number of the screen's
    pixels at 1, 1.5 and 2, and the drawing moved by less than one to start on a screen pixel
  '';
  the-cat-keeps-joels-words = ''
    every design Joel stated for the margin cat, dated, is held by a phrase one of its cases says
    outside a comment, or owed since a date, and the owed ones are listed
  '';
  the-cat-runner-names-how-each-case-ended = ''
    the margin cat's case runner fails a run with a case that failed, ran past its own processor
    time, was killed from outside or hung past the clock's guard, names each by how it ended and
    says every case's time, and stops at a hang, naming the cases it left unrun; it refuses
    limits that aren't whole seconds before any case runs, and, where the builder holds the run
    below a case's hard limit, says so once with the hard limit in force, what would fit and
    ulimit's reason, and runs no case; the check fails where that can't be shown; a case
    whose limits are refused as it is run fails unrun rather than running with none; and a run
    that can't make both its scratch files stops before any case, leaving neither behind
  '';
  the-cats-walk-is-read-only-from-its-lists = ''
    the cat's code is read as no walk, by the rule it breaks, when WALKS holds anything but
    frozen quoted words, either isn't found outside a comment, a word holds a space or
    mismatched quotes, either is left unfrozen or frozen only in part, WALKS is named
    elsewhere, or either is declared again, in a block or as a function, class or loop's
    variable, though as its walk when they're only spaced, quoted or commented otherwise,
    mid-line or at length too, a longer name or another's property holds theirs, and STRIDING
    is read above its declaration or below; and it's read past a copy commented out, the forms
    its reader says it doesn't see passing unseen
  '';
  the-click-strength-finds-the-intensity = ''
    the click strength's setter finds a haptic pad's intensity, the XPS 14's among them, and writes
    only it, only when it differs
  '';
  the-click-strength-holds-after-sleep = ''
    a machine naming a click strength sets it as each hidraw device appears and after every sleep,
    and one naming none runs nothing
  '';
  the-clipboard-says-once-it-is-silent = ''
    Copies and Emoji say the clipboard's history is not answering only once it has been silent a
    second, and stop on its next answer, and say a paste that never landed for a minute after it
  '';
  the-collector-keeps-up = ''
    in the shell's own Qt, one long call making garbage around a scattered few it keeps, beside
    a heap already holding what a started shell holds, grows its memory by under 300 MB, where a
    collection stepped only from the event loop let it grow by about 420
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
    pass over them, timed in turn; and a late refusal after Joel typed on is journalled by the
    daemon's code, its words quoted only with reports on and otherwise by how many letters
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
  the-daemons-test-the-launchers-words = ''
    every word a daemon's tests are given, To Do's, Location's, Screens', the machine's rows' and
    Slack's, is the launcher's vocabulary's own at the same place, and a changed, unknown or
    vanished word is refused
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
  the-document-portal-comes-back = ''
    a graphical machine's document portal starts again a second after it fails, keeping the
    account manager's PATH, where fusermount3 is
  '';
  the-drawer-over-the-card-meets-it-square = ''
    a floating surface over another, as the launcher's drawer over its card on a narrow screen,
    meets it with a square foot and region, its top and the card's foot staying round
  '';
  the-drawer-search-keeps-pace = ''
    a drawer's search over 100 long lines of hyphenated names finds all 800 places within six
    times the same lines with no joiners take, and those within twelve times the plainest search,
    in the fastest of five processes
  '';
  the-ergodox-command-reaches-the-board = ''
    `ergodox` writes each command whole to the board's channel alone, and nothing for --help or a
    word too many or too few
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
  the-home-network-is-offered-where-eero-runs = ''
    the launcher offers the Home Network place on a machine whose home imports eero's module, which
    runs its daemon, and not on one without it
  '';
  the-home-network-says-each-figure = ''
    the figures the Home Network, the Work Network, Machine and the virtual machines say rates and
    sizes in read in their own units to three digits, and the Home Network's row at home counts
    its nodes and devices; its rows and drawers are the eero daemon's, held by its own tests
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
    under ~/src, served by the programs daemon once for both as the shell starts and told
    again only as they change, nothing asked as the launcher opens, hold one they
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
    says which; the weather's own drawer says it in brief: now, where, the next hours, the rain
    while it's due, and each day
  '';
  the-launcher-answers-what-is-asked = ''
    sums, bases and bits, addresses, subnets, characters, measures, zones, colours, the machine's
    figures and its parts' heat answer, and WARP says how its link stands from the record warp
    prints, connected, degraded, down, not answering or unreadable, and nothing with no warp-cli,
    every figure of the record a part in its own unit and in words.json's words, its settings
    saying who set each, its TLS and DNS a step in; what it read while shown sparks Round Trip,
    Loss and Traffic, each with a drawer over the tunnel, one read sparking nothing; and its
    switch says who holds it, when WARP always on comes back, and a switch held or refused; and
    each command's worked example with a fixed answer says the headline it gives
  '';
  the-sessions-kitty-outlives-a-switch = ''
    a switch never restarts the kitty every session's window is in, so no switch closes them
  '';
  the-sessions-kitty-stays-off-the-disk = ''
    the kitty every session's window is drawn by keeps what it swaps out compressed in memory, never
    writing it to disk
  '';
  the-sessions-kitty-keeps-niris-pace = ''
    the kitty every session's window is drawn by draws only the frames niri asks for, so the
    session windows niri doesn't show aren't drawn
  '';
  the-sessions-kitty-answers-only-its-own-socket = ''
    the kitty every session's window is drawn by is asked for windows only over its own socket, in
    a runtime folder only Joel's login opens, never as a single instance another program could
    answer for
  '';
  the-sessions-kitty-is-asked-only-its-own-way = ''
    a session's window is asked of the sessions kitty over its socket, or of the unit's own kitty
    from before it listened, waiting up to 20 s for one starting again, and nothing opens with
    neither; and the packaged kitten says an unanswered launch in the words the script reads
  '';
  the-sessions-kitty-says-its-pid = ''
    the kitty every session's window is drawn by writes its pid beside its socket, which names
    none, failing its start where it can't, so the shell can find a session's window among those
    that kitty draws, and then tells the shell it started, by the name its sessions handler
    answers, within 5 s and only journalled where it can't, so a viewer kept for a kitty come back
    is asked of it without a shell not yet up stopping the kitty
  '';
  the-sessions-kitty-comes-before-its-sessions = ''
    the kitty every session's window is drawn by sits in agents.slice above its sessions' weight,
    with a hard memory floor agents.slice keeps beside the 1.5 GiB jobs-sorter holds for the
    windows Joel is at, so a session opening never starves the window Joel types in
  '';
  a-paste-lands-while-the-shell-is-busy-on-niri = ''
    on niri, Return on a copy in the launcher pastes it into the terminal under, though the
    shell is held stopped the moment it has asked and let go of the keys, since the
    clipboard's history serves the copy and presses the paste keys through a virtual keyboard,
    within 20 ms of the launcher letting go
  '';
  the-screens-go-off-on-niri = ''
    on niri, Turn Every Screen Off closes the launcher and then asks niri to power its monitors
    off, and :set noscreens asks it too, as read on the way to niri's socket; a :setl scale
    niri refuses says niri's words for why on the : line; and Screens, inside Machine, says
    how many screens there are as the windows daemon counts them
  '';
  the-newest-surface-takes-the-keys-on-niri = ''
    on niri, a notice arriving while the launcher is open never takes its keys; a click on a
    window gives that window the keys and closes the launcher; and a window of the program the
    launcher handed to closes it and takes the focus
  '';
  an-agents-return-does-its-harmless-act-on-niri = ''
    on the account's whole shell on niri, its sessions the realm's mycelium node's, a listening
    agent's Return asks what to say to it and a paused one's lands on Resume, which a second
    Return sends, while Ctrl+] still steps into every act either has; a drawer names the repo
    its record says over the folder it works in; a paused one's glyph is never drawn in a
    listening one's colour; and an orchestration's waiting sessions fold into one row after its
    own, which Return steps into and typing still finds each in; and a look's choice in For You
    steers its answer to the session that left it and, once taken, moves to History saying it
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
    letters found, steps into For You's looks by :filter look and back out by -, stands the
    selection on the row an arrow lit in the arrow's first frame and draws it on its row as
    rows above it change, draws again no row a narrowing letter kept and, for a first letter, only
    the rows in view and one past them, reads on a long name while it's selected, tells the meaning daemon the
    parts its places hold and adds one meant below what the letters found, saying where it lives,
    and warns of nothing it drew;
    WARP, read by the real warp from a stand-in warp-cli, says how its link stands in Network
    after Traffic, and its service not answering, and its :disconnect runs warp disconnect and
    says when WARP comes back, or, its switch held, leaves it and says who holds it;
    and the notices it holds, taken as Super+N takes them, turn, forget and step into their cards
    by vi's keys
  '';
  the-launcher-asks-a-fresh-pi = ''
    the Ask row says why pi-ask can't start before it's asked and within 3 s of Return, which
    asks again rather than closing, and each question is answered by a pi of its own
  '';
  every-command-row-has-a-drawer = ''
    every command the launcher offers says in its drawer, under the place that lists it or
    anywhere, what it answers and one or two worked examples, naming only documentation addresses
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
    the clipboard's history finds emoji by name and meaning, best first, over the built list
  '';
  the-launcher-finds-the-desk-light = ''
    the launcher says how the desk light is lit, its drawer its lumens, node and serial, sets its
    brightness, warmth and power through litra, and the bar hears changes but not a camera; with
    no light its row stays, saying it isn't plugged in or litra isn't answering
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
    each saying how it stands as Machines does, makes a quest's row told while it's open elsewhere
    ahead of the step back, after a frame, and one told while already there, making no row on the
    step back, though none as an opening fills in at home or in another place, under a viewer's
    command line, or shut with a step held, and sees one made since, remaking only the quests a change
    names and none a list written again leaves the same, and breaking each title into the pieces
    A to Z reads once, again only when it changes
  '';
  the-launcher-finds-the-queue = ''
    the launcher's queue names each job in words, running first, says its time against its last
    run or what it waits for, its quest and memory, and goes to its session or copies its name;
    the sessions said anew make no row again, and the queue written again only the job it changed
  '';
  the-launcher-heads-the-selection-at-a-whole-word = ''
    On the Selection's head is the selected text on one line, given way at a whole word; an
    answer is named by its first sentence, pi-verbs' actions keep its order, and each answer's
    note says how it stands, a replacing one that it was copied
  '';
  the-launcher-hears-talks-from-the-node = ''
    Agents hears each talk from the node by its session's address, newest first, moves it on as
    its session writes a line, hears one made since and lets one go, reading no talk's file; and
    the opening that reads the quest index orders the closed orchestrators by it
  '';
  the-launcher-finds-the-runs = ''
    the launcher lists the day's fleets and councils, running first, with progress or cost and time,
    goes to their session, opens an answer whole, and hides amounts while sharing
  '';
  the-launcher-finds-what-is-meant = ''
    the right row comes first for each query, within budget, and every text matches every query,
    scored and marked, as fuzzy.json says, the cases the daemons' matcher is held to too
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
    changes nothing, an act another provider gives makes its own row and no program's, a step
    asked again makes none of its Recent's rows again, and a row the programs daemon serves is
    the program's, going to the window served or starting it, its windows left to a say that
    counts what it holds; typed inside Programs, the rows holding its letters stand at once, and
    the programs daemon's answer takes its own order, the list shown again
  '';
  the-launcher-holds-still-as-it-refreshes = ''
    a list the launcher shows keeps its order and its selection as what's in it changes, but an
    answer given only while nothing else was goes once something comes; each row stands as
    Steady's tallOf says, with its own gap above it wherever it stands, and a heading with the
    groups' rung above it but for the list's first; and the height Steady's tallFor gives the
    card's list is what the rows it shows stand, every gap and heading among them counted and
    none below them, within the screen's room and with no heading alone at its foot, a list
    shorter than the card sized for the room
  '';
  the-launcher-joins-only-what-changed = ''
    joining the launcher's rows again makes ready only the rows of sources whose things or
    stand-ins changed, lets go of a source that's gone, and indexes the rows by key, name and
    what they're part of in one walk, saying each name plainly once while it's there
  '';
  the-launcher-closes-only-for-keys-it-held = ''
    the open launcher holds its keys once its card is active and niri says no window has them,
    so an opening right after a closing stays open as the closing's late focus arrives; held, a
    click on a window closes it, and a notice held over it leaves it open
  '';
  the-launcher-holds-a-key-for-its-answers = ''
    a key whose search awaits a daemon's answer is held for it, a key typed over it drops its
    search, and Return or anything else that reads what was found searches it at once; the next
    key's letters count from the held key's text
  '';
  late-answers-are-taken-in-once = ''
    what a key's sources find after its search, heard in one turn, is taken in once, fresh when
    any came fresh and for what was typed as the last came; a key's own search drops what's
    gathered, and answers heard in turns apart are each taken in
  '';
  a-place-reads-alike-everywhere = ''
    a place's row says nothing after its name; at its right it says how it stands where it says,
    else its word for none while it holds nothing, else how many it holds, and nothing for a place
    that only groups others; its drawer names what it holds, in its own order; and kept for the
    next opening and still unfilled a second after it, home's rows stand in their order by their
    names, each saying how it stands as being read, its note as it was
  '';
  a-place-stands-by-its-part = ''
    a place tied to a part of the system says nothing more while the part is good or the system
    daemon isn't answering; a part it couldn't read says so in a muted word, never blank, its mark
    its own; a part worth a look says the daemon's own side, the first item kept longest, its mark
    and side in the warning's role, or the alert's when bad, and its foot the whole sentence
  '';
  the-shell-hears-the-systems-parts = ''
    the shell watches the system daemon's parts and holds each by its name, the summary aside; a
    part told anew replaces the one held, a part let go is gone, and once the daemon goes no part
    is held and the daemon reads as unheard
  '';
  the-launcher-opens-on-what-it-kept-on-niri = ''
    on niri, an opening draws home as the closed launcher left it, each row saying what it said and
    no row made before its frame; a viewer's command line opens with no source's rows made at
    all; left with a word typed, home is kept all the same, and its rows
    stand where they stood once it settles; moved down as it opens, the cursor stays on that row;
    no frame from the first to the settled one empties a row and fills it again or puts a shown
    value back as being read; keys typed the moment it holds them are all found; and chords said
    as the shell restarts open it once the new one listens, a second never shutting it; a
    program's next ask what to open a file with is drawn from what the last kept, asking
    about its own file, and refused or shut if shut or withdrawn before it's filled in;
    and chords said all through a handover are each presented within 2 s, software-rendered,
    with niri never holding both shells' surfaces, the new shell told to systemd as the unit's
    main process and the old one ended; while niri is yet to say its casts, a chord into On and
    home each open afresh, the journal saying why, and a share begun as home, On or the
    clipboard is drawn from what was kept fills it in at once, the kept copy's text gone from
    the card; and with a clipboard showing a copy left on the way forward, as the last place
    and on the tags' way back before a share began, each of Ctrl+I, Space Tab and Ctrl+T during
    the share stays where it is, the key forgetting that place and saying so
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
  the-launcher-weighs-what-it-owes = ''
    the launcher's weighing plan, apart from its timers, starts nothing before the census, counts
    the shell's holders once, then weighs what letting go of each of the six largest by that count
    would free, one a turn, leaving out names folded under a tilde; and a busy count or weighing
    stays owed for the next closing to take up rather than being dropped
  '';
  an-act-shows-before-its-daemon-answers = ''
    a row an act asked a daemon to change is drawn as it will be, keeping what the launcher made
    of it, until its source's row reads the same; one to go is left out until it's gone; a hold
    comes due once its time is up; and the foot says what wasn't done, then why
  '';
  the-launcher-paces-its-refreshes = ''
    an open launcher takes the first of its sources' changes at once and those after it a pace
    apart, together, the pace three times the median of what the last three refreshes cost, at
    least a frame and at most a quarter second, so one dear refresh holds the next change no longer
    and a storm of changes spends at most a third of the time refreshing; and a
    refresh taken another way takes the one waiting with it; and a source's rows are wanted only while it's held,
    searched, named at home or has an act waiting, and none over a viewer, so a change nobody
    sees builds none; and a key's act is said shown by the first frame begun after its refresh,
    which another act refreshed while that frame is drawn never holds back; and an opening says what its own work
    went on slowest first, and weighs the foot's room for Return's verbs again only once the
    words file or the face has changed; and a daemon's changes are timed from the first line
    its draw shows, and said a place a line, with where their time went as a key's is, a refresh
    held for home's find said apart from the pace and held from when it was first due; and y's
    flash starts its time a turn after the frame that draws it, and only for the newest yank;
    and a chord into a kept place draws the rows it kept only once niri has said nothing casts
    the screen, unless its source, a mark or what it's led to lists them afresh, and a share
    begun before a kept opening is filled in, or before home is found after it, is judged to
    do it at once
  '';
  the-launcher-places-each-row-it-is-served = ''
    the rows a daemon serves ready are kept in the order each says, each a change names placed
    between its neighbours without sorting the rest, a row gone let go, and the list before left
    as it was
  '';
  the-launcher-places-each-window-it-hears = ''
    the windows the windows daemon tells of are kept the one used last first, each a change names
    placed where its order puts it without sorting the rest, the same record again changing
    nothing, and a window gone let go; and agents' windows, as the agents daemon pairs them,
    fold into one row where the one used last was, named for its session, and inside it go
    by orchestration, a session a step in under its orchestrator's own window
  '';
  the-launcher-places-the-time-only-when-now-is-wanted = ''
    an opening into a place that shows none of For You's rows places no time or date, so makes
    none of Time's or Today's rows, until Now's rows are wanted, and they place them once, only
    once a minute has turned since they were placed
  '';
  the-launcher-plays-from-spotify = ''
    Spotify's program in the launcher reads Spotify only as it's stepped into, once an opening,
    and holds its Recent, newest first, each row timed and noting its kind and size, and Liked
    Songs, Playlists, Albums, Artists and Podcasts, each counted, each played, opened and copied as
    it says, its note saying when it's signed out or out of reach
  '';
  the-launcher-says-a-lagging-keys-kind-on-niri = ''
    on niri with reports off, a lagging key's journal line says down and up whole, and a step in by
    Return, out by - and in by a word and a space by its arrow and the key of the launcher's own
    place it took, never the place's name; a key listed without the notices' answer, and their
    answer late, say what was typed by how many letters; and a program's lagging drawer is said by
    its desktop entry's id, never by its name
  '';
  the-launcher-says-asks-from-outside-by-kind-on-niri = ''
    on niri with reports off, a link to a session, a network, a machine or a repository says in
    the journal the words it was given by how many letters, a catalogue run and a terminal's do
    say the rows they walked by kind, never whole, and a terminal's do that fails says only its
    error's name
  '';
  the-launcher-says-how-each-session-stands = ''
    a pi session offers its acts in their order, as the agents daemon says how it stands, its
    drawer's times are worded by the clock, the sessions beside the one drawn are found
    to watch their drawers ahead, what it landed is kept past its end, and what a Reopen Them
    All walk found gone is said as one folder gone or many, then how many reopened
  '';
  pi-tells-of-its-sessions-as-an-agent-source = ''
    pi, as an agent source, sorts the node's sessions into those running, parked and closed this
    boot, and says each as the agent source contract's record, leaving out what pi didn't say
  '';
  pi-agent-proxy-passes-clippy = ''
    clippy finds nothing in pi-agent-proxy or its tests, the SSH agent a fenced pi command reaches,
    which signs only logins to the hosts it's told of
  '';
  pi-agent-proxy-runs-only-with-machines = ''
    pi's SSH agent proxy runs only where it's given machines or the builders' key, never on a Mac,
    started with the session where it makes an OS Login key and otherwise by the first ssh that
    asks, on a socket in a folder only the account reaches, told of
    those hosts and the agents behind it, or of the builders' key and root's file of builders lent
  '';
  pi-agent-proxy-reaches-os-login-machines = ''
    where pi's SSH agent proxy runs, by default, it makes a key for the OS Login hosts gcloud
    knows, started with the session, never in a core dump or swap short of a hibernation image;
    the account's gcloud registers each key it writes, once the proxy lists it, for twelve hours,
    and again as the desktop starts, renews it each quarter hour by the wall clock, every call
    bounded and a removal's tighter, tries a failed registration, or a proxy it couldn't ask or
    that never answers, again from half a minute on in growing steps, never giving up, and
    removes it as the proxy stops, keeping the file; only while the proxy runs is it registered
    or renewed, from the one line the proxy lists under its OS Login comment, copied beside the
    key and never into the temporary folder, a removal sending the same line, never a key in the
    file the proxy doesn't hold or holds for another use, even beside its own, and only a key whose file is the proxy's is
    removed; no gcloud, no gcloud login or a gcloud that can't say is warned of and passed over;
    the machine's ssh names it for compute.* hosts; and with no known-hosts file named,
    there's no such key
  '';
  pi-agent-proxy-signs-only-logins = ''
    pi-agent-proxy lists the keys of the agents behind it and signs a login only once the host it
    names, one it's told of, signed the session; with no session, another host, a forged binding,
    a commit, another session or key, or a forwarded agent, nothing reaches the agent behind; it
    never adds, removes or locks keys, and answers at most 64 clients at once; it signs with the
    builders' key only nix-offload's login to a builder lent now, and no agent's key for one; it
    signs with a key made in memory only a login to a session an OS Login host signed, one
    gcloud's known hosts name and neither told nor lent, and no other key for one, looking at
    those hosts only in a plain file, a line at a time, and its first 8 MiB, making the key only
    once it's listening, keeping its memory from the account's other processes, and going on
    without it, saying so and taking a previous key's file away, when it can't be made, but
    failing when nothing else was asked of it, and refusing one OS Login flag without the other;
    and it says each login signed and each refusal in one line of a fixed shape, its host and why
    as a word
  '';
  pi-clipboard-door-keeps-secrets = ''
    pi-clipboard-door answers a client only when the kernel puts it below a bubblewrap of this
    account, for that bubblewrap's session and never the one it claims; it never reads a password
    manager's secret, refuses a request unread, past 64 MiB or for a type not offered, and its shims
    take wl-clipboard's options as getopt does, refuse any other by name and exit as it does; each
    copy, paste and refusal is one line of a fixed shape, never with what was copied
  '';
  pi-clipboard-door-passes-clippy = ''
    clippy finds nothing in pi-clipboard-door or its tests, the clipboard a fenced pi command
    reaches, and its wl-copy and wl-paste
  '';
  pi-clipboard-door-reaches-only-the-fence = ''
    pi's clipboard door starts from a socket in a folder only the account reaches, tagged in the
    journal, its shims linked from a folder of their own, and the sandboxed display runs with the
    graphical session at %t/pi-fence-display/wayland, in a folder of its own that outlives a
    restart; a Mac gets none
  '';
  pi-fence-clipboard-holds-on-niri = ''
    on a real niri, the sandboxed display offers windows' protocols and none of the privileged
    ones; a fenced command copies and pastes both selections through the door, is refused a secret
    and an option it doesn't take; a command outside is refused; and the journal names the
    bubblewrap's session, never the one claimed or what was copied; shell-preview's nested niri
    opens on the sandboxed display and takes wtype's keys, which the display itself refuses
  '';
  pi-fence-census-loads = ''
    pi-fence's census, as core builds pi-fence from the harness's source, loads and judges a
    machine with no sessions, counting none and with nothing proven fenced saying it doesn't hold,
    rather than refusing because it couldn't load or throwing after its first line
  '';
  pi-fence-never-runs-unfenced = ''
    pi-fence, the store's launcher for one command in pi's own fence, refuses a kind it doesn't
    know with its usage, and where it can't lay the fence, as in a build with neither bubblewrap
    nor landrun, refuses either kind and says why rather than running the command unfenced
  '';
  pi-fence-display-passes-clippy = ''
    clippy finds nothing in pi-fence-display or its tests, the sandboxed display pi's fenced
    commands open windows on
  '';
  pi-github-door-lets-only-allowed-calls-through = ''
    pi-github-door sends a fenced call on to GitHub with the person's token in the placeholder's
    place only when it's allowed; a merge, a push, a pull request opened, a credential, a
    repository's settings, protection written or a mutation off its list, judged by its fields,
    never reaches GitHub and names its route; another credential is refused, the token is read
    again on a 401, git's fetch passes, other hosts are tunnelled unseen, the session asked for
    least lately is given back once 256 are open, and the relay carries plain HTTP straight out,
    one request a connection and refused to GitHub's four names, and leaves with the process it
    watches
  '';
  pi-github-door-passes-clippy = ''
    clippy finds nothing in pi-github-door or its tests, the door a fenced pi command reaches
    GitHub through
  '';
  pi-github-door-runs-beside-the-fence = ''
    pi's GitHub door runs on every Linux machine and no Mac, started by the first fenced command
    that asks, on a control socket at %t/pi-github/control.sock only the account reaches, its
    sessions beside it, reading the token with gh, and on the account's PATH for the fence's relay
  '';
  pi-spawner-passes-clippy = ''
    clippy finds nothing in pi-spawner or its tests, which starts pi's shell commands for it from
    a process small enough to start them cheaply
  '';
  pi-spawner-starts-commands-for-pi = ''
    pi-spawner passes on what a command writes and how it ends, runs it in the folder and with
    only the variables pi gives, in a process group of its own, says why one couldn't start, stops
    sending a dropped one, and kills every group still running once pi goes
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
  the-launcher-keeps-a-daemons-words-out-of-its-journal = ''
    with reports off, the journal says how many letters a daemon's refusal held, not its words, for
    an act shut and refused, one withheld, what an act asks besides, words typed left unanswered and
    a search refused, a search's unserved row by its kind and Wi-Fi's own refused act by its code
  '';
  the-launcher-keeps-its-trials = ''
    each opening of the launcher is kept as its keys by kind, its pick and how long it took, and a
    miss only where letters were typed and nothing picked, and the file gains a close's line alone
  '';
  the-launcher-keeps-its-trials-one-by-one = ''
    a close adds its own opening's line to the file and rewrites none before it, however many are
    kept, and a start carries the old list into lines and lets the oldest go; with reports off, a
    slow key's line says how many letters, a slow step's keeping its arrow, and a key the
    launcher names itself, such as page down, is said whole
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
  the-launcher-makes-rows-lightly = ''
    a ready row's name, made ready to match once it's first read, adds at most 800 bytes to the
    row over its own fields, where a getter keeping the defaults and fields the row was made
    from added 920 to 1,190
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
  the-launcher-puts-back-a-look-it-couldnt-write = ''
    a look cleared in For You shows cleared in the act's own frame, and when its file can't be
    written is put back as it read, its failure said in the journal with reports off by the
    file's letters and the daemon's code, never their words, the shut launcher keeping the looks
    it read, the daemon's word on one it settled, the same look written its own way, leaves the
    text the act showed, and a served look whose session is gone is never settled and Now tells
    the launcher once that it's gone
  '';
  the-launcher-reads-spec-from-its-daemon = ''
    the launcher's Spec reads what the cadence-spec daemon serves: the head as the daemon says it,
    1 reading out of spec with the alert glyph, its axes in the order served, each a step in, and
    the out axis holding its reading
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
    the launcher's Screens draws each screen's row as the windows daemon serves it, makes again
    only the row a change names, offers Windows the screens laid out by what their rows hold, asks
    the daemon to look again once it's changed a screen, and says the Brightness's share, or that
    it isn't read yet, as the place's line serves it; with reports off, a share the daemon refuses
    is journalled by its code and how many letters, not its words, and one the : line takes goes
    to it with the daemon's words and code; and as the launcher opens Screens feels a span by its
    name, while work a source feels through its span runs from before it to after it, is named
    shutting while the launcher is shut, and gives what the work gives and throws what it throws
  '';
  the-launcher-remakes-only-the-windows-that-changed = ''
    the launcher's windows, as the windows daemon tells of them, list This Window, then the rest
    the one used last first, and when the focus moves make again only the rows of the windows it
    moved between, a renamed one too, the same whole answer again none and a closed one going, as
    Capture's rows of the windows do, and Capture asks each thing taken's facts of the programs
    daemon's one file-preview, only the newest wanted waiting, one changed asked again, and one
    that can't answer said so
  '';
  the-launcher-remakes-what-it-holds = ''
    a step the launcher holds is made again with its row as it's offered now, a step whose row
    is gone is kept as it was, and one whose row is still offered but offers nothing now, a typed
    one's too, is left back to the step above it, or past the collection a search found it in,
    never past a dialog's own collection, which is kept with its rows gone; and, as Held.js
    answers over histories given it, every place through the spent step goes from the way back,
    Ctrl+I, Ctrl+T and Tab, which forget where it landed, and an answer saying a new figure is
    still the same place
  '';
  the-launcher-says-what-counted-most = ''
    the launcher's journal names what counted most, the most first and a tie by name, no more
    than it's asked for, none that counted nothing, and nothing for none
  '';
  the-launcher-says-the-page-zathura-is-on = ''
    zathura's provider reads open-now's lines as each zathura's document, page and page count,
    says the page beside a document only one zathura has open, and its drawer says each one's
    page and how far through it is
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
    modules/quickshell/tests/launcher-shown.py lists, each printed with its word, and the check refuses a stand-in
    vocabulary breaking each of its rules
  '';
  the-launcher-shows-the-moment-in-now = ''
    Now's rows after For You and before Lately's are the time and the date as the launcher
    places each anywhere, never a version Now draws, the location, saying its town and when the
    place keeps another zone than the clock, the weather, placed the same way, read as the
    launcher opens and kept while it's asked again, then the calendar's own row for the next
    meeting, and Now counts nothing while only they are there; the location and the clock's zone
    are whereabouts' own, over its socket;
    and For You, asked again, makes none of its rows again, its looks, its to-dos, the sessions
    wanting Joel and To Look At's own, only one whose file or what it says changed, a session
    wanting Joel making only the look it left again and to-dos said anew as they were none; and a
    piece of what's on that moved is the only one whose row and acts are made again, a kept one
    running its piece's act as it is now; and Now's Desktop holds the notices unread as the
    notices' own rows, and its History those read, stepping in again after one is read making
    only that one's row again; and Now, stepped into For You since it last said what it shows,
    says it changed only where For You's rows differ from those the launcher shows; and the
    notices moving, by their rows or their history, each heard outright with nothing held, aren't
    said outright as they move with For You held, in that turn, though whether one is said in the
    next turn is left unproven, while with Desktop, Slack's notices or what's on screen held right
    under Now, Now's own noticesMoved says them outright, though not with Mail there
  '';
  the-launcher-sums-within-a-key = ''
    the launcher's sums and measures, worked out by libqalculate in the shell's own process, take a
    typical key under five milliseconds
  '';
  the-launcher-traces-each-opening = ''
    each opening of the launcher is a trace whose steps, backs, acts and searches are spans under
    it, a void above a list's first row is told from a list scrolled down, and the whole is sent
    in OTLP's JSON mapping
  '';
  the-launcher-words-each-ready-row-it-is-served = ''
    a place's rows served ready in the crate's wrapped form are placed by their order and a
    summary beside them never is; each time form is said in the launcher's words as of now, a
    time put into its words where they hold one; a side joins its items by a dot, a drawer's
    facts are drawn as the drawer reads them, its picture and its eyebrow's parts passed through
    only where served, and a row says whether its words turn by minute, by day or never; a row
    saying only that it has a drawer has the one its place makes of it, and an emoji's says its
    group, name, keywords (none where it has none) and every code point in four hex digits or more
  '';
  the-launchers-field-says-which-keys-acted = ''
    a modal launcher field's keys, sent through Qt to its own handlers, act exactly where they do
    something: a letter typed, a caret's edit, Escape at rest, h, a modifier alone and a
    register's keys acting on nothing, Tab alone asking where it goes, the drawer and gv acting
    only where there's one out or something to take back, the operator again swallowed, and an
    ask's answer, the operator again among them, acting where the ask takes it; a secret naming
    Escape or Ctrl+C as it's given up, saying Return, and saying nothing of its deleting keys; and
    the command line naming the key that let it go, Escape, Ctrl+C, or Backspace and Ctrl+H on
    an empty line, while Return, editing, choosing and a register's keys let nothing go
  '';
  the-launchers-graphs-draw-in-braille = ''
    a row's graph is ten braille characters over a dim track, a bar from the left, a spark from the
    right and a strip a reading a member, warning at 95%; graphs line up in one column, and a
    figure wears no glyph
  '';
  the-launchers-rows-keep-their-words-apart = ''
    a launcher row and field keep their words apart and inside them, however long, ending what
    doesn't fit in an ellipsis, words that fit as Qt draws them said whole, hinted fully at 1
    too, and the launcher's note, nothing typed, ends where a row's value ends and says all it
    has room for
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
  the-mac-pi-host-outlives-a-log-out = ''
    a realm a Mac's primary account names gets two pi hosts as launchd daemons run as that
    account, started at boot once the store is mounted and again after a failure, given 150 s
    to park and 8192 open files, naming no store path a switch would restart it for, with no
    log launchd opens as root, and becoming node under a name mycelium tells as an agent's, as
    its own kind with a log of its own, keeping V8's own heap growth though its realm names a
    percent for conversations, on a Mac that evaluates whole; a Mac naming none gets none
  '';
  the-mac-pi-host-starts-as-its-account = ''
    a Mac's pi host start, run under dash and bash with the Mac's programs stood in for, passes
    shellcheck, moves only a log past 10 MB aside, waits before a fourth start in ten minutes,
    counting from the oldest of three, and not after two or one recent start of three, saying
    that removing its record of starts ends the wait, which then ends within ten seconds,
    says why and fails on a missing harness, on a temporary or cache folder that isn't there, is
    a file, is a link, is another's, the cache folder alone included, or others may read, and on
    a cache folder that is a link named with two slashes or a dot, or is named by a relative
    path or through .., each through the one check both folders pass, naming the folder, and
    otherwise becomes the
    realm's node in its own place with its words, its output in its log, every environment value
    whole, the realm's LANG and no LC_ALL to outrank it, the account's home realm, TMPDIR the
    account's temporary folder, AGENTS_RUNTIME_DIR its cache folder, and no XDG_RUNTIME_DIR
    even when launchd gives one, since disk-guard reads it for its own, the probes
    its own restart waits on with their % single, and none of the eight variables that would run
    code as node starts
  '';
  the-macs-faces-are-the-themes = ''
    the graphical role's fontconfig, asked for a Mac's system or monospaced font by a name pages give
    it, gives the theme's sans or monospaced face, carrying the name asked for
  '';
  heat-remakes-only-what-moved = ''
    Heat answers its rows and headline as the machine daemon serves them ready, in its order, each
    part's own rows a step in; asked again with nothing told it makes no row, a row told anew is
    made again alone, and a row let go goes
  '';
  heat-stands-while-its-daemon-is-away = ''
    with nothing at the machine daemon's socket, Heat's row stands: held in its place and being
    read before its watch begins, then saying its daemon isn't answering, never gone
  '';
  power-remakes-only-what-moved = ''
    Power answers its rows and headline as the machine daemon serves them ready, in its order, and
    lends a row by its key; asked again with nothing told it makes no row, the charge told anew is
    made again alone, and a charge limit's stop draws the limit as it becomes until the daemon
    refuses it
  '';
  the-machine-answers-from-its-daemon = ''
    Machine answers the processors, the memory, the disk and the uptime from the machine daemon
    reading the sandbox's own /proc, their headlines making no parts, and asks it for the programs
    using the machine only once a part saying them is looked at
  '';
  the-margin-cat-follows-the-keys-on-niri = ''
    on a nested niri, the margin's cat sits over the column the keys are in and, two seconds after
    they move to another, walks there a step every frame with none dropped, its strides in step
    with the ground it covers, never for h then l tapped; going down a workspace it rides up with
    its window and climbs to its seat, going up it stays as the new one slides under, and a second
    move seats it at once; on the overlay layer taking no keys, it
    leaves while a window is fullscreen, draws nothing for 30 s with no keys, then curls up
    asleep in a frame and wakes at
    a key, stretches for about half a second as the lock lets go, and is drawn in whole screen
    pixels of its hue at scales 1.5 and 2, each screen kept as its result; a pointer moved onto
    it leans it toward the pointer's side until it goes, a pointer it walks back under leaves it
    sitting, and a click beside it reaches the window there
  '';
  the-margin-cat-plays-more-the-less-he-types-on-niri = ''
    on a nested niri, the margin's cat makes a single small move in place while keys come, held
    a moment in a frame each way, plays in place once they pause, and once they're gone hops,
    rising and landing where it sat, and wanders a few cells along the margin, a key bringing it
    straight back
  '';
  the-margin-cat-sits-above-its-column = ''
    the margin's cat sits a cell above the focused column's top edge over the middle of what's
    seen of it, beside the margin's marks rather than on them, nowhere in fullscreen or a gap too
    shallow, and is drawn in whole device pixels about two thirds of the gap tall; its walk turns,
    thinks, crouches back, eases in and out with strides stepped by the ground covered,
    overshoots, steps back and crouches as it stops, hurries far and stretches with the slowdown;
    it's livelier the less Joel is engaged and still on a call, locked or drawn nowhere, its waits
    and plays at random, no play twice running, and its wander and hop kept to the margin
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
    the music is the player that started last, never playerctld's echo,
    and it shows itself as one row, on a key always, on a track unless held
  '';
  the-mx-brio-is-shared = ''
    among the machine's own udev rules, udev reads the MX Brio's, read after every rule tagging
    cameras for the seat and before the handing, keeping the Brio from the seat, marking it
    relayed and starting its relay on the capture node; the relay is bound to that node, takes
    its loopback under the Brio's name, and starts with its own plugins on the Brio's NV12 at
    1080p and 30 frames a second and on its MJPEG, taken only where NV12 can't be had; the
    loopback's holders file is named as the Brio is and removed when the Brio goes, the shell
    lists the Brio only while that file is there, the Brio's own watch writes in the hidden
    folder, never beside it, and the seat's give-back is dropped
  '';
  the-mx-brio-is-the-seats = ''
    a graphical machine runs no relay on the MX Brio, has no udev rule marking a camera relayed
    by any of udev's assignments or taking the Brio's seat tag off as the sharing's rule does, as a
    search with each pattern shown to find its line says, and starts at every boot, and at a
    switch bringing it in, the give-back of a Brio a relay held, so the seat opens the Brio itself
    while no role takes its relay in
  '';
  the-mx-brio-give-back-never-fails = ''
    the MX Brio's give-back, run under set -e against a stand-in udevadm, replays both of the
    Brio's nodes and never another camera a relay holds, even of its maker or model number, never
    triggers with no device, bounds each wait on udev to a second or more and to fit three times,
    and every wait together once, within its unit's limit, ends well when udev refuses a replay,
    queues part of a refused add, never settles, hasn't handled the arrival yet, loses the Brio's
    record or can't be read, naming the way out of each, adds the Brio back when its
    unit's timeout ends the run, says a Brio unplugged midway is the seat's once plugged in, skips
    a node unplugged as it's read, says it gave the Brio back only when udev's record of every node
    shows it tagged for the seat and unmarked and no camera was left unread, unrecorded or still
    being added, and that the seat has it only when udev recorded
    every camera, warning with the way out, named for its nodes, otherwise, waits out adds the boot
    still has queued rather than calling them lost, points at udevadm's own words, which it keeps,
    for an add it refused, and warns nothing, udevadm included, on a machine with no camera, a boot
    whose adds came late or a run that went right
  '';
  a-reply-to-the-void-says-where-it-went = ''
    Joel's line to the void is found in the talk it began and read once replied whole, where the
    void says it went is told apart from the time it can be undone by, and a take-back stands
    taken on the void's own undone, else told what it said in a line's few words
  '';
  the-talk-line-says-what-became-of-its-words = ''
    Super+S's line sends Joel's words with the window they're said over attached for the void,
    names who hears it quietly, and says inside itself that they were sent, the answer in their
    place, and taken back
  '';
  a-live-notice-asks-the-daemon = ''
    a notice the notices daemon serves reads as Quickshell's own did, its urgency in Quickshell's
    numbers, and each act on it is an ask of the daemon, an action's with the token niri gave
  '';
  the-mypy-path-keeps-a-folders-stubs = ''
    a folder named on jitsusama.mypy.paths reaches the mypy nix fmt runs with its stubs and its
    Python alone, not the other files beside them
  '';
  the-mypy-path-refuses-what-it-cant-read = ''
    jitsusama.mypy.paths takes a folder and a Python module or stub named alone, and refuses a
    module that isn't there, a file that isn't Python and a place given as a string, as nix
    fmt's wrapper is read
  '';
  the-notices-land-on-niri = ''
    the account's notices, on its real niri nested in a headless sway in the sandbox, draw a notice
    sent on the arrival line, niri's notices layer, an overlay that takes no keys and stays mapped
    while it's quiet, showing nothing, and what Super+Shift+N runs puts a critical one
    away unread in the history; and Google Drive's notice that answers a save is kept unmissed, by
    its x-answers hint, while its failure is missed
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
  the-passwords-answer-no-ipc = ''
    nothing of the Passwords place, the daemon interface it asks through, nor Asks, how it says a
    line was shown and the ready rows it acts through answers the shell's IPC, so no handler of
    its own reaches typing a password
  '';
  the-passwords-daemon-binds-no-socket = ''
    none of the passwords daemon's own source binds a socket of any kind, so it's asked only
    through its standard input, the shell's pipe, which agents reach until they're fenced
  '';
  the-passwords-place-copies-through-its-daemon-on-niri = ''
    on a nested niri, the Passwords place runs the passwords daemon as the shell's own child and
    asks it nothing until Joel steps in,
    then lists once as his, leads with the locked manager's row, whose Return unlocks it, one
    refused saying why the next time the launcher opens, types an entry's password on Return
    into the window the launcher opened over, landing on the row from the find first, copies it
    by Copy the Password through the daemon and Passwords' own wl-copy to the clipboard as the
    launcher goes, the margin wearing the clipboard's mark and the foot, opened again, counting
    to its clearing,
    nothing on screen saying it, reads an entry in full as the selection rests on it, never as
    Joel's own ask, its drawer never saying its password, and holds its acts in the design's order
    once it's stepped into, Open the Website handing a stand-in Chrome its website as the
    launcher goes, its foot saying 1Password offers to fill it, and a walk from outside the
    launcher finding no entry and asking nothing, with a stand-in for 1Password's manager
  '';
  the-passwords-place-off-gives-the-shell-no-daemon = ''
    while the launcher's Passwords place is off, the shell's own config names no passwords daemon
    command, so rewriting the off setting gives the shell nothing to run
  '';
  the-passwords-place-off-goes-into-its-drawer-on-niri = ''
    on a nested niri, the Passwords place off with no password manager set up holds its one row,
    whose Return, with nothing to open, puts the keys in its drawer, which says why
  '';
  the-passwords-place-says-why-its-off-on-niri = ''
    on a nested niri, with the Passwords place off as the module leaves it, Home's Passwords row
    and the place's one row each have a drawer saying why it's off and that Return opens a
    stand-in Chrome, where 1Password's extension fills in sign-ins
  '';
  the-passwords-show-every-figure-but-the-secret = ''
    each kind of entry wears its own mark, the favourites are listed first, then what changed
    lately, newest first, twenty at most, a row says its username at its side and names its
    password manager only while two are on the machine, an entry's acts come in the design's
    order, each only where it has what it needs, opening it in the browser its manager fills in
    only on an http or https website, its manager's link is filled only where the entry
    says every part, Type the Password leads only where there's a window under the launcher and
    the entry has a password, the foot saying why a type was refused, its drawer's eyebrow says
    its vault and kind, and its facts every figure but
    its username and its password, the design's first in its order, a hidden detail as hidden and
    never its value, a website sent in the clear as its health and a weak password as its strength,
    each in the warning's role, its password reused or breached as its health once every password
    is checked, its figures grouped, and nothing before, those needing attention by title, the
    foot counts a copy down to its clearing, or a one-time code to its lapsing, a manager's drawer
    in Programs says how many entries it holds in how many vaults and how many are favourites,
    each left out at none, and For You's one to-do for what the last check found stands while
    some login is breached or reused, is done once a later check finds none, says how many are
    breached then reused at its side, each left out at none, and is weighed as a thing to do
    ready now; and while the place is off, its one row opens the browser a manager fills, else
    a manager's own app, else nothing, its note saying which
  '';
  the-patches-from-linux-next-apply-exactly = ''
    each patch the XPS 14 takes from linux-next applies to its kernel's source, after every patch
    applied before it, with no offset or fuzz, as next.nix says they do
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
  the-processor-starts-with-its-fixed-microcode = ''
    the XPS 14's initrd is set to start with Intel microcode giving its processor, 06-cc-02 on
    platform 0x80, revision 0x11d or newer, which fixes functional issues close to the hang behind
    two of its resets
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
    asked once answered by Return alone and let go by any other key, y and n among them and
    reaching nothing, but not in its first moment,
    with an Escape that cancels a c or a count begun before it steps back, normal's moves through
    the rows as Vim's, with counts: j k, gg G, H M L and Ctrl+D U F B, its - out, its q, ZZ and
    Space w closing, its : opening the command line with the range a count before it gives, Tab
    to the place used last, and Space ? and F1 listing every key, while / ? n N search nothing on
    the rows; and Vim's d and y over the rows, dd 3dd d3j dgg dG yy y{motion}, each row's ending
    action and copy, the ask once before what costs a lot and can't be put back, the rest done
    at once, and yG or dG over 400 rows
    within half a key's 4 ms, u and Ctrl+R taking back and doing again as many changes as counted,
    a page's search by smartcase, and which keys act: a letter, a caret move, an edit, a count or
    a sequence begun on the line acting on nothing, Backspace leaving an empty find acting
  '';
  the-queue-is-heard-from-the-node = ''
    the machine queue is the harness's file as the node's record holds it, none of a version it
    doesn't read, and a write of only when it was written says nothing new
  '';
  the-rest-residency-adds-every-core = ''
    the XPS 14's journal line on rest adds each rest state over every core with rest states in its
    number's place, past state9 and cpu9, with each state's exit latency and the count of cores
    added, says plainly when the kernel has no cpuidle driver, and fails when the driver or its
    states can't be read or a core's states differ from the first core's in name or number
  '';
  the-sandboxes-door-trusts-no-one = ''
    the Nix daemon's door for sandboxes, run as its socket unit runs it, answers as a daemon and
    takes its caller as untrusted, so a sandbox passed it rather than the daemon's socket can't
    make itself root
  '';
  the-screen-learns-its-level-for-the-room = ''
    the backlight daemon learns a level for the light the sensor reads from the levels picked,
    sets the keyboard's light and the monitors' beside it, and stops a move when the lid shuts
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
    swayidle marks someone at the machine from any key or touch, never from its own start, and
    takes the mark away after two minutes without one or with the session, for a followed switch
    and the session moves to wait on
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
    what was taken lately reads its folder through the real programs daemon, and finds screenshots
    by their words from the screen-text daemon, on one
    connection made only at the first search and let go at rest, each text asked once and the last
    four answers kept, each screenshot found with the line that matched; and finds a few thousand
    files by name, newest first, as Recent does should the programs daemon not search
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
    kinds names both in every watch, a field asked deep is mended with only what moved inside it
    while one not asked is replaced whole, a kind with none is known to hold none once answered, a
    whole answer after a gap lets go what went in it, one whole only of some keys lets go only those
    that went, a handler of a told change reads when its line was heard while one of a watch's
    whole answer, or one it put off, reads none, and a connection the shell lets go with an ask in
    flight is said let go, as information rather than a warning
  '';
  the-desktop-reads-the-spend-watch = ''
    the AI spend piece draws what the watch wrote, and the tokens piece is there only while the
    watch's tokens warn, with its acts for a running session; the collection holds the rows
    spend-watch serve serves ready, puts what wants a look first, steps into each figure's parts,
    hides amounts while sharing and answers reports by their words; a budget's row, a window's
    and a cap's open the drawer served with it, and a period's its days and what its cuts went
    to most; acts are asked of the socket, spawning nothing; told by the socket, a change remakes
    only the rows it changed, and a watch never answered is drawn from what's held while still
    reading; and a row shown elsewhere reads as served, as none once the place has answered
    without it or while the screen is shared, before the quiet place's first answer too, and as
    can't say only before the first answer on a screen said unshared, reading no more of how
    Spend stands than it needs
  '';
  the-deep-rest-hold-starts-unless-turned-off = ''
    the XPS 14's hold keeping the cores from their deepest rest starts with the machine, and with
    its one setting turned off nothing wants, requires or upholds it
  '';
  the-deep-rest-hold-stops-short-of-c6 = ''
    the XPS 14's hold bounds the cores' exit latency below every rest state from the first C6 on
    and above every shallower one, by the laptop's cpuidle table as it read on 20 November
  '';
  the-desktop-speaks-its-javascript = ''
    the shell calls no method Quickshell's JavaScript lacks
  '';
  the-shell-compiles-once = ''
    the shell's own files, served on Quickshell's qs: scheme, are compiled once and kept in Qt's
    disk cache without syncing any to the disk, a second start compiles none of them, and a file
    that changed is compiled again
  '';
  the-shell-is-given-a-token-on-niri = ''
    the shell's QML asks niri for an activation token through Quickshell.activationToken and is
    given one rather than nothing
  '';
  the-shell-warns-of-no-dangling-screen-as-its-screen-goes = ''
    the account's whole shell, with a notice up as both of sway's screens are unplugged at once,
    a notice sent while there's none and a screen made again, warns of no dangling screen object
    and of no binding or handler that throws
  '';
  flat-windows-hold-no-depth-on-niri = ''
    a full-screen window drawn flat weighs at least a quarter of a 2880 by 1800 depth and
    stencil buffer less than one drawn with them, and niri shows it drawn in order, a square over
    its picture
  '';
  uncached-pictures-let-go-on-niri = ''
    a picture the shell reads uncached from a file, as the wallpaper's, lets the file's pixels
    go once it's a texture, rather than keeping them beside it, and is read again from its
    file for a window it moves to
  '';
  still-windows-draw-nothing-as-animations-start-on-niri = ''
    a window the shell shows standing still draws no frame while two other windows' squares
    start moving ten times, one by a NumberAnimation and one by an XAnimator, through Qt's
    threaded render loop, Qt logging its timer starting over all three windows straight after
    its animationStarted() within at least half of those starts; each moving window draws a
    frame in every start and at least two in all but one, and the still one draws when its
    colour changes
  '';
  the-shell-hands-over-only-once-its-surfaces-are-shut = ''
    the old shell is ended in a handover only once its launcher and Super+S's line are shut,
    even one come up while the new shell loaded or opened by a chord heard just after it was
    found shut, whose socket is put back, or by one sent before that a busy shell read only
    later; a locked screen restarts the shell instead, and a new shell that fails as it loads
    leaves the old one the unit's
  '';
  the-shell-takes-a-switchs-files-at-idle = ''
    after a switch the shell is handed over onto its new files once the launcher and Super+S's
    talk line have been shut 30 s, or past its bound only as the open one shuts, and restarted
    only if the handover refuses, and a shell already on its files, or one stopped, is left be
  '';
  the-shell-titles-its-window = ''
    a shell titles its window with its folder at each prompt and with the command's first line
    while one runs, any control character in it spaced out, so no two terminals read "zsh"
  '';
  the-shell-opens-enough-files = ''
    the account's zsh gives its commands and its prompt's git-engine a soft open-files limit of
    8192, or the hard limit where that's lower, never lowering a higher soft limit or moving the
    hard one, an unlimited one included
  '';
  the-shell-starts-nothing = ''
    the account's zsh starts only git-engine, its fifo and direnv, and each prompt after runs direnv
    alone
  '';
  the-shell-stays-light-on-niri = ''
    opening every place the desktop's chords open and typing at home, three rounds over, the
    third round keeps no more than a tenth more of the shell's JavaScript heap live than the
    second, as Qt's collector says after each closing's collection, and a closing says what
    jemalloc holds in the shell's own process, and no memfd backs the heap, so each of its
    pages is held once
  '';
  a-meter-stays-its-moment-once-seen = ''
    a meter's moment, at a tenth of its length in Qt's engine, is spent its length after the
    first frame shown of it, or after the last change once one has been, at its most unseen,
    and never once hidden, waiting anew for a frame when shown again
  '';
  a-meter-stays-its-moment-from-niris-frame = ''
    the screen's brightness picked while the nested niri is held stopped for 2.5 s, longer than
    a meter's moment, has the brightness meter drawn once niri goes on, its moment begun at
    niri's presented frame and not Qt's swapped one, and gone between 1.2 and 3 s after, so it
    ran from niri's frame and not from its cap
  '';
  the-shells-list-names-what-it-runs = ''
    the list of the shell's files its companion compares after a switch also names the command
    the shell runs and each setting in its environment, so a new Quickshell is taken too
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
    its order, a signed-out account alone, and no account once the daemon stops answering, a
    change remaking only the parts of the kinds its keys name, the account told again a minute
    on remaking none of the lists; says
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
  the-signers-sit-with-the-agents = ''
    the agents serving the agents' key and a realm's link key sit in agents.slice, spared by
    oomd there, with a TPM or without, and Joel's login key's agent stays with his programs
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
  the-surface-names-each-device-it-misses = ''
    every device the Surface Laptop 4 expects is one its sysfs was seen to hold, and `device-check`
    names each missing, and only it, once what its path matches is taken from a stand-in of that
    sysfs
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
  the-surface-touchscreen-keeps-off-a-removed-workqueue = ''
    the Surface Laptop 4's touchscreen driver, as its source stands once patched, names system_wq
    nowhere, the queue Linux 7.3 says it will remove and warns of once a boot
  '';
  hang-panic-sleeps-whole = ''
    hang-panic's sleep hook, run with an empty PATH against a stand-in of busctl, stops systemd's
    runtime watchdog for each step that hibernates and hands it back to systemd's own setting
    after, leaves it running through a suspend, suspend-then-hibernate's first step included,
    and touches nothing when it isn't running
  '';
  the-surface-watchdog-wakes-again = ''
    the Surface Laptop 4 arms systemd's runtime watchdog at 30 s, and its sleep hook, run with an
    empty PATH against a stand-in of busctl, stops that watchdog before a sleep and starts it again
    after at the same timeout, touching nothing when it isn't running
  '';
  the-viewfinder-keeps-its-own-keys = ''
    ZZ closes the camera's viewfinder, and Escape and q leave it open, even where a script in
    mpv's own folder binds Z, as the player's vi keys do, since the viewfinder loads none of them;
    and its window's title says what Return makes, Photo or Video, following Tab
  '';
  the-wallpaper-is-read-at-its-screens-pixels = ''
    the wallpaper's picture is read at its screen's own pixels, by niri's scale for the output
    where niri names one, 3840 by 2160 on a 4K screen at 1.5 rather than the 5120 by 2880 the
    screen's whole-number ratio gives, and by that ratio where niri doesn't, and never past the
    size a PNG's header says, which Qt otherwise reads up to fill by crop
  '';
  the-web-kit-carries-the-theme = ''
    the web kit's CSS holds every role of the machine's theme as its dark look and of core's light
    theme as its light one, a first view's three faces come in under 350 KB, and every type step,
    and code set in small text, starts at 12 px or more
  '';
  the-wifi-cases-fail-on-a-throw = ''
    the guard every Wi-Fi case runs over what the shell said fails a run holding a file's line that
    threw a TypeError, a ReferenceError or a RangeError, or a value Qt couldn't assign, and passes
    one holding only the launcher's own lines, an Error: in no file's line among them
  '';
  the-window-buffers-stay-under-load = ''
    the XPS 14's udev sets xe's `shrinker_non_purgeable_seeks` to 64, so reclaim under a build
    leaves window buffers alone
  '';
  theme-json-says-which-marks-turn = ''
    theme.json says of every mark whether it's turned, and turns only the down triangle, so a
    terminal reading it draws that one as pointing down rather than as the bare triangle
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
  transcripts-people-passes-clippy = ''
    clippy finds nothing in transcripts-people or its tests
  '';
  transcripts-people-says-each-call = ''
    transcripts-people answers an address at once with each call with it as the thing they met
    at, named, noted, acted on and drawn as the launcher's Transcripts show it, and again as
    call-transcripts' daemon says a call with it changed or went
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
    tuple, among them every command core runs against it and the flags its followers pass
  '';
  tuple-ends-its-flags = ''
    The real tuple takes a name that starts with a dash after "--" as a name, never a flag, for each
    command the launcher runs on a contact: call start, call join, and contacts favorite and
    unfavorite; and its script, which stops Tuple's daemon, refuses before reaching tuple from a
    person's shell in their home or a checkout, the repository's dev shell, a bare environment and
    a home holding its own runtime and data folders, and from the build with any one of its four
    signs moved, while run as the build is it reaches tuple
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
  tuple-says-when-ready = ''
    Tuple's service returns from starting at once, then, only once it's active and its log says the
    daemon can take a call, says how long that took from the log's first timed line, in the words
    health reads, under Tuple's name, and nothing after; past the wait, as a clock stepped either
    way gives, it says only that the signaller connected and when, and it says it gave up on a
    unit never active and on a caller's name posing as the line, each in words health doesn't
    take for ready; it outlasts an ask systemd never answers, serves daemons systemd can't choose
    between, and says so and leaves at once when systemd takes it for the unit's main process;
    and a unit whose name, level, ceiling, filters or output would keep that line or systemd's
    Started line from health is refused, each refusal proven first on a table of settings
  '';
  tuple-shares-through-the-portal = ''
    Tuple reads the portal setting where the switch writes it
  '';
  tuple-takes-no-real-time-priority = ''
    Tuple's daemon may take no real-time priority, and goes first by a nice below 0 and a CPU weight
    above 100
  '';
  twin-screens-each-take-their-own-windows-back = ''
    Reopen Them All, with two screens of one model back on new connectors, puts a window kept on
    one twin on the connector its session kept on the other didn't take, and one kept on the
    session's own twin with it; the window of a twin put back after the walk on the connector a
    window put back before it didn't take, unless a session took that connector, when the
    session's twin goes with it; gives windows no connector from sessions handed over outside a
    walk; keeps, for a walk joined after Joel's focus moved, the screen in use as it began; notes a
    window away from a screen named constructor or __proto__ as any other, without a throw; and
    ends each walk before the next case begins, starting nothing for a session in a gone folder
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
  update-to-main-follows-on-a-mac = ''
    a Mac's follower is its primary user's launchd agent, started again whenever follow ends and
    naming no store path; a Mac refuses an update during a call, refuses the follower's ask alone
    while mac-room.sh says it's short of memory, for two hours at most, ending that refusal with
    the readings it judged and a call's without the call check's cautions, as the config the Mac's
    daemon serves by says too; and the follower's switch waits while someone is at the Mac, for 45
    minutes at most
  '';
  update-to-main-opens-enough-files-on-a-mac = ''
    a Mac's update-to-main daemon and its follower may each open 8192 files, not launchd's 256, so
    git can read a checkout holding hundreds of packs, their hard limit left as launchd's
  '';
  update-to-main-passes-clippy = ''
    clippy finds nothing in update-to-main or its tests
  '';
  update-to-main-waits-for-a-macs-memory = ''
    a Mac has room for its update only from two readings a minute apart, both at 35% free or over
    with nothing swapped out between; a first, too close, stale or future reading, a share under
    the floor, a swap-out or a count started again is short, and what it can't read or keep can't
    tell, never room
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
    lacks main and a read of the host's API fails, saying what the read said, its private inputs
    fetched by their locks, refused when rewritten, and builds
    nothing without a token, each git it runs over HTTP ending a transfer stalled for a minute;
    and a followed input is built at its own signed main past the lock,
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
    goes or the watch is stopped; a holder's name is one printable line, and only a system unit its
    cgroup is in is left out, never a person's own unit of the same name; and while it streams,
    the running watch says every ten seconds the frames a second uvcvideo counted it giving to a
    holder it sees; its line adds them up across a stream started afresh between wakes, says a
    stream's end, as it stops, the camera goes or the watch is stopped, up to the last wake that
    saw it, even with no count, and counts the next from its own start, survives a log that takes
    no write, quotes each holder, gives the packets in
    error, each interface by number when there are several, why a count, format or rate is
    unknown, and the controls under the names v4l2-ctl gives them, a control it can't read said
    so, and one that timed out, or a list that stalled there, kept unasked, one asked again each
    minute in turn and all at the next stream; all asked by the kernel's own call numbers and
    fields; a stop's rate left out for a moment's stretch, and a count unread at a line not
    carried into the next; and a camera that goes as its unit is stopped said to have gone; and a
    file the watch couldn't write, a camera's streaming or idle or a loopback's, is written once
    it can be, its trouble said once as it begins and once as it clears, even when the loopback's
    nobody met it and its camera's watch clears it, and so is the node a camera's watch couldn't
    say beside it, its trouble said once too; and a loopback's file is said ready only once it and
    its node are written; and as a loopback's camera goes or its watch is stopped, its node file
    goes only once its nobody is written, and as its camera goes, one it couldn't remove is tried
    again until it can be; and a stop tries its nobody and node file once more, whether it comes
    with no camera or just as its camera went; and a loopback with no camera is said ready only
    once its node file is gone or can't be there; and a camera a loopback moves to is said held
    while its node file still says the camera before; and a wrong file nobody holds is tried twice
    as long each time, up to sixteen seconds, while a held camera is looked at each interval
  '';
  usb-camera-holders-forget-a-killed-run = ''
    each USB camera watch's unit, the relayed one's too, forgets what says its node in the folder it
    writes as it stops, so a run killed before removing its files leaves no camera said there
    unless a loopback has the node by then, and starts again when its run crashes, backing off to a
    minute, with no start limit to refuse a camera plugged in again
  '';
  usb-camera-holders-may-open-the-camera = ''
    the USB camera watch's unit lets it open a camera to read, under its ProtectClock, with no
    private devices hiding the camera, and no device it names given write or mknod, an entry
    without access counting as all of it
  '';
  v4l2loopback-holders-passes-clippy = ''
    clippy finds nothing in the cameras' holders watcher or its tests
  '';
  vm-firmware-records-reach-an-account = ''
    a boot copies the firmware's platform health table, its boot error record table and the
    error region that table points at, and the machine's serial number, under the boot's own
    folder, where an administrator reads them and an account that isn't one reads none, with
    when the boot began, which a later run of the unit puts right; firmware-errors lists the boot
    as the one running now, begun when the kernel says, in text and in its JSON, says what the
    kernel's own region holds without calling it a reset, gives the boot in full by the start of
    its id, and refuses an account that isn't an administrator, naming the wheel group, never
    root
  '';
  vm-firmware-asks-an-administrator = ''
    in an administrator's active, local session, writing firmware, changing a device's settings and
    restarting into the firmware's setup need an administrator's answer, kept a few minutes, and a
    security fix or undo one each time; a plain restart still needs none; and the password typed
    through polkit's sign-in, with a YubiKey named but not there, answers for that action alone
  '';
  vm-admins-switch-unasked = ''
    an account in wheel switches to another system the machine has run and back with no password, and
    to one it never ran only by authenticating; one outside wheel, and what isn't a NixOS system in
    the store, is refused; and with a YubiKey named but not there, polkit takes the password alone
  '';
  vm-a-switch-outruns-home-manager = ''
    a switch that changes a system service and gives an account a user unit whose start never ends
    has that service running again within 45 s, while home-manager's setup of the account still
    waits on the unit
  '';
  vm-a-fenced-run-stays-fenced = ''
    an agent's run, started through the launcher a fenced or strict fence declares, as the node starts
    it, works in its folder but reaches none of the user bus, the display, the node, an SSH agent and
    an abstract X socket the account reaches outside, reads no key or GitHub sign-in, writes nothing
    outside its folder, and runs nothing in a folder that is home or holds it
  '';
  vm-a-focused-window-reads-its-swap-back = ''
    an administrator's memory-swap-in@ reads back what swap holds of a pid's scope when it belongs to
    the session window niri says has the focus, once a window in 30 s; never another window's, a
    window that's no session, root's, or for an account outside wheel
  '';
  vm-admins-tune-memory = ''
    an account in wheel sets the watermarks' spacing, transparent huge pages, the multi-gen LRU,
    swappiness, swap's read-ahead, proactive compaction, the free memory floor and defrag_mode to a
    listed value with no password; one outside wheel can't, and an unlisted value needs one; the
    machine boots swapping at 60 and compacting ahead at 40
  '';
  vm-admins-tune-the-work-slices = ''
    an account in wheel sets build.slice's and machine.slice's weight and cores for the boot with no
    password; one outside wheel can't, and no other unit is opened
  '';
  vm-apps-come-first = ''
    with no disk named, the system's user.slice holds a 1 ms latency on the drive behind /home,
    through the kernel, and no slice inside the account, machine or system does; every slice's
    io.weight follows its CPU weight;
    app.slice holds a hard 13% of memory and a soft 20%, session.slice a hard and a soft 4.5 GiB,
    agents.slice a soft 10% and a hard 2 GiB, every slice above them a hard 24% and a soft 38%,
    and jobs is slowed past 26%, a ceiling the sorter raises while jobs fills it and nothing it
    spares waits on memory, and that its unit, not its fall, puts back as it stops; timed work
    sits at 10000 inside idle jobs, and work in jobs can move itself there; a unit work in jobs
    starts with systemd-run, naming no slice, lands in jobs-work.slice; a scope it starts through
    systemd's own systemd-run goes back to the scope of the work that started it, and one with a
    memory ceiling stays; each command written into jobs.service gets a scope of its own in
    jobs-work.slice, at weight 100, with its own swap, and the holder stays;
    one naming a quest pi's sessions weigh goes in that quest's slice under jobs-work.slice at its
    weight for CPU and disk, which follows the file as it changes, and one naming none or an
    unlisted quest stays; two quests' work on one CPU shares it four to one by weight, and one
    alone takes it all; with tiers declared, a quest's work goes in the tier of its nearest ranked
    quest, through the harness's parents file, or in jobs-work-unranked.slice, each tier weighing
    half the one above from 640, its work's OOM score ten lower for each tier above the
    unranked one; a quest whose parent changes tier is said to move and its next work follows;
    and the first tier's work runs about four times the third's on one CPU
  '';
  vm-a-pis-busy-work-goes-to-jobs = ''
    a busy command a pi host's unit in agents.slice started goes to a scope of its own in
    jobs-work.slice, with what it started in a session of its own, first for the OOM killer, while
    the host stays in its unit and a session beside it in agents.slice wakes within 100 ms; and
    while a call program records in the account's PipeWire, build.slice and jobs.slice hold a
    quarter of the processors, each is braked again when its quota is cleared mid-call, and the
    sorter lets go of both once the call ends
  '';
  vm-a-measured-disk-shares-by-its-model = ''
    with a disk named with a model, io.cost holds its model and qos, no slice holds a latency
    target, one left from before is cleared as io.cost turns on, and stopping its
    `disk-order-<disk>` unit turns io.cost off
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
    in its record and under the Summary heading of the call's one file, over its transcript,
    with no summary.md beside it, the model's process and the delay are let go after, and a
    notice from Calls says the summary is ready, whose Read It opens the file in the session's
    reader;
    a notice raised as the call started offers the line for its chat, and the next meeting,
    stopped from that notice's Stop Transcribing, lets its recorders go with the call still on
    and isn't recorded once rejoined; a call holds logind's lock against going idle while it's
    on, transcribed or not, and lets go once it ends
  '';
  vm-a-call-keeps-its-packets = ''
    behind a router sharing 90 Mbit/s down and 15 up flow by flow under fq_codel, with the jobs
    downloading and uploading and an app downloading, two calls both ways at 2.5 Mbit/s, one in
    the video tin and one to 443, lose at most a hundredth of their datagrams coming down before
    the client's socket while a call is on, the shaper holding the download to nine tenths of
    the line's declared 75 Mbit/s while they run, its own queue dropping what the router's
    would, and letting it go straight in once the call ends; with the upload held
    at 6 Mbit/s and an app uploading beside its video call, the shaper drops at most a hundredth
    of the call going up; and while a call is on, an app's call to 443 goes in the video tin and
    is seen as real-time traffic
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
    10 dB, and once the recording stops the program says it rests and stops hearing it; Choose
    by Ear records the default microphone, offers it as four clips named by letter alone, and a
    pick names every clip's cleaner and its cost and, cleaned, makes the twin WirePlumber's chosen
    microphone
  '';
  vm-a-yubikey-answers-only-once-in = ''
    with a FIDO2 key plugged in and named, polkit takes its touch alone for run0 from a terminal on
    the seat's desktop while the lock is let go, asks for the password, which still works, while
    the lock is up or has said nothing, and never takes the touch from a text console signed in
    beside the desktop or an ssh login
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
    disabling it, and the host the account's Chrome source names lists two pages it opened by
    its ids, and the source's commands, finding the host through the file the module writes,
    bring one to the front and close the other; and with no extension asked, chrome-session reads
    two pages it opened from the session file it writes, each with its address, in one window
    with one in front, looked at lately
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
    commits are signed with the TPM's keys, PINs asked for; the agents' socket stays off until
    their key is made and listens once it is; stopped by hand, it listens and serves
    again, and stays off once its path unit is stopped first, until that unit starts again
  '';
  vm-desk-facts-follow-a-replug = ''
    desk-facts names a USB sound card plugged into a running machine, stops naming it once it's
    pulled out, and names it again when it's plugged back in; names a card not on USB by each of
    its devices, saying what one plays at only while it plays; and names a USB keyboard with its
    polling rate until it's pulled out
  '';
  vm-desktop-answers-secret-asks = ''
    the passwords daemon the shell runs answers an agent's secret ask drawn in the launcher's Asks:
    Joel's Return twice releases the stand-in value into the one command asked for, which no
    journal holds, his r refuses it and a Return on it then closes, throwing nothing, a launcher
    opened from outside takes no yes until his key moves over the rows, nor after an agent's run
    or walk onto Asks until his own Escape comes after it, and a yes walked through the
    launcher's IPC releases nothing
  '';
  vm-desktop-captures = ''
    the desktop takes a screen, a window, a region, a recording and the camera's pictures and
    videos by the keys, and says how long a recording has run; and near a meeting no agent's
    slice may open the camera while Joel's programs in app.slice still do
  '';
  vm-desktop-keeps-copies = ''
    the desktop keeps what's copied, text and pictures, past the programs that copied it, Super+V
    gives it back into the window it opened over, and the launcher forgets it all when asked; and a
    copy marked secret reaches Joel's own X11 client but never a command fenced as pi fences one,
    given the display: handed the cookie it reaches no socket, with the sockets open it holds no
    cookie, and with both walls gone it reads it; and a password Passwords copies by its Copy the
    Password, each copy proven its own, pastes into a terminal and into a page, Super+V never
    lists it and the history keeps nothing of it, a command whose wl-paste is the fence's shim is
    refused it by the clipboard door while an ordinary copy after it reaches that command, and
    it's held till about 30 s and then gone
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
    after it weighs none of the words file's verbs again; and through 40 windows retitled 100
    times a second, every core busy, each of twenty openings is ready to draw its first
    frame within 50 ms of the ask, its own work, the settling after it and the wait for the
    frame, drawn from what the closed launcher kept and filled in without moving a row or the
    cursor, and its drawing takes within twice its time with the load frozen and 50 ms more;
    the keys bar's share of the frozen time stretched by as many times slower than the slowest
    normal builder's as the keys subtest's frozen loops ran, at most one and a half, and none on
    a normal builder, while the storm subtest fails if its storm's windows fell before its
    openings were timed;
    each printing, passed or failed, what the guest counted its builder stole in each turn,
    which no bar reads; and
    with every thread of the VM held to two of the builder's processors under load, the guest
    counts at least a quarter of its busy time stolen
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
    pressed; refreshes the list no oftener than its refreshes' costs earn and never on a key's
    path; and
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
    session from the shell's start and hold still as one changes, and pi's runs are found; and
    the safe word's Slack reader stays down, telling For You nothing of its own
  '';
  vm-desktop-locks-and-asks = ''
    the lock screen takes only the right password, kills a hung check and tries it again, lets go
    when released or told to by logind, and holds through a shell's restart; polkit's prompt and
    the askpass answer and hold, and polkit's prompt comes up after polkit restarts three times
    under the shell and after a hand-over that waited past 10 s for the launcher to close
  '';
  vm-desktop-looks = ''
    on the laptop's screen at 1.5 and at 2, a session sits a step in under its orchestrator, a
    meter stands over dictation rather than on it, AI spend's budgets stand behind one row, and
    what's found by its meaning sits below what typing found; an unplugged ErgoDox is shown
    nowhere until it's back; the Home Network and Nearby list what's around; and WARP, from the
    real warp over a stand-in warp-cli, stands in Network connected, degraded and not answering,
    each shot for a person to judge
  '';
  vm-desktop-looks-file-picker = ''
    on the laptop's screen at 1.5 and at 2, the file picker's plate previews each kind of file on
    the card's lines; a picker for one file asks for one and says Return opens it, a save is
    named on the line and a name taken asks once, :arga gathers files across folders, and :sort
    lists a folder in the order chosen, folders first
  '';
  vm-desktop-looks-machine = ''
    on the laptop's screen at 1.5 and at 2, Machine's processor, memory, disk and uptime step
    into the parts the machine daemon reads from the guest, and those parts into their own;
    Machines holds this machine's row first, saying its model and system as its realm's node
    measured them; Heat opens a chip's drawer, the Charger a USB-C port's, Firmware an offered
    update's, its Security Level says what falls short and steps into each attribute, and
    Sleep says where it failed
  '';
  vm-desktop-looks-plates = ''
    on the laptop's screen at 1.5 and at 2, the launcher's drawer takes the room beside the card,
    up to 520, and every line of it lies on one of the card's to a hundredth of a pixel: its
    eyebrow on the field's centre, its title on the first row's baseline, and its facts and words
    a whole number of half rows below, for a notice, a time server and a device; a person's
    Unread and Recent say how many at their right; a chord's plate gives its keys and where it
    works, each fact beside its name; and Now's plate sets its head as one unit and
    its groups as a body under it
  '';
  vm-desktop-looks-programs = ''
    on the laptop's screen at 1.5 and at 2, Drive's places and Recent, Spotify and its podcasts,
    what mpv stopped, GIMP's acts and Recent, Neovim's files open in two kitty windows, Chrome's
    tab and latest download, Tuple's Recent, the VMs, kitty's repositories and machines, btop's
    drawer saying it runs in a terminal, Shortcuts over Mail and a Docs tab, Passwords, an entry
    and its copy in the margin, and 1Password's program
    holding Check Every Password after its vaults, each drawn for a person to judge
  '';
  vm-desktop-looks-rows = ''
    on the laptop's screen at 1.5 and at 2, a found row's note is in the serif a step after its
    name and on its baseline to half a device pixel, and an empty list's cat stands in the pose
    its nothing means, each cell whole pixels of the screen and starting on one
  '';
  vm-desktop-looks-vi = ''
    on the laptop's screen at 1.5 and at 2, y flashes the rows it copies and d's ask stands on the
    line, visual counts its rows while a window arriving waits in the foot, space-right goes into
    the key list's drawer with its rule on the cursor's line, and the drawer's search and visual
    are shot for a person to judge
  '';
  vm-desktop-minds-the-margin = ''
    the margin shows keeping awake, lid shut too, a camera opened, tokens running out, do not
    disturb, For You's tray lit by a critical notice unread, a wait lasting a minute, the muted
    microphone, dictation, what's playing and caps lock, and Now says each and answers the keys
  '';
  vm-desktop-moves-keep-their-window = ''
    a session the launcher holds moves into pi's host in its own window, or carries on as plain pi
    there when the host won't take it or later releases it, keeping the window's id, workspace,
    column and size, taking
    no focus, changing no pixel and losing no key; a viewer its host refuses attaches again in that
    window; the node gives the session that window, so no second viewer opens and Go To goes to it
  '';
  vm-desktop-opens-and-sets = ''
    the launcher sets sound and opens its outputs' and microphones' parts, chooses a microphone's
    noise cancelling by ear over the trial daemon's socket, works a program's media,
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
    hands the shell over once that stop is done, reloading its unit, never stopping it, onto a
    new main process; and a session's window opened in that kitty is told nothing systemd tells
    the kitty of its own unit, its runtime folder among it
  '';
  vm-desktop-reopens-under-orchestrators = ''
    sessions come back, or start, on their orchestrator's workspace by themselves: what
    restore.json kept of two orchestrations goes where the shell kept each window, two stacked in
    one column too, and one its orchestrator hands over again is begun once, and one started for
    another's tree opens with it, in title order after its orchestrator, one pi's host brought
    back opens as its viewer alone, and ten the host gave back open as plain pis where their
    viewers sat this boot, with Joel's window keeping the keys
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
    the desktop boots without an ordering cycle and its shell without an error, each bus finds
    each service once, niri alone keeps
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
  chunks-are-given-back-each-night = ''
    the root's empty data chunks are given back from a oneshot unit on a 03:00 timer that starts
    with the machine and isn't caught up on, bounded past its longest night, six hours from 01:00
    as the clocks go back, and weighted behind other work in the background slice
  '';
  chunks-are-given-back-only-while-short = ''
    Btrfs's data chunks are balanced only from 01:00 to 06:00 and while too little of the disk is
    in no chunk, each step ended at 06:00 by the epoch as the clocks go forward or back, stopping
    at the first step that frees enough, going past a step that fails, stopping at half-full
    chunks, saying files must go only when the last finished or failed for want of room,
    whatever an earlier step met, ending well when left short only by full chunks or 06:00, and
    failing when no step finished, the half-full one was killed by day or failed for anything
    but room, or its usage can't be read
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
  vm-hibernation-resumes-through-its-boot-entry = ''
    a machine started through systemd-boot from an entry whose initrd isn't its default's, as
    between a switch and its reboot, hibernates and comes back from its image through that same
    entry, the one-shot entry hibernate-unlock set for it gone after
  '';
  vm-hot-folders-get-subvolumes-of-their-own = ''
    on a machine whose /home is Btrfs, each of an account's folders named by its modules is
    moved by a unit of its own as the account's manager starts, before the units that write in
    it: a free one becomes a subvolume's root with its files, hard links and links, the folder as
    it was kept beside it; one a process of the account's holds open stays and says so; one a
    program opens as it's copied stays whole; each a process held moves by itself once that
    process quits; a missing one passes; one that can't be copied is left whole with no half
    copy, its unit failing and its writer starting all the same; a second run leaves a
    subvolume as it is; and a copy marked for its late writes, or a split, keeps its folder and
    fails each run, though a mark with nothing written since its copy began is cleared; and a
    waiter that held nothing a minute three times running starts the move no more, its count cleared
    once a start lands the move
  '';
  vm-joel-comes-first = ''
    on an eighth of the laptop's memory, with agents' sessions, their work and shared work each under
    its slice's ceiling and walking what it holds, a stand-in shell in session.slice waits no
    more than a frame's 16 ms on memory, and no program reclaims for itself, through a fetch in
    agents.slice and a grep and three commands in jobs.slice bursting until together they pass
    memory by a fifth, as at 12 November's 16:33 freeze
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
  vm-a-switch-takes-a-plugged-in-brio = ''
    a switch that brings in the MX Brio's sharing over a Brio already plugged in and handed to the
    seat names it for its relay, starts the relay, and takes it back from the seat, with nobody
    plugging it in again; and a switch back to the seat's machine gives the seat the Brio again,
    tagged for it, unmarked and watched as its own, with nobody replugging it
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
  vm-net-reads-the-switch = ''
    net's switch reader starts signed out with no credential, signs in once when one is written
    and reads all eight ports; Joel's account reads its report but not the password, and the
    password is in no report or journal line
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
  vm-cores-kept-from-their-owner = ''
    the cores' folder is root's alone from boot and after each crash, so an account that aborts
    three of its own processes, stand-ins named pi and node and sleep, can neither open nor dump
    a core, while the journal still shows it, in wheel, each crash's stack trace; the removal
    keeps a pi's and a node's cores five hours on and takes them seven hours on, leaving sleep's
  '';
  vm-a-wedged-pi-starts-again = ''
    a realm's background pi host, its unit as the module renders it on a stand-in node, is
    killed by the watchdog and started again once its loop stops pinging, its abort's core refused
    by systemd-coredump under a hard core limit of 0, and killed 10 s on when it ignores that
    abort, and one that
    crashes
    at every start is started three times, then left failed with start-limit-hit, and one that
    refuses what it was given is left down at once, each saying which host is down and why, and
    one whose command the kernel kills for memory stays up, unrestarted
  '';
  vm-pi-conversations-come-back = ''
    a realm's attended pi host, its unit as the module renders it on the harness's own host
    program and a pi with the host's patches, holding a parked and an idle conversation, is
    started again after a kill -9 holding both with their records unchanged, and after a reboot
    starts with the account's manager, ready and holding both, and a viewer attaching then is
    drawn each one's screen
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
  vm-a-hang-leaves-a-record = ''
    hang-panic arms the watchdog at a minute, stops it for a hibernation and has it armed again at
    once after, turns the NMI watchdog on over perf's off for a hard lockup to panic, not a soft
    one, and holds a panic's 10 s over the command line's; netconsole sends the kernel's lines to
    the listener's own hardware address, where the Mac's listener command keeps them and drops a
    line from outside its network, each line numbered and timed, refuses an address nothing
    answers for now though the machine remembers one, follows the listener to another card
    past the one remembered, makes a missing or turned-off target again at the next asking, keeps
    it while nothing has changed or the listener sleeps, its name unanswered too, lets it go while
    the card is off the listener's network or on another numbered the same, or another machine
    holds the address sent to while the name goes unanswered, starts with the system as a switch
    would start it, loading its module itself, stays up through a start the listener missed,
    which fails no switch, and sends once the following finds it, stops when its card is
    unplugged and sends again once it's back and renamed, and a panic's words and its 10 s
    arrive before the kernel restarts, sooner than the watchdog could
  '';
  vm-a-reset-leaves-its-last-seconds = ''
    last-seconds-keep runs as its own account and writes each second to the boot's ring, wheel's
    to read, where an administrator reads the boot as still recorded and another account is
    refused, and reads what the kernel gives through its sandbox, every rest state it lists
    included; a stop writes a record saying so and a restart goes on after it, a kill restarts
    it by itself, a sleep, as far as the kernel's own test of one freezes every process, waits
    until the recorder's record says it and a wake has it go on within seconds without counting
    the sleep, a power off stops it after a service that stops late, a machine reset while frozen
    in a sleep leaves its boot said asleep, and a machine killed as a reset
    kills it leaves its boot said cut short, its last second read kept
  '';
  vm-the-relay-waits-out-a-frameless-camera = ''
    the shared cameras' relay, read by a client, plays a camera's pipeline, makes a camera that
    never frames again 90 s after its client came, by its count and the journal's, and ends 90 s
    later, saying so each time, and makes one whose frames stop again 10 s after its last frame,
    each time it stops, never ending it
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
  vm-the-house-holds-the-upload = ''
    with another machine on the line running house-report as a service, naming no interface, and
    sending 3 Mbit/s across the home network, the shaper hears it through its firewall, its
    reports read 2.4 to 3.6 Mbit/s, and it holds its own upload under what the house's 8 Mbit/s
    leaves, never under 4, and says so; once the reporter's route moves to another address the
    house names, the reporter says so, the shaper hears the new address and the hold stands; the
    reports are signed, and ones signed under another key are refused and hold nothing; a key
    swapped on both machines is taken up and the hold stands again; once the reports stop it says
    the house went quiet and the upload climbs past the hold
  '';
  vm-the-cores-stay-out-of-deep-rest = ''
    a boot starts the XPS 14's hold keeping the cores from their deepest rest, bounding their exit
    latency at 20 us; stopped, the bound goes at once; started by hand, it holds by the time its
    start returns; a hold killed by TERM or KILL comes back; and the journal's line on how long
    each rest state ran, written in its confinement over haltpoll's rest states, names the states
    root sees
  '';
  vm-the-desk-lights-come-back = ''
    a Litra plugged back in while a camera streams is told to light again, and the camera stopping
    puts it out; a Pebble X Plus plugged back in still has its lighting put out at a lock
  '';
  vm-the-code-page-reads-its-daemon = ''
    an account's sites' server, as its home module renders it, serves the code page with the strip
    its build writes, and the page's feed, asked from the page's origin as its kit asks, starts the
    stopped repos daemon through the socket systemd holds for it and says live with the repository
    it read on the disk under the root the account gives it; three commits git makes while one
    watch is open are each told on it as changed, with the checkout's new head, unasked; and the
    homes the daemon writes as it reads, answered beside the page, say where the repository is home
  '';
  vm-the-network-page-reads-its-daemon = ''
    an account's sites' server, as its home module renders it, serves the network page with the
    strip its build writes, and the page's feed, asked from the page's origin as its kit asks, says
    live with how each of net's sources stands, read over the socket systemd holds for it, eero's
    saying it hasn't written before anything is at the path eero's daemon writes; once eero's
    golden record is put there, the running net's next round tells the house's devices with eero
    answering; stopping net and its socket makes it say broken, and the socket back makes it live
  '';
  vm-the-system-page-reads-its-daemon = ''
    an account's sites' server, as its home module renders it, serves the system page with the strip
    its build writes and the kit it imports, and the page's feed, asked from the page's origin as its
    kit asks, says live with the summary the system daemon holds, read over the socket systemd holds
    for it; stopping the daemon and its socket makes it say broken, and the socket back makes it live
  '';
  vm-the-network-keeps-its-order = ''
    an account's jobs, background, the system's builds and a guest it forwards for go in CAKE's bulk
    tin both ways, the jobs' leaving marked CS1, what Joel waits on at a terminal and an app's pages best effort, its calls, its
    UDP to Meet's media servers on 443 and Tuple's daemon video, even what it leaves to send as
    it ends, names and the shaper's pings voice; a call's packets turn real-time on; a slice made
    again keeps its class; the client's own network isn't slowed; inside a WireGuard VPN the same
    classes reach the VPN's own CAKE, and the VPN's own packets go in a lane of their own on the
    connection even carrying only bulk, beside which bulk gets at least a fifth; the queues follow the default route, a link unplugged and
    plugged back in is shaped afresh both ways, udev sets nothing on the IFBs, and they go when the shaper stops
  '';
  vm-the-vpn-takes-the-quick-way = ''
    where a router sends two adjacent source ports a near way to the VPN's far end and every other
    a way 40 ms slower, the shaper probes the ports a hop past the split and says it pinned the
    near two; the VPN's socket from before keeps its port, its next lands on one of the two, and
    the one after, while that flow still holds its port, on the other; a restarted shaper keeps the
    pin and says so, a stopped one leaves none, and started again puts the same one back; with both
    held by other flows, the shaper says its pin let go and the VPN's next socket leaves from its
    own port
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
    wifid joins the networks it's given, off the frequencies Nix avoids on each, finds sign-ins
    and sessions ending, measures how well the access point it's on carries, hears an account say
    a call is on and names who said it, tells the shell what it hears in detail, chooses a network
    the launcher saved first, trusts one when asked, and moves a card whose access point carries
    slowly to another of its network
  '';
  vm-wifi-joins-when-asked = ''
    the shell has wifid sign in, join with a password or an account, leave, save and switch the
    radios; a stuck look is nudged, a silent access point passed, and the card dozes only on
    battery
  '';
  vm-wifi-moves = ''
    the card moves between the access points of one network: on at once from one that doesn't
    answer, to another when an account asks, keeping its neighbours, to another within seconds
    of its echoes being lost during a call, and to the one its point names when the point warns
    it drops the card, in time, keeping its chooser off the point. It stays for a plain ask, and
    says it. Warned, it looks over the named one's channel first when it isn't heard, stopping a
    look under way too; it stops a look to move when it is heard. It's let go of its point when
    the whole network drops it, let go rather than sent back when the point it's warned to
    doesn't answer, and back within wifid's 2 s for a way back past any wait the point bids, its
    other networks off, from one gone since it was heard, or free to join any network when it
    can't get back either; another network forgets them
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
    on comes back and warp-cli refusing in its words; and serving it, reads the link at once when
    a listening warp-cli tells a status or reason other than the last, and on nothing else
  '';
  wezterm-finds-its-font-itself = ''
    WezTerm draws in the theme's monospaced face from the files its own configuration names, on a
    machine with no fonts of its own, so a switch swapping the system's fonts can't raise a window
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
  wifi-asks-each-word-a-join-needs-first = ''
    the launcher's Wi-Fi joins a network by the acts wifid serves it: a password asked as a
    secret, an account's name, its server's domain and password in turn, each set in the ask and
    sent after the last; a step out partway sends nothing and says so; a join's steps and a
    refusal are said in the foot, never a card; no password is shown; and with reports off a
    refused join is journalled by its act and code, with no network's name, password or wifid's
    words on any launcher line
  '';
  wifi-holds-the-network-joined-ahead = ''
    the launcher's Wi-Fi holds the network joined ahead while the launcher's open and Wi-Fi's on,
    so its drawer is served before Joel steps onto it; a change to that drawer alone tells the
    launcher nothing while nobody looks at it, the row it holds drawing the drawer as served
    now, and one looked at, a drawer coming or going, or a change past it, is told; and no line
    throws, names nothing or goes unassigned while it does
  '';
  wifi-looks-only-while-its-rows-are-shown = ''
    the launcher's Wi-Fi asks wifid to look for networks only while its rows are shown: Wi-Fi or
    something in it held, or a find from the root showing a row of its, the launcher open, or a
    terminal's show walked into it; never open on another place, at home or on another's find,
    and at once on stepping in; and no line throws, names nothing or goes unassigned while it does
  '';
  wifi-names-a-network-without-a-country = ''
    core's kernel, with its own patches, records the name of the network a card joined even
    when its access point names no country, so networkd matches that network's file
  '';
  wifi-sessions-survive-fragmented-memory = ''
    the XPS 14's kernel lets Wi-Fi's block-ack reorder buffers come from vmalloc when memory is
    too fragmented for an order-5 block, and frees them nowhere with kfree()
  '';
  wifid-calls-passes-clippy = ''
    clippy finds nothing in what tells wifid of calls, or its tests
  '';
  wifid-calls-starts-at-a-switch = ''
    what tells wifid of calls starts with the account's session as well as with PipeWire, so a
    switch that brings it in starts it while PipeWire already runs
  '';
  wifid-calls-tells-wifid = ''
    what tells wifid of calls says nothing is on at its start, then a change or a call still on a
    minute later, as exactly a calling act wifid's rows socket takes, and says a refusal as it is
  '';
  wifid-comes-back-from-a-stalled-switch = ''
    a switch restarts wifid and wpa_supplicant in place, never stopping them first, so a switch
    that hangs later still leaves the card on its network
  '';
  wifid-passes-clippy = ''
    clippy finds nothing in wifid, its sign-in window or their tests
  '';
  wifid-reads-the-air = ''
    wifid's tests pass: what it hears and offers, how each network is joined, the addresses it
    shows, what it keeps, sign-ins, and what each access point carried by the hour
  '';
  wifid-sign-in-reads-its-network = ''
    the sign-in window's tests pass: its proxy lets on only the window, and it follows which card
    wants a sign-in
  '';
  windows-hears-niris-windows = ''
    the windows daemon says each window niri describes named by its program and placed as its row
    says it, the one used last first; tells a watch only the window a retitle changed and those it
    reads alike with, only the two windows a focus moved between, and nothing for a resize or the
    launcher taking the keys; says a window closed as gone, names a window's program again once an
    entry is added, describes the desktop again once niri is gone and back, and refuses an ask or
    a kind it doesn't know; times a program's first window from its process's start and waits
    for its process once its last window closes, looking ever more seldom once past the budget,
    and says nothing of a second window, a window there as the stream began or a process started
    long before
  '';
  windows-passes-clippy = ''
    clippy finds nothing in the windows daemon or its tests
  '';
  windows-reads-the-launchers-words = ''
    the launcher's vocabulary has every word the windows daemon words Screens' rows with: the
    built-in screen's name, how a screen stands and runs, and Turn Every Screen Off
  '';
  windows-runs-beside-the-shell = ''
    the shell brings the windows daemon, on the daemons' socket, which systemd holds whether or not
    it runs, restarted by a switch, handed the launcher's words
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
    jobs is slowed past 26% of memory and agents' sessions past 42%, with no hard limit that
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
