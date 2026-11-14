// The notices on screen, as the notices daemon serves them (Notices.qml):
// one stack in the top
// right corner, the oldest not yet dealt with in front and up to three
// newer behind it as edges, a critical one going to the front. A click on
// the front one opens what it's about, or puts it away when it's about
// nothing; one on an edge brings that one forward. The front one leaves by
// itself when its time is up, unless it's critical. While notices are held
// (Quiet.qml), only a critical one shows.
//
// The front one that still waits on Joel shows on the arrival line instead
// (NoticeLine.qml), at the top centre where the launcher stands, and the
// stack draws no notice while it does, nor while Super+S's line stands
// there, so each is in one place. The stack stays, counting each one's time,
// until the line has run a day on its own (2RCBOJ, 2026-11-13).
//
// Super+N gives the stack the keys while there are notices, and opens the
// history in the launcher when there are none; pressed again, it lets them
// go. Held, the stack is Vim's quickfix list, a line a card, and every key
// is Vim's (HeldNotices.qml).
//
// The notices sit a step inside the window under the top right corner, so
// a notice lies on one window rather than across a border, or a step inside
// where a window's corner would be when there's none (`inside` says how),
// clear of the margin's marks, which stand at the top centre in the gap
// above the windows.
//
// Each is kept in the history as it arrives, by the daemon, for the
// launcher to find later, where the ones still here can be answered
// or forgotten, and while it's open the corner leaves them to it. What a
// notice offers to do (Reply, Play it) leads its verbs, reached by keys.
//
// It's the notice kind (docs/design.md, Shape and Kinds): the edge frame,
// the accent's while the stack holds the keys, or the alert's when
// critical; and three groups on the launcher's line, the same room from
// the frame to the ink on every side, each at its type's level (Type):
// who or what first, at Name in strong, then the message at Words, and a
// signature a size smaller, at Note, saying what sent it, its mark in a
// column of its own, and when.
//
// What's playing (Music.qml) is a notice too, for a moment when a track
// changes or a media key is pressed, at the top of the same column so the
// two never cover each other, in the same three groups: the cover beside
// the title and the artist, and the player and where in the track. It's no
// message, so it isn't kept; a click lets it go.
pragma ComponentBehavior: Bound
import Quickshell
import Quickshell.Io
import Quickshell.Services.Notifications
import Quickshell.Wayland
import QtQuick
import "Columns.js" as Columns
import "LiveNotice.js" as LiveNotice
import "When.js" as When
import "Stack.js" as Stack

