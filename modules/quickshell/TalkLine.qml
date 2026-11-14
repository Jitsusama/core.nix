// The line Joel says something on from anywhere, on Super+S: one frame a step inside the top
// of the focused screen, over the furniture of his windows (a browser's tabs, a terminal's
// oldest lines) rather than what he's reading, its typing the hero (Joel, 2026-11-13: "so many
// padding issues, no room to type"). What's typed wraps at a whole word, a line at a time,
// never scrolling off its start. Under it, quiet, who hears it, the void, which passes each
// line to whoever owns it, and the window it's said over, which goes with the words for the
// void to route it by (QCXZEB, 2026-11-13 [ae5dfdb6]). Return sends it and the line stays up:
// under a hairline, his last three lines, newest first, each with whom it waits on until
// every answer comes, all in this frame and never on a card elsewhere (VKCF3Z, 2026-11-13
// [092d52ca]; WCJQSG [cf8639ab]). Put away first, the answers wait here for its next opening. Where the void can undo what it did,
// :take in normal takes it back, the foot naming it while it's offered. It opens in insert;
// Escape gives normal and never puts it away; the chord again does, or :q, the words kept, as
// a switch keeps them (Drafts.qml). TalkLine.js says what it says and sends.
pragma ComponentBehavior: Bound
import qs
import Quickshell
import Quickshell.Io
import Quickshell.Wayland
import QtQuick
import KeyTimes
import "Columns.js" as Columns
import "TalkLine.js" as Said
import "When.js" as When

Scope {
    id: root

    // The launcher, whose drafts it keeps and whose Agents it says through.
    required property var launcher
    // Whether it's up, and on which screen: the one focused as the chord came.
    property bool up: false
    property var screen: null
    // Who hears it, `{ to, name }`: the void, or null while none stands at Agents' top.
    property var hears: null
    // What goes with a line of the window it's said over (TalkLine.js's attached), read as
    // it comes up; the session in it comes a moment later, from the agents daemon.
    property var window: null
    property var session: null
    readonly property var rows: Said.attached({
        window: root.window,
        session: root.session,
        tab: root.tab,
        registers: root.registers
    })
    // The rows shown of them under a hairline before it's sent, the window
    // left to a tab or a session that already names it (TalkLine.js).
    readonly property var shownRows: Said.shownRows(rows)
    // What Ctrl+R * and Ctrl+R + attached, the selection and the clipboard
    // as Vim names its registers, kept with the words until they're sent.
    property var registers: []
    // The tab in front of the Chrome window it's said over, as the programs daemon answers
    // it (askTab), or null.
    property var tab: null
    // The talks Returns in this shell began (TalkLine.js's begun), newest first, as each goes.
    property var saidHere: []
    // Every line the line shows: those said here and those the void keeps, which outlive the
    // shell, lines still open first (TalkLine.js's gathered); and the first, or null before
    // any, which :take acts on.
    readonly property var talks: Said.gathered(saidHere,
        heard.filter(one => !Said.unanswered(one)).map(one => Said.fromServed(hears.to, one)),
        events, seen)
    readonly property var talk: talks.length > 0 ? talks[0] : null

    // How many of each line's answers he has seen, by its key in the void's day file, kept
    // on disk so a restart shows again only what he hasn't; written as the line is put away,
    // what it showed then being what he saw (WCJQSG [a2eb9575]).
    property var seen: ({})
    FileView {
        id: seenFile
        path: Quickshell.statePath("talk-line-seen.json")
        blockLoading: true
        printErrors: false
    }
    Component.onCompleted: seen = Said.seenIn(seenFile.text())

    TalkLineMark {
        up: root.up
    }
    function sawShown() {
        const after = Said.seenAfter(seen, talks, events, Date.now());
        if (JSON.stringify(after) === JSON.stringify(seen))
            return;
        seen = after;
        seenFile.setText(JSON.stringify(after));
    }
    // Whether the command line has the keys, in the field's place.
    property bool commanding: false
    // What's typed on the line is kept under its own name until it's sent.
    readonly property string key: "line"
    readonly property var words: Words.of("agents:line")
    readonly property var said: words.said ?? {}
    readonly property string takes: String(Words.of("verb:agent:take-back").name ?? "")

    function agents() {
        return launcher.collections.find(one => one.settings.name === "agents") ?? null;
    }

    // Agents opens the line from the void's row, so it's told which line is this one.
    Binding {
        target: root.agents()
        property: "line"
        value: root
        when: root.agents() !== null
    }

    // Brought up from a row rather than its chord, and left up where it already is.
    function open() {
        if (!up)
            toggle(-1);
    }

    // `chord` is when launcher-say ran for it, on the wall clock in ms, or -1 where it didn't
    // say, as qs ipc doesn't.
    function toggle(chord) {
        if (up) {
            putAway();
            return;
        }
        // Read once, as the line comes up, in a function rather than a binding, so there's
        // nothing for a notify to update.
        const after = surface.FramePresentation.framesBegun; // qmllint disable stale-property-read
        timing = { heard: KeyTimes.now(), late: chord > 0 ? Math.max(0, Date.now() - chord) : -1,
            after };
        bringUp();
        surface.FramePresentation.ask();
    }

    // From its chord to niri presenting its first frame up, held to Joel's 50 ms (VKCF3Z,
    // e8b4139e), said once each time it comes up; null once said.
    property var timing: null
    function presented(frame, at) {
        if (timing === null)
            return;
        if (frame <= timing.after) {
            surface.FramePresentation.ask();
            return;
        }
        const shown = Math.round(at - timing.heard);
        const late = timing.late;
        console.info(late < 0 ? `talk line: up ${shown} ms after it was asked`
            : `talk line: up ${shown + late} ms after its chord, heard ${late} ms in`);
        timing = null;
    }

    function bringUp() {
        screen = launcher.focusedScreen;
        const from = agents();
        hears = from?.voidOf() ?? null;
        window = null;
        commanding = false;
        const kept = launcher.drafts.kept(key);
        prompt.text = kept.text;
        up = true;
        prompt.take();
        prompt.typing();
        if (kept.text !== "")
            prompt.caretTo(kept.caret);
        look();
    }

    // The window it's said over, read again while it's up as niri says more of it, since a
    // window opened a moment before the chord may not be listed yet, nor its title said.
    // Once the line's layer has the keys niri says no window has them, which leaves the
    // window it was said over as it was.
    // Opened again the moment it was put away, niri may not have given that window the keys
    // back yet, so it's said over the window it was last, while niri still lists it.
    // It's read again as its title, its own title or its program changes, the last two as
    // the windows daemon names them (core #4242).
    function look() {
        const id = Niri.focused ?? (window === null ? lastOver : null);
        const focused = id === null ? null : Niri.windows[id] ?? null;
        if (focused === null && window !== null)
            return;
        if ((focused?.id ?? null) === (window?.id ?? null)
            && lookedAs(focused) === looked)
            return;
        const asked = (focused?.id ?? null) !== (window?.id ?? null);
        window = focused;
        if (focused !== null)
            lastOver = focused.id;
        looked = lookedAs(focused);
        if (!asked)
            return;
        session = null;
        agents()?.sessionIn(focused?.id ?? null, found => {
            if (root.up && (root.window?.id ?? null) === (focused?.id ?? null))
                root.session = found;
        });
    }
    property string looked: ""
    function lookedAs(window) {
        return JSON.stringify([window?.title ?? "", window?.named ?? "", window?.program ?? ""]);
    }
    property var lastOver: null

    Connections {
        target: root.up ? Niri : null
        function onFocusedChanged() {
            root.look();
        }
        function onRevisionChanged() {
            root.look();
        }
        function onWindowsChanged() {
            root.look();
        }
    }

    // The tab in front of a Chrome window, asked of the programs daemon by niri's id for the
    // window as the line comes up over it, or as it's said over another, over a connection
    // held only while it's up over one, so it reads Chrome's tabs for nothing else. The
    // daemon matches the window's title to its front tab and answers none on a tie (FEH1HE,
    // core #4190). The tab stands while the window does, so a title the window changes as
    // it loads leaves the row as it was rather than dropping it to ask again.
    readonly property string browser: "google-chrome"
    readonly property bool browsing: up && (window?.app_id ?? "") === browser
    Daemon {
        id: fronts
        socket: `${Quickshell.env("XDG_RUNTIME_DIR")}/daemons/programs.sock`
        kind: "front"
        live: false
        connects: root.browsing
        onWireChanged: root.askTab()
    }
    onBrowsingChanged: askTab()
    onWindowChanged: askTab()
    function askTab() {
        const over = window?.id ?? null;
        if (!browsing || over === null) {
            tab = null;
            return;
        }
        if (tab !== null && tab.window === over)
            return;
        tab = null;
        if (fronts.wire === null)
            return;
        fronts.find({ ask: "get", keys: [Said.tabKey(browser, over)] }, answer => {
            if (root.browsing && (root.window?.id ?? null) === over)
                root.tab = Said.tabFrom(answer, root.browser, over);
        });
    }

    // The selection, read from the clipboard's history only as Ctrl+R * asks for it and
    // never kept but as what's attached, as On the Selection reads it (OnSelection.qml),
    // over a connection held only while the line is up.
    Daemon {
        id: selection
        socket: `${Quickshell.env("XDG_RUNTIME_DIR")}/clipboard-history.sock`
        kind: "selection"
        live: false
        connects: root.up
    }

    // Ctrl+R and a register: * attaches the selection, + the clipboard, each a row; any
    // other attaches nothing.
    function attach(name) {
        if (name === "+") {
            registers = Said.registered(registers, name, Copied.text, said);
            return;
        }
        if (name !== "*")
            return;
        selection.find({ ask: "get", keys: ["selection:primary"] }, answer => {
            const record = (answer?.records ?? []).find(one => one?.key === "selection:primary");
            if (record !== undefined)
                root.registers = Said.registered(root.registers, name, record.text, root.said);
        });
    }

    function putAway() {
        if (up)
            sawShown();
        launcher.drafts.flush();
        commanding = false;
        up = false;
    }

    // Return: the words go to the void with what's attached, and the line stays up, saying
    // under them how the talk stands. Nobody to hear it keeps the words on the line.
    function send() {
        const text = prompt.text.trim();
        if (text === "" || hears === null)
            return;
        const sent = Said.said(text, rows);
        launcher.drafts.handed(key, hears.to, sent);
        agents()?.say(hears, sent);
        saidHere = [Said.begun(hears.to, text, sent, Date.now())].concat(saidHere);
        registers = [];
        prompt.clear();
    }

    // Said again whenever a talk's line comes, its answer among them, up or not, so an
    // answer that came while it was put away is there as it opens.
    // It reads the talks as Agents folds them, so it hears them once folded rather than
    // when the sources change, which comes first.
    // The void's talks, which it keeps through restarts, are the line's too.
    readonly property var heard: hears === null ? [] : agents()?.talksOf(hears.to) ?? []
    onHeardChanged: follow()
    function follow() {
        let moved = false;
        const now = saidHere.map(one => {
            const next = Said.followed(one, heard);
            moved = moved || next !== null;
            return next ?? one;
        });
        if (moved)
            saidHere = now;
    }

    // What became of each line past the void, as the void writes it: every answer, one
    // superseding another, and who has it meanwhile (8GYNJ6 [a9490eb0]). The home realm's
    // node serves the void's file as one `void-answers` record, its lines in the order written
    // (modules/mycelium/home.nix), so the shell opens no file of the void's and is told each
    // line as the node hears it, rather than reading the whole file again on each. Watched only
    // while the line is up, the one time anything of it is drawn.
    readonly property var answersKind: answersLink.item
    LazyLoader {
        id: answersLink
        active: Sessions.nodePath !== ""

        Daemon {
            socket: Sessions.nodePath
            kind: "void-answers"
            live: root.up
            changesDiffer: true
            answersWhole: true
            keptThroughGap: true
        }
    }
    // The node's one record of the void's answers, keyed by the machine's name.
    readonly property var events: {
        void answersKind?.revision;
        return Said.eventsOf(Object.values(answersKind?.held ?? {})[0]?.lines);
    }
    // Each line as it shows, one said on an earlier day saying which beside its time.
    readonly property var exchanges: {
        const days = { yesterday: String(said.yesterday ?? ""),
            date: (date, format) => Qt.formatDate(date, format) };
        return talks.map(one => Said.exchange(one, events, said,
            since => When.day(since, Date.now(), days)));
    }

    // :take, while the void can undo what it did with the line.
    function take() {
        const asked = Said.taking(talk, Date.now());
        if (asked === null)
            return;
        // A line the void kept from before is said here from now, as it's taken back.
        saidHere = [asked.talk].concat(saidHere.filter(one => !Said.same(one, talk)));
        agents()?.talkTo(talk.to, asked.asked);
    }

    // What : runs: take, or q, which puts it away as the chord does.
    readonly property var menu: {
        const entry = (command, short, title) => ({ command, short, title, key: "", kind: "line",
            argument: "", after: "", lowered: ` ${title.toLowerCase()}`, bang: false,
            takes: false, ranged: false, undoable: false });
        const away = entry("quit", "q", "Put Away");
        return (talk?.undo ?? "") !== "" ? [entry("take", "take", takes), away] : [away];
    }

    function ran(read) {
        commanding = false;
        prompt.resume("normal");
        const command = read.entry?.command ?? "";
        if (command === "take")
            take();
        else if (command === "quit")
            putAway();
    }

    // A screen going takes the line's surface with it, and when it comes back Qt moves the
    // window onto it by itself, so Quickshell sees no change of screen and never makes the
    // surface again: Super+S only flipped `up`, with nothing to draw or take the keys (Joel,
    // 2026-11-13 23:01, after the laptop's only screen went dark under the lock at 21:58). So
    // once the screens change and the one it's on is gone, it's put away, its draft kept, and
    // made again on the screen the keys are on, as the launcher's is (Launcher.qml's remake);
    // with no screen at all it waits unmade for the next, which changes the screens again.
    Connections {
        target: Quickshell
        function onScreensChanged() {
            Qt.callLater(root.remake);
        }
    }
    function remake() {
        const real = Quickshell.screens.filter(one => one.name !== "");
        if (surface.visible && real.includes(surface.screen))
            return;
        if (up)
            putAway();
        screen = null;
        surface.visible = false;
        surface.visible = real.length > 0;
    }

    PanelWindow {
        id: surface

        Counted { name: "talk-line" }
        visible: true
        screen: root.screen ?? root.launcher.focusedScreen
        anchors.top: true
        anchors.left: true
        // Quickshell's type files can't place margins, so qmllint can't.
        margins.top: root.up ? Theme.space.rung.screen - card.reach // qmllint disable unqualified
            : 1 - implicitHeight
        // The same holds for the left margin.
        margins.left: !root.up || !surface.screen ? 1 - implicitWidth // qmllint disable unqualified
            : Math.round((surface.screen.width - root.launcher.width) / 2) - card.reach
        implicitWidth: root.launcher.width + 2 * card.reach
        implicitHeight: card.height + 2 * card.reach
        exclusionMode: ExclusionMode.Ignore
        color: "transparent"
        mask: root.up ? card.area : nowhere
        BackgroundEffect.blurRegion: root.up ? card.area : nowhere
        WlrLayershell.layer: WlrLayer.Overlay
        WlrLayershell.keyboardFocus: root.up ? Niri.asksForKeys : WlrKeyboardFocus.None
        WlrLayershell.namespace: "talk-line"
        FramePresentation.onPresented: (frame, at) => root.presented(frame, at)
        FramePresentation.onDiscarded: frame => {
            if (root.timing !== null)
                surface.FramePresentation.ask();
        }

        Region {
            id: nowhere
        }

        Card {
            id: card
            x: reach
            y: reach
            width: root.launcher.width
            keys: root.up
            // Clear rather than hidden while put away, since a hidden field can't hold the
            // focus, and a letter typed as it comes would be lost.
            opacity: root.up ? 1 : 0

            // Everything at the inset on every side, measured to the ink: the field's first
            // capital at the top, the foot's baseline at the bottom (docs/design.md, The
            // Ladder). Down it, space counts in lines: each line of one thing a pitch on, each
            // group a beat further, and the talk a region of its own under a hairline.
            Item {
                id: body

                readonly property real pitch: Theme.level.figure.pitch
                readonly property real beat: Theme.space.beat
                readonly property real caps: prompt.lineBaseline - prompt.capsTop
                // Where what's typed ends: its last line's baseline.
                readonly property real typedEnds: caps + prompt.wrapsTall
                readonly property bool talking: root.talk !== null
                // What goes with it a region under what's typed, under a hairline halfway
                // between, so the room over it and under it is the same to the ink; the
                // talks another under that, the same way.
                readonly property bool attaching: root.shownRows.length > 0
                readonly property real attachedAt: typedEnds + 2 * pitch + beat
                readonly property real attachedRuleAt: Math.round((typedEnds + attachedAt - caps)
                    / 2)
                // The last row's baseline.
                readonly property real attachedEnds: attachedAt - pitch + attached.height
                readonly property real talksFrom: attaching ? attachedEnds : typedEnds
                readonly property real lineAt: talksFrom + 2 * pitch + beat
                readonly property real ruleAt: Math.round((talksFrom + lineAt - caps) / 2)
                // The last talk's last baseline.
                readonly property real talksEnd: lineAt - pitch + spoken.height
                readonly property real textAt: prompt.fieldX - card.inset

                width: card.contentWidth
                height: foot.y + foot.baselineOffset

                Item {
                    x: -card.inset
                    y: -prompt.capsTop
                    width: card.contentWidth + 2 * card.inset
                    height: prompt.height

                    Prompt {
                        id: prompt
                        width: parent.width
                        lineTall: body.pitch
                        height: body.pitch + wrapsTall
                        wraps: true
                        steady: true
                        modal: true
                        words: true
                        awake: root.up
                        taking: root.up && !root.commanding
                        opacity: root.commanding ? 0 : 1
                        fade: Options.caret.fade
                        stopsAfter: Options.caret.stopsAfter
                        wait: Options.keyTiming.wait
                        glyph: ""
                        lead: Theme.glyph[root.words.glyph ?? ""] ?? ""
                        leadHue: Theme.voice
                        // Its own frame, not the launcher's, so the lead stands at the inset
                        // as the first capital does, rather than on the launcher's columns.
                        glyphWidth: 0
                        // Held on a mark's column (Columns.js), as the launcher's lead is,
                        // so what's typed and what's attached start on one text column, a
                        // mark rung clear of the rows' marks under the lead.
                        leadColumn: Columns.across(0, 0, Columns.ofMark(Theme.level.name.pitch),
                            Theme.space.rung).text
                        hint: root.hears === null ? root.said.nobody ?? "" : root.words.ask ?? ""
                        aside: ""
                        registers: true
                        onRegistered: name => root.attach(name)
                        onAccepted: root.send()
                        onTextChanged: root.typed()
                        onCaretChanged: root.typed()
                        onCommanded: range => {
                            root.commanding = true;
                            commandLine.take(range, "");
                        }
                        // j and k in normal go down and up the lines, as many as counted.
                        onStepped: (act, count) => {
                            if (act === "next" || act === "previous")
                                spoken.scroll((act === "next" ? 1 : -1) * Math.max(1, count));
                        }
                    }

                    // The command line, the launcher's own, in the field's place while : is
                    // open, as Vim's : takes the line.
                    CommandLine {
                        id: commandLine
                        visible: root.commanding
                        width: parent.width
                        height: body.pitch
                        markMiddle: prompt.cellMiddle
                        typedAt: prompt.fieldX
                        menu: root.menu
                        onRan: read => root.ran(read)
                        onLetGo: how => {
                            root.commanding = false;
                            prompt.resume("normal");
                        }
                    }
                }

                // What's attached its region, inset to inset, never meeting the frame.
                Rectangle {
                    visible: body.attaching
                    y: body.attachedRuleAt
                    width: parent.width
                    height: Theme.border
                    color: Theme.edge
                }

                // What goes with the line once it's sent, a muted row each.
                TalkAttached {
                    id: attached
                    visible: body.attaching
                    y: body.attachedAt - body.pitch
                    width: parent.width
                    rows: root.shownRows
                    textAt: body.textAt
                }

                // The talk's region, inset to inset, never meeting the frame.
                Rectangle {
                    visible: body.talking
                    y: body.ruleAt
                    width: parent.width
                    height: Theme.border
                    color: Theme.edge
                }

                // The talks, newest first, as many as the screen has room for under the
                // field and over the foot, the rest a scroll away (TalkSpoken.qml).
                TalkSpoken {
                    id: spoken
                    visible: body.talking
                    y: body.lineAt - body.pitch
                    width: parent.width
                    exchanges: root.exchanges
                    textAt: body.textAt
                    gap: body.pitch
                    up: root.up
                    // The screen's room a screen rung from each edge, less the frame, the
                    // card's inset over and under, everything above them and the foot below.
                    room: (surface.screen?.height ?? Infinity) - 2 * Theme.space.rung.screen
                        - 2 * (card.inset + Theme.border) - y - body.pitch - body.beat
                }

                // The foot, a group under the last line: who hears it and the window it's
                // about, quiet, on the text's column; and, while the void can undo what it
                // did, :take at its right.
                Text {
                    id: foot
                    x: body.textAt
                    y: (body.talking ? body.talksEnd : body.talksFrom) + body.pitch + body.beat
                        - baselineOffset
                    width: (offer.visible ? offer.x - Theme.space.rung.groups : parent.width) - x
                    // Who hears it alone: what goes with it is its own rows above.
                    text: root.hears === null ? "" : root.said.hearer ?? ""
                    textFormat: Text.PlainText
                    color: Theme.muted
                    elide: Text.ElideRight
                    font.family: Theme.level.note.family
                    font.pointSize: Theme.level.note.pointSize
                }

                Row {
                    id: offer
                    visible: (root.talk?.undo ?? "") !== ""
                    x: parent.width - width
                    // A Row says no baseline of its own, so it says its words'.
                    baselineOffset: takeCommand.baselineOffset
                    anchors.baseline: foot.baseline
                    spacing: Theme.space.rung.mark

                    Text {
                        id: takeCommand
                        text: ":take"
                        color: Theme.text
                        font.family: Theme.level.note.family
                        font.pointSize: Theme.level.note.pointSize
                    }

                    Text {
                        text: root.takes.toLowerCase()
                        color: Theme.muted
                        font.family: Theme.level.note.family
                        font.pointSize: Theme.level.note.pointSize
                    }
                }
            }
        }
    }

    // How a talk reads under his words, for the launcher's showing: its answers, or else
    // whom it waits on, or how it stands.
    function standingOf(shown) {
        if (shown === null)
            return "";
        return shown.answers.length > 0 ? shown.answers.join("\n")
            : shown.waiting !== "" ? shown.waiting : shown.stands;
    }

    function typed() {
        if (up)
            launcher.drafts.typed(key, prompt.text, prompt.caret);
    }

    // What the line shows, for the launcher's showing: whether it's up with the keys, what's
    // typed, its mode and caret, who it names and what it's about, how the talk stands and
    // what it offers, and how far down the screen its card stands.
    function shown() {
        return { up, keys: up && prompt.Window.active, typed: prompt.text, mode: prompt.mode,
            caret: prompt.caret, hears: root.hears === null ? "" : root.said.hearer ?? "",
            about: foot.text, attached: root.shownRows.map(row => ({ kind: row.kind,
                text: row.text, note: row.note })), lines: prompt.typedLines, talk: root.talk?.said ?? "",
            standing: root.standingOf(root.exchanges[0] ?? null), talks: root.exchanges,
            takes: offer.visible, commanding,
            top: up ? surface.margins.top + card.reach : -1,
            bottom: up ? surface.margins.top + card.reach + card.height : -1,
            screen: surface.screen?.height ?? 0, scrolled: spoken.scrolled,
            over: { focused: Niri.focused, known: Object.keys(Niri.windows).length,
                title: root.window?.title ?? "" } };
    }
}