Scope {
    id: root

    // Wide enough that what a notice says holds about 40 letters of the
    // reading face at the launcher's size on a line, beside its mark: a
    // short message in one or two lines (docs/design.md). A letter is the
    // lowercase alphabet's average, as a measure is set in type, since the
    // face's own average counts capitals and runs a fifth wide. Each card
    // is the one card (Card.qml), its ink the inset from its frame on every
    // side, and its columns are a notice's (Columns.js).
    readonly property int letters: 40
    readonly property int width: Math.ceil(Columns.cardFor(
        letters * alphabet.advanceWidth / alphabet.text.length,
        Columns.ofMark(Theme.level.name.pitch), Theme.space.rung, Theme.border))
    TextMetrics {
        id: alphabet
        font.family: Theme.level.words.family
        font.pointSize: Theme.level.words.pointSize
        font.weight: Theme.level.words.weight
        font.variableAxes: Theme.level.words.variableAxes
        text: "abcdefghijklmnopqrstuvwxyz"
    }
    // How far the signature's capitals clear the message above them: a
    // beat more than Note's pitch from the message's last baseline, less
    // the capitals' own height, so the signature reads as a group of its
    // own, never as the message's last line.
    readonly property real signed: Theme.level.note.pitch + Theme.space.beat
        - noteCapital.tightBoundingRect.height

    // A capital in the face that says who a notice is from, so the room
    // above is measured to its top rather than to its line's.
    TextMetrics {
        id: capital
        font.family: Theme.level.name.family
        font.pointSize: Theme.level.name.pointSize
        font.weight: Theme.level.name.weight
        text: "H"
    }

    // The signature's face, to measure its ink by: a capital's, and where a
    // mark's, a name's and a time's ink starts and ends, since a
    // signature's gaps are measured to the ink.
    TextMetrics {
        id: noteCapital
        font: noteFace.font
        text: "H"
    }
    FontMetrics {
        id: noteFace
        font.family: Theme.level.note.family
        font.pointSize: Theme.level.note.pointSize
        font.weight: Theme.level.note.weight
    }

    // Whether what's playing is called for, as the arrival line's row is.
    // The card's own visibility can't say, since an item in a hidden window
    // counts as hidden.
    readonly property bool playing: Music.arrival !== null
    // What the card says, or nothing while there's no player to say it of.
    readonly property var song: Music.card ?? {
        title: "", artist: "", art: "", from: "", mark: "", where: ""
    }

    // The notices open, as Notices.qml hands them, in the order they
    // arrived: by id, which the daemon never gives twice, as an object's
    // whole-number keys are listed in rising order.
    readonly property var tracked: Object.values(Notices.live)

    // When each arrived, by id, for the footer's time, which rests longest
    // and the held list's order (HeldNotices.qml), as the daemon's
    // records say it. Quickshell's JavaScript has no Object.fromEntries.
    readonly property var arrived: {
        const at = {};
        for (const one of tracked)
            at[one.id] = Notices.made[one.id]?.at ?? 0;
        return at;
    }

    function arrivedAt(id) {
        return root.arrived[id] ?? 0;
    }

    // Where the notices sit: inside the window nearest the top right
    // corner of all those wholly on screen, the rightmost and then the
    // highest, their right edge and top each the frames rung within its
    // own, a frame on a frame, so a notice lies on one window and never
    // across a border, and no wider than fits in it; but never nearer the
    // screen's edge than the screen rung (docs/design.md, The Ladder).
    // With no window wholly on screen, or the nearest too narrow to hold a
    // notice, they sit the screen rung from the corner. niri says where a
    // window will come to rest (modules/niri/tiled-positions.patch), so
    // they move once a scroll settles, not with every frame.
    property int settled: 0
    readonly property var inside: {
        void settled;
        void Niri.revision;
        const step = Theme.space.rung.frames;
        const edge = Theme.space.rung.screen;
        const corner = { right: edge, top: edge, width: root.width };
        const screen = window.screen;
        if (!screen)
            return corner;
        const workspace = Object.values(Niri.workspaces)
            .find(w => w.output === screen.name && w.is_active);
        if (!workspace)
            return corner;
        const under = Object.values(Niri.windows).filter(w => w.workspace_id === workspace.id
            && w.layout && w.layout.tile_pos_in_workspace_view)
            .map(w => ({
                left: w.layout.tile_pos_in_workspace_view[0],
                top: w.layout.tile_pos_in_workspace_view[1],
                width: w.layout.tile_size[0]
            }))
            .filter(t => t.left >= 0 && t.left + t.width <= screen.width)
            .sort((a, b) => (b.left + b.width) - (a.left + a.width) || a.top - b.top)[0];
        if (!under)
            return corner;
        const right = Math.round(Math.max(screen.width - (under.left + under.width) + step, edge));
        const room = screen.width - right - (under.left + step);
        if (room < 2 * root.width / 3)
            return corner;
        return {
            right,
            top: Math.round(Math.max(under.top + step, edge)),
            width: Math.min(root.width, Math.floor(room))
        };
    }
    Connections {
        target: Niri

        function onLayoutsChanged() {
            root.settled++;
        }
    }
    // Whether anything's drawn, so the tiles are held only meanwhile: niri
    // says one moving at each resize and scroll, which woke the shell with
    // nothing up. Drawn only once they're held, so a notice opens beside
    // the window under it rather than moving there a moment later.
    readonly property bool drawing: shown.length > 0 || playing || holding
    onDrawingChanged: Niri.wantTiles("notices", drawing)
    // Led by how wide a card would be with room for it, which its words set.
    onInsideChanged: Report.say(
        `places notices ${root.width} ${inside.right} ${inside.top} ${inside.width}`)

    // The launcher, for the history the stack hands on to.
    property var launcher: null
    // Whether Super+S's line is up, where the arrival line stands, so only a critical notice
    // shows under it and the rest wait, counting no time, until it's put away.
    readonly property bool talking: launcher?.talking ?? false
    // Whether the arrival line shows the front notice, or the talk line stands where it
    // would, so the stack draws none of the notices and each is in one place.
    readonly property bool lined: line.up || talking

    // The notices on screen, as one stack: their ids, the front first, in
    // the order Stack.js keeps. Those resting off it (Notices.qml) are
    // still open, but not here.
    property var order: []
    readonly property var shown: Stack.shown(order,
        tracked.map(notification => notification.id)
            .filter(id => !(id in Notices.resting) && !held.awayIds.includes(id)))
    // How many may rest at once before the oldest is closed, so no program
    // is asked to hold its notices for ever.
    readonly property int rests: 30
    readonly property var front: {
        const id = shown[0];
        return tracked.find(notification => notification.id === id) ?? null;
    }
    // How many behind the front show their edge, lower and narrower each
    // on both sides, and by how much: eight hairlines, a stroke that
    // draws the stack rather than a gap between inks.
    readonly property int behind: 3
    readonly property int peek: 8 * Theme.border
    // Whether the stack glides as it moves and its edges as they follow a
    // new front card, rather than jumping: the one line to turn them off.
    readonly property bool glides: true
    // The front card's height, and the one the cards behind it take, which
    // follows it in a glide, so the edges peeking out beneath don't jump
    // as a card of another size comes to the front. The front card itself
    // is its own height at once, since its words are drawn whole.
    property real frontHeight: 0
    property real edgeHeight: frontHeight
    Behavior on edgeHeight {
        enabled: root.glides
        NumberAnimation {
            duration: Theme.motion.short
            easing.type: Easing.BezierSpline
            easing.bezierCurve: Theme.motion.curve
        }
    }
    // Whether the stack holds the keys, which Super+N gives it while there
    // are notices, as the quickfix list HeldNotices.qml draws.
    property bool holding: false
    // Whether Joel's away: nothing typed or moved for a while, whatever
    // keeps the screen awake, since a call or Keep Awake does that while
    // he's gone too. A card waits while he's away, so one that comes while
    // he's gone is there when he's back, counting again in full.
    readonly property bool away: idle.isIdle
    // The notices on screen, as their notifications, which the held list
    // lists: every one live but those resting.
    readonly property var live: tracked
        .filter(notification => !(notification.id in Notices.resting))
    onHoldingChanged: {
        // Taken, the cursor starts on the card in front, the one Joel was
        // looking at; let go, what was done is let go for good, later,
        // since the stack can't change while it's being worked out.
        if (holding)
            held.take(shown[0] ?? null);
        else
            Qt.callLater(held.letGo);
        if (holding) {
            root.saidLetGo();
            Report.say("holds notices");
        } else if (taker.given) {
            // Said once niri has taken the keys back, so a test that
            // takes them again next waits for niri to give them anew
            // rather than hearing they're held while niri still moves
            // them, which lost its first key.
            root.letting = true;
        } else {
            Report.say("lets notices go");
        }
    }
    // Whether the stack has let the keys go and niri has yet to take them.
    property bool letting: false

    function saidLetGo() {
        if (!root.letting)
            return;
        root.letting = false;
        Report.say("lets notices go");
    }
    onAwayChanged: Report.say(away ? "away" : "back")

    IdleMonitor {
        id: idle
        timeout: Options.notices.away
        respectInhibitors: false
    }

    // What a notice said, as plain text: its summary, and its body without
    // the markup it may carry.
    function said(notification) {
        const body = notification.words.plain;
        return body === "" ? notification.summary : `${notification.summary}\n${body}`;
    }

    // A link in what a notice said opened in the program the desktop opens
    // it with, through niri, as every program is (the daemon's body.rs lets
    // only the web's and mail's through).
    function follow(href) {
        Niri.spawn(["xdg-open", href]);
    }

    // What a notice's sender does when it's chosen, or null.
    function opens(notification) {
        return notification.actions.find(action => action.identifier === "default") ?? null;
    }

    // Every card away at once, as a click on each would, so each stays
    // unread where Super+N finds it; held, the keys go back first.
    function putAway() {
        holding = false;
        root.dismiss(shown.slice());
    }

    function dismiss(ids) {
        for (const notification of tracked
            .filter(notification => ids.includes(notification.id)))
            notification.dismiss();
    }

    // What the stack means to draw, by rank, read from what it shows rather
    // than from what's visible, for the IPC to hold the gathered frames to.
    function meant() {
        const found = root.playing ? [track] : [];
        if (root.holding)
            return found.concat([held]);
        // The arrival line has the notices while it's up, and draws its own.
        if (root.lined)
            return found;
        const front = root.shown.slice(0, root.behind + 1);
        for (let i = 0; i < cards.count; i++) {
            const card = cards.itemAt(i);
            if (card && front.includes(carded.get(i).noticeId))
                found.push(card);
        }
        return found;
    }

    // One whose time is up leaves the screen but stays open, resting, so
    // its program still answers its actions from the launcher; one its
    // sender asked not to be kept closes. Past `rests` of them, the oldest
    // closes.
    function rest(notification) {
        if (notification.transient) {
            notification.expire();
            return;
        }
        Notices.rest(notification.id);
        Report.say(`rests ${notification.summary}`);
        // Counting this one, which the daemon has yet to say rests.
        const resting = tracked
            .filter(one => one.id in Notices.resting || one.id === notification.id)
            .sort((a, b) => root.arrivedAt(a.id) - root.arrivedAt(b.id));
        for (const oldest of resting.slice(0, Math.max(0, resting.length - root.rests)))
            oldest.expire();
    }

    // The notices in the launcher: For You's Desktop, those unread and then
    // History, what's read (launcher/Now.qml's leadsTo), the cursor on the
    // card in front while one is up, since notices are rows of the launcher
    // (PLAN-20261113-821NSZ).
    function history() {
        holding = false;
        const front = shown.length > 0 ? shown[0] : -1;
        if (launcher)
            launcher.intoOn("notices", row => front >= 0 && row.notice === front);
    }

    function since(then, now) {
        const minutes = Math.floor((now - then) / 60000);
        if (minutes < 1)
            return "now";
        if (minutes < 60)
            return `${minutes} min`;
        return Qt.formatDateTime(new Date(then), "HH:mm");
    }

    SystemClock {
        id: clock
        precision: SystemClock.Minutes
    }

    // Super+Shift+N puts every card away, as a click on each would, so each
    // stays unread where Super+N finds it.
    IpcHandler {
        target: "notices"

        function putAway(): void {
            root.putAway();
        }

        // Super+N: the notices in the launcher, on the card in front, where
        // the launcher's keys, acts and drawer do what the held stack's did
        // (PLAN-20261113-821NSZ). Pressed again while the launcher shows
        // them, it closes, as it let the held stack go.
        function take(): void {
            if (root.holding)
                root.holding = false;
            else if (root.launcher?.open && Notices.gathered)
                root.launcher.toggled();
            else
                root.history();
        }

        // The stack taking the keys as Vim's quickfix list, as Super+N did
        // before the notices were the launcher's; asked again, it lets them
        // go. It stays until the stack goes, which the launcher's notices
        // take over whole (the plan's fourth slice).
        function hold(): void {
            if (root.holding)
                root.holding = false;
            else if (root.shown.length > 0)
                root.holding = true;
        }

        // Every notice missed counted as seen, as the margin's Let Them Go
        // does.
        function seen(): void {
            Notices.seen();
        }

        // Where the front card is drawn, as a fraction of the screen across
        // and down, for a test to click on it, since no shot says where a
        // card without a coloured mark is; what it's signed, the glyph it
        // wears and that glyph's hue; whether a picture of its own stands
        // in its mark column; and where its links go.
        function front(): string {
            // While the arrival line is up, the front notice is the one it draws, where it
            // draws it, and what niri blurs is its own frame.
            const lined = line.where();
            if (lined !== null) {
                const from = line.notice.from;
                return JSON.stringify(Object.assign({
                    signed: from.name,
                    glyph: from.glyph,
                    hue: from.hue,
                    pictured: Notices.picture(line.notice) !== "",
                    links: line.notice.words.links.map(link => link.href)
                }, lined));
            }
            let card = null;
            for (let i = 0; i < cards.count && card === null; i++) {
                if (carded.get(i).noticeId === root.shown[0])
                    card = cards.itemAt(i) as Card;
            }
            const screen = window.screen;
            if (!card || !screen || !root.front)
                return "{}";
            const at = card.mapToItem(null, 0, 0);
            const left = screen.width - window.width + at.x;
            const pileAt = pile.mapToItem(null, 0, 0);
            const from = root.front.from;
            return JSON.stringify({
                signed: from.name,
                glyph: from.glyph,
                hue: from.hue,
                pictured: Notices.picture(root.front) !== "",
                links: root.front.words.links.map(link => link.href),
                left: left / screen.width,
                top: at.y / screen.height,
                right: (left + card.width) / screen.width,
                bottom: (at.y + card.height) / screen.height,
                // Where the stack ends, below its last edge.
                foot: (pileAt.y + pile.height) / screen.height,
                // What niri blurs and the pointer reaches, a frame each,
                // beside where each thing meant to be drawn stands in the
                // window, in the window's points, which those frames must
                // match. What's meant is read from the stack itself, not
                // from what gathers the frames, so a frame gathered by
                // mistake shows: what's playing, the held list, or the
                // shown cards from the front to the last edge.
                blurred: window.areas.map(area => [area.x, area.y, area.width, area.height]),
                drawn: meant().map(one => {
                    const where = one.mapToItem(null, 0, 0);
                    const x = Math.floor(where.x);
                    const y = Math.floor(where.y);
                    return [x, y, Math.ceil(where.x + one.width) - x,
                        Math.ceil(where.y + one.height) - y];
                })
            });
        }
    }

    // A keyboard's Do Not Disturb key holds notices until it's pressed
    // again, as a Mac's does, or lets them show if they're held.
    IpcHandler {
        target: "quiet"

        function toggle(): void {
            if (Quiet.on)
                Quiet.end();
            else
                Quiet.keep(0);
        }
    }

    // One arriving goes on the stack where Stack.js puts it, unless the
    // daemon held it, as one is while notices are (Quiet.qml) or when its
    // sender's level never shows it; then it rests from the start, never
    // shown but keeping its actions. One open as the shell starts was
    // there before, so it isn't said to arrive, and the stack puts it
    // behind those it orders (Stack.js's shown).
    // The cards, one a notice open, by id: a card stays as long as its
    // notice does, so its count isn't started over by another's change.
    ListModel {
        id: carded
    }

    function card() {
        const open = root.tracked.map(notification => notification.id);
        for (let i = carded.count - 1; i >= 0; i--) {
            if (!open.includes(carded.get(i).noticeId))
                carded.remove(i);
        }
        const had = [];
        for (let i = 0; i < carded.count; i++)
            had.push(carded.get(i).noticeId);
        // Each new one has a higher id than those before, so it goes last.
        for (const id of open.filter(id => !had.includes(id)))
            carded.append({ noticeId: id });
    }

    onTrackedChanged: card()

    Connections {
        target: Notices

        function onCame(notification, notice) {
            if (notice.held) {
                Report.say(`held ${notification.summary}`);
                return;
            }
            root.order = Stack.arrived(root.order, notification.id, notice.urgent);
        }
    }

    // A screen coming or going (a dock, the lid, a monitor) can leave the
    // notices' surface nowhere: with no screen left, Qt moves the window
    // onto a stand-in that can hold no surface, then onto the screen that
    // comes back by itself, so Quickshell never makes the surface again
    // (Launcher.qml says more). A critical notice never goes by itself, so
    // nothing would bring it back. So once the screens change, a surface
    // that's up is let go for a turn and made again.
    property bool remaking: false

    // One said again in place that tells something new comes back to the
    // front, as one arriving would, unless notices are held and it isn't
    // urgent, when it keeps resting. Arrivals asks for it through Notices,
    // and the daemon says it was raised.
    Connections {
        target: Notices

        function onRaisedChanged() {
            const id = Notices.raised?.id;
            const notification = root.tracked.find(one => one.id === id);
            if (!notification
                || Quiet.on && notification.urgency !== NotificationUrgency.Critical)
                return;
            Notices.wake(id);
            root.order = Stack.forward(root.order.includes(id) ? root.order
                : root.order.concat([id]), id);
            Report.say(`raises ${notification.summary}`);
        }
    }

    // A click on the front notice, the card's or the arrival line's: it opens what it's
    // about, or puts it away when it's about nothing to open.
    function choose(notification) {
        const opens = root.opens(notification);
        if (opens) {
            Notices.mark(notification.id, true);
            Notices.open(opens, notification.appName);
        } else {
            notification.dismiss();
        }
    }

    // What a notice's signing line says on the right: how many more wait behind it and
    // when it came, or, for one saying how it stands, that and its time, "sent · 12:40"
    // (VKCF3Z, 2026-11-12).
    function whenOf(notification, more) {
        const now = clock.date.getTime();
        return (more > 0 ? `${more} more \u00b7 ` : "")
            + (notification.said !== ""
                ? `${notification.said} \u00b7 ${When.clock(notification.at)}`
                : root.since(root.arrivedAt(notification.id) || now, now));
    }

    // The arrival line, where the front notice shows while the stack draws none of them
    // (NoticeLine.qml).
    NoticeLine {
        id: line

        launcher: root.launcher
        shown: root.shown
        talking: root.talking
        holding: root.holding
        whenOf: (notification, more) => root.whenOf(notification, more)
        onClicked: notification => root.choose(notification)
        onLinked: href => root.follow(href)
    }

    Connections {
        target: Quickshell

        function onScreensChanged() {
            root.remaking = true;
            Qt.callLater(() => {
                root.remaking = false;
            });
        }
    }

    PanelWindow {
        id: window

        Counted { name: "notices" }

        // Where the pointer reaches and what niri blurs: the notices alone,
        // not the gaps between them.
        property var areas: []
        readonly property Region notices: Region {
            regions: window.areas
        }

        // What's drawn, in the order its frames are gathered: what's
        // playing, the held list, and the cards drawn, as they're tracked.
        function drawn() {
            const found = root.playing ? [track] : [];
            if (root.holding)
                found.push(held);
            for (let i = 0; i < cards.count; i++) {
                const card = cards.itemAt(i) as Card;
                if (card?.visible)
                    found.push(card);
            }
            return found;
        }

        // Gathered as what's drawn changes, never as it moves, since each
        // frame follows its surface; a card resting off the screen, or
        // past the last edge, is tracked but not drawn, so it's left out.
        function gather() {
            areas = drawn().map(one => one.framed);
        }

        visible: root.drawing && Niri.tilesReady && !Notices.gathered && !root.remaking
        anchors.top: true
        anchors.right: true
        implicitWidth: noticeColumn.x + noticeColumn.width + root.inside.right
        implicitHeight: noticeColumn.height + noticeColumn.y + Theme.floating.reach
        exclusionMode: ExclusionMode.Ignore
        color: "transparent"
        mask: notices
        BackgroundEffect.blurRegion: notices
        WlrLayershell.layer: WlrLayer.Overlay
        WlrLayershell.namespace: "notifications"
        // Asked for as the launcher asks (Niri.asksForKeys), so held over it
        // the notices take the keys and give them back as they let go; a
        // card coming asks for none.
        WlrLayershell.keyboardFocus: root.holding ? Niri.asksForKeys : WlrKeyboardFocus.None

        // Keys reach the held list while the stack holds them, which niri
        // gives a frame after it's asked and takes back a frame after
        // they're let go, so each is said once niri has.
        Item {
            id: taker

            readonly property bool given: Window.active
            readonly property bool taken: activeFocus && given

            focus: root.holding
            Keys.onPressed: event => held.pressed(event)
            onTakenChanged: if (taken) Report.say("keys notices")
            onGivenChanged: if (!given) root.saidLetGo()
        }

        // A card coming or going changes the stack's height while the
        // surface stays up.
        Reported {
            id: reported
            name: "notifications"
        }

        // What's playing changes its card's words, not the stack's height.
        Connections {
            target: Music

            function onCardChanged() {
                reported.changed();
            }
        }

        Column {
            id: noticeColumn
            // Held, the list stands its gutter wider than the column, out
            // to the left, its right edge under the corner as every card's.
            x: Theme.floating.reach + (held.visible ? held.gutter : 0)
            y: root.inside.top
            onHeightChanged: reported.changed()

            Behavior on y {
                enabled: root.glides
                NumberAnimation {
                    duration: Theme.motion.short
                    easing.type: Easing.BezierSpline
                    easing.bezierCurve: Theme.motion.curve
                }
            }
            width: root.inside.width
            // Cards are floating frames that belong together.
            spacing: Theme.space.rung.frames

            Card {
                id: track

                width: noticeColumn.width
                visible: root.playing
                standsAt: Qt.point(noticeColumn.x + track.x, noticeColumn.y + track.y)
                onVisibleChanged: Qt.callLater(window.gather)

                // A notice's three groups and room, as its neighbours'
                // are: the track, who plays it, and the player and where
                // in the track. Its cover stands in the mark column, and
                // the player's mark when there's no cover, so its lines
                // start on the text column a picture leaves (VKCF3Z,
                // 2026-11-06), with the player's name and where in the
                // track on the signing line.
                readonly property var columns: Columns.across(said.width, 0, cover.visible
                    ? Columns.ofPicture(cover.width) : Columns.ofMark(Theme.level.name.pitch),
                    Theme.space.rung)

                Item {
                    id: said
                    width: parent.width
                    implicitHeight: playedBy.y + playedBy.height

                    Item {
                        id: song
                        // The message's last baseline, or the cover's
                        // foot when it reaches lower, which the signature
                        // clears.
                        readonly property real ends: Math.max(y + cover.y + cover.height,
                            y + lines.lastBaseline)

                        y: -title.baselineOffset - capital.tightBoundingRect.y
                        width: parent.width
                        height: Math.max(cover.y + cover.height, lines.height)

                        // Two lines tall from the title's capital.
                        Cover {
                            id: cover
                            x: track.columns.mark
                            y: title.baselineOffset + capital.tightBoundingRect.y
                            source: root.song.art
                        }

                        // The title and who plays it, lines of one saying,
                        // each a pitch below the last, the cover a run
                        // apart from them.
                        Item {
                            id: lines
                            readonly property real lastBaseline: artist.visible
                                ? artist.y + artist.baselineOffset
                                : title.baselineOffset
                                    + (title.lineCount - 1) * Theme.level.name.pitch

                            x: track.columns.text
                            width: track.columns.end - x
                            height: artist.visible ? artist.y + artist.height : title.height

                            Text {
                                id: title
                                width: parent.width
                                text: root.song.title
                                color: Theme.strong
                                wrapMode: Text.Wrap
                                maximumLineCount: 2
                                elide: Text.ElideRight
                                lineHeight: Theme.level.name.pitch
                                lineHeightMode: Text.FixedHeight
                                font.family: Theme.level.name.family
                                font.pointSize: Theme.level.name.pointSize
                                font.weight: Theme.level.name.weight
                            }

                            Text {
                                id: artist
                                y: title.baselineOffset
                                    + (title.lineCount - 1) * Theme.level.name.pitch
                                    + Theme.level.words.pitch - baselineOffset
                                width: parent.width
                                text: root.song.artist
                                visible: text !== ""
                                color: Theme.text
                                elide: Text.ElideRight
                                lineHeight: Theme.level.words.pitch
                                lineHeightMode: Text.FixedHeight
                                font.family: Theme.level.words.family
                                font.pointSize: Theme.level.words.pointSize
                                font.weight: Theme.level.words.weight
                                font.variableAxes: Theme.level.words.variableAxes
                            }
                        }
                    }

                    // The player and where in the track, as a notice's
                    // signature.
                    Mark {
                        x: track.columns.mark
                        line: song.y + title.baselineOffset
                        visible: !cover.visible
                        text: root.song.mark
                        color: Theme.program
                    }

                    Signature {
                        id: playedBy
                        x: track.columns.text
                        y: playedBy.onPixel(song.ends + root.signed)
                        width: track.columns.end - x
                        name: root.song.from
                        side: root.song.where
                    }
                }

                whole: MouseArea {
                    anchors.fill: parent
                    onClicked: Music.dismiss()
                }
            }

            // Held, the stack is the quickfix list, in its place.
            HeldNotices {
                id: held

                stack: root
                live: root.live
                keysTo: taker
                x: -gutter
                width: noticeColumn.width + gutter
                visible: root.holding
                screenHeight: window.screen ? window.screen.height : 768
                // It moves as what's playing comes and goes above it.
                standsAt: Qt.point(noticeColumn.x + held.x, noticeColumn.y + held.y)
                onVisibleChanged: Qt.callLater(window.gather)
            }

            // The notices, one stack: the front one whole, and behind it
            // the next few as edges, each a peek lower and a peek narrower
            // on both sides, with nothing drawn in them. Each edge starts
            // on the bottom hairline of the card before it, so only what
            // shows beneath is drawn and no edge's frame shows through the
            // cloudy card in front of it. It reaches the lower of the front
            // card and the last edge as they glide.
            Item {
                id: pile

                width: noticeColumn.width
                height: root.shown.length === 0 || root.holding || root.lined ? 0
                    : Math.max(root.frontHeight,
                        root.edgeHeight + Math.min(root.shown.length - 1, root.behind) * root.peek)
                visible: root.shown.length > 0 && !root.holding && !root.lined

            Repeater {
                id: cards
                model: carded
                onItemAdded: Qt.callLater(window.gather)
                onItemRemoved: Qt.callLater(window.gather)

                delegate: Card {
                    id: card

                    required property int noticeId
                    // Its notice, or none for the moment between its
                    // notice going and the card with it, so nothing
                    // reads a notice no longer there.
                    readonly property var modelData: Notices.live[noticeId] ?? LiveNotice.none
                    readonly property var from: modelData.from
                    // Where it stands in the stack, 0 at the front.
                    readonly property int rank: root.shown.indexOf(modelData.id)
                    readonly property bool front: rank === 0

                    visible: rank >= 0 && rank <= root.behind
                    z: -rank
                    x: Math.max(rank, 0) * root.peek
                    y: front ? 0 : (Math.max(rank, 1) - 1) * root.peek + root.edgeHeight
                        - Theme.border
                    width: noticeColumn.width - 2 * x
                    height: front ? implicitHeight : root.peek + Theme.border
                    flatTop: !front
                    urgent: modelData.urgency === NotificationUrgency.Critical

                    onFrontChanged: if (front) root.frontHeight = implicitHeight
                    onImplicitHeightChanged: if (front) root.frontHeight = implicitHeight
                    // It moves with the stack as it turns, and with what's
                    // above it as that comes and goes.
                    standsAt: Qt.point(noticeColumn.x + pile.x + card.x,
                        noticeColumn.y + pile.y + card.y)
                    // Drawn or not as it comes to the stack, goes past its
                    // last edge, rests or the stack is held.
                    onVisibleChanged: Qt.callLater(window.gather)

                    Component.onCompleted: {
                        if (!(card.modelData.id in Notices.resting))
                            Report.say(`shows card ${card.modelData.summary}`);
                    }

                    // A click anywhere on the front card opens what it's
                    // about, or puts it away when it's about nothing to
                    // open, and one on an edge behind brings it to the
                    // front, edge to edge inside its frame. What else it
                    // offers the keys reach (Joel wants keys first).
                    whole: MouseArea {
                        anchors.fill: parent
                        onClicked: {
                            if (!card.front)
                                root.order = Stack.forward(root.shown, card.modelData.id);
                            else
                                root.choose(card.modelData);
                        }
                    }

                    // The notice as a row of its card (NoticeRow.qml), with
                    // no gutter, since one notice isn't a list; on the
                    // right of its signing line how many wait behind it
                    // and when it came, or, for one saying how it stands,
                    // that and its time, "sent · 12:40" (VKCF3Z,
                    // 2026-11-12). The frame stands as far from the
                    // ink on every side (Joel, 2026-11-03: "feeling quite
                    // cluttered").
                    NoticeRow {
                        readonly property int more: root.shown.length - 1

                        visible: card.front
                        width: parent.width
                        mark: card.from.glyph
                        hue: Theme[card.from.hue]
                        picture: Notices.picture(card.modelData)
                        quiet: card.modelData.urgency === NotificationUrgency.Low
                        title: card.modelData.summary
                        words: card.modelData.words.styled
                        onLinked: href => root.follow(href)
                        from: card.from.name
                        when: root.whenOf(card.modelData, more)
                    }

                    // Five seconds when the sender doesn't say, and ten for
                    // one offering something to do. Quickshell's documents
                    // call its timeout seconds, but it passes on what the
                    // sender said, which is milliseconds. Only the front
                    // one counts, so one behind waits its turn and is seen
                    // whole; and it waits while the keys hold the notices,
                    // here or in the launcher, so one being acted on can't
                    // go from under them, or while Joel's away, starting
                    // again in full after.
                    Timer {
                        running: !card.urgent && !Notices.gathered && card.front && !root.holding
                            && !root.away && !root.talking
                        onRunningChanged: Report.say(
                            `${running ? "counts" : "waits"} card ${card.modelData.summary}`)
                        interval: card.modelData.expireTimeout > 0
                            ? card.modelData.expireTimeout
                            : card.modelData.actions.length > 0 ? 10000 : 5000
                        onTriggered: root.rest(card.modelData)
                    }
                }
            }
            }
        }
    }
}
