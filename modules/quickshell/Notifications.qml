// The notification server: the notices on screen are one stack in the top
// right corner, the oldest not yet dealt with in front and up to three
// newer behind it as edges, a critical one going to the front. A click on
// the front one opens what it's about, or puts it away when it's about
// nothing; one on an edge brings that one forward. The front one leaves by
// itself when its time is up, unless it's critical. While notices are held
// (Quiet.qml), only a critical one shows.
//
// Super+N gives the stack the keys while there are notices, and opens the
// history in the launcher when there are none or it's pressed again. The
// keys are the launcher's (`pressed` says what each does): Return does the
// front one, and the right arrow steps into its verbs, the same ones its
// past self has in the history. While the stack holds them, the front
// card's footer shows them where it said when the notice came.
//
// The notices sit a step inside the window under the top right corner, so
// a notice lies on one window rather than across a border, or a step inside
// where a window's corner would be when there's none (`inside` says how),
// clear of the margin's marks, which stand at the top centre in the gap
// above the windows.
//
// Each is kept in the session's history as it arrives (Notices.qml), for
// the launcher to find later, where the ones still here can be answered
// or forgotten, and while it's open the corner leaves them to it. A notice
// offering something to do (Reply, Play it) lists it above its footer with
// the chord that takes the stack beside it, and its verbs lead with it.
//
// It's the notice kind (docs/design.md, Shape and Kinds): the edge frame,
// the accent's while the stack holds the keys, or the alert's when
// critical; who or what
// first, in words, then the message; and a footer in muted monospace saying
// where it came from, its glyph a step apart from its name, and when.
//
// What's playing (Music.qml) is a notice too, for a moment when a track
// changes or a media key is pressed, at the top of the same column so the
// two never cover each other: the cover beside the title and the artist,
// and the player and where in the track in the footer. It's no message, so
// it isn't kept; a click lets it go.
pragma ComponentBehavior: Bound
import Quickshell
import Quickshell.Io
import Quickshell.Services.Notifications
import Quickshell.Wayland
import QtQuick
import QtQuick.Effects
import "Levels.js" as Levels
import "Stack.js" as Stack

Scope {
    id: root

    // Wide enough for a short message on one or two lines.
    readonly property int width: 380
    // The body step's line, to a whole pixel on the space grid.
    readonly property int line: Theme.space.xl - Theme.space.xs / 2

    // Whether what's playing is called for. The card's own visibility can't
    // say, since an item in a hidden window counts as hidden.
    readonly property bool playing: Music.shown && Music.card !== null
    // What the card says, or nothing while there's no player to say it of.
    readonly property var song: Music.card ?? {
        title: "", artist: "", art: "", from: "", mark: "", where: ""
    }

    // When each notification arrived, by its id, for the footer's time.
    property var arrived: ({})
    // The one whose words were just copied, whose footer says so in its
    // time's place a moment, since the copy gives the keys back and
    // leaves nothing else to show it happened; -1 for none.
    property int copied: -1

    Timer {
        id: copiedFor
        interval: 2000
        onTriggered: root.copied = -1
    }

    // Where the notices sit: a step inside the window nearest the top
    // right corner of all those wholly on screen, the rightmost and then
    // the highest, their right edge and top each a step within its own, so
    // a notice lies on one window and never across a border, and no wider
    // than fits in it. With no window wholly on screen, or the nearest too
    // narrow to hold a notice, they sit in the corner, a step inside where
    // a window's would be. niri says where a window will come to rest
    // (modules/niri/tiled-positions.patch), so they move once a scroll
    // settles, not with every frame.
    property int settled: 0
    readonly property var inside: {
        settled;
        Niri.revision;
        const step = Theme.space.s;
        const corner = { right: Theme.space.m + step, top: Theme.space.m + step, width: root.width };
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
        const room = under.width - 2 * step;
        if (room < 2 * root.width / 3)
            return corner;
        return {
            right: Math.round(screen.width - (under.left + under.width) + step),
            top: Math.round(Math.max(under.top, 0) + step),
            width: Math.min(root.width, Math.floor(room))
        };
    }
    Connections {
        target: Niri

        function onLayoutsChanged() {
            root.settled++;
        }
    }
    onInsideChanged: Report.say(`places notices ${inside.right} ${inside.top} ${inside.width}`)

    // The launcher, for the history the stack hands on to.
    property var launcher: null

    // The notices on screen, as one stack: their ids, the front first, in
    // the order Stack.js keeps. Those resting off it (Notices.qml) are
    // still open, but not here.
    property var order: []
    readonly property var shown: Stack.shown(order,
        server.trackedNotifications.values.map(notification => notification.id)
            .filter(id => !(id in Notices.resting)))
    // How many may rest at once before the oldest is closed, so no program
    // is asked to hold its notices for ever.
    readonly property int rests: 30
    readonly property var front: {
        const id = shown[0];
        return server.trackedNotifications.values.find(notification => notification.id === id)
            ?? null;
    }
    // How many behind the front show their edge, a step lower and a step
    // narrower each, and how far.
    readonly property int behind: 3
    readonly property int peek: Theme.space.s
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
    // Said while the stack holds the keys, so the desktop test sees the
    // card hold still as the keys move into, through and out of its acts.
    // It's said once its bindings settle, since one step changes several
    // and the height between them is never drawn, and only when it differs
    // from what was said last.
    property int stood: 0
    onFrontHeightChanged: Qt.callLater(stands)
    function stands() {
        const standing = Math.round(frontHeight);
        if (!holding || standing === stood)
            return;
        stood = standing;
        Report.say(`notice stands ${standing}`);
    }

    // Whether the stack holds the keys, which Super+N gives it while there
    // are notices; whether they've stepped into the front one's verbs, as
    // the right arrow does into a launcher row's; and which verb is chosen.
    property bool holding: false
    // Whether Joel's away: nothing typed or moved for a while, whatever
    // keeps the screen awake, since a call or Keep Awake does that while
    // he's gone too. A card waits while he's away, so one that comes while
    // he's gone is there when he's back, counting again in full.
    readonly property bool away: idle.isIdle
    property bool stepped: false
    property int chosen: 0
    onShownChanged: {
        stepped = false;
        if (shown.length === 0)
            holding = false;
    }
    onHoldingChanged: {
        stepped = false;
        stood = 0;
        Qt.callLater(stands);
        Report.say(holding ? "holds notices" : "lets notices go");
    }
    onSteppedChanged: if (!stepped) Report.say("notice back")
    onAwayChanged: Report.say(away ? "away" : "back")

    IdleMonitor {
        id: idle
        timeout: Options.notices.away
        respectInhibitors: false
    }
    onChosenChanged: if (stepped) Report.say(`notice verb ${verbs[chosen]?.name ?? ""}`)

    // What can be done with the front one, the same as its past self in
    // the launcher (launcher/PastNotices.qml): opening what it's about,
    // which Return does too, what it offers, copying what it said,
    // forgetting it, forgetting them all, and the rest of the notices;
    // each named as the vocabulary says it (launcher/words.json), with
    // the key that does it straight away, where there is one. Forgetting
    // one changes the stack
    // rather than taking Joel anywhere, so it stays: the keys stay with
    // the stack, which shows the next. The rest take him out, so they give
    // the keys back (Joel, 2026-10-26: Return closes only when the act
    // takes him out of where he is).
    readonly property var verbs: {
        const notification = front;
        if (!notification)
            return [];
        const done = run => () => {
            Notices.mark(notification.id, true);
            run();
        };
        const opening = opens(notification);
        const open = opening === null ? [] : [{
            glyph: Words.glyph("verb:notice:open"),
            name: Words.name("verb:notice:open"),
            chord: [{ text: "\u21b5" }],
            run: () => {
                Notices.mark(notification.id, true);
                Notices.open(opening, notification.appName);
            }
        }];
        return open.concat(offers(notification).map(offer => ({
            glyph: /reply/i.test(offer.text) ? Theme.glyph.reply : Theme.glyph.offer,
            name: offer.text,
            chord: [],
            run: done(() => offer.invoke())
        }))).concat([{
            glyph: Words.glyph("verb:notice:copy"),
            name: Words.name("verb:notice:copy"),
            chord: [],
            run: done(() => {
                Quickshell.clipboardText = root.said(notification);
                root.copied = notification.id;
                copiedFor.restart();
            })
        }, {
            glyph: Words.glyph("verb:notice:forget"),
            name: Words.name("verb:notice:forget"),
            // Keys drawn as the desktop draws them (home.nix, keyFaces),
            // Backspace by its name, since it has no glyph of its own.
            chord: [{ text: "Backspace" }],
            stays: true,
            run: () => notification.dismiss()
        }, {
            glyph: Words.glyph("verb:notices:forget"),
            name: Words.name("verb:notices:forget"),
            chord: [{ text: "\u21e7" }, { text: "Backspace" }],
            stays: true,
            run: () => root.putAway()
        }, {
            glyph: Words.glyph("verb:notices:all"),
            name: Words.name("verb:notices:all"),
            // The chord that opens them, pressed again while the stack
            // holds the keys; Tab only completes (Joel, 2026-10-26).
            chord: root.reaching,
            run: () => root.history()
        }]);
    }

    // What a notice said, as plain text: its summary, and its body without
    // the markup it may carry.
    function said(notification) {
        const body = notification.body.replace(/<[^>]*>/g, "").replace(/&lt;/g, "<")
            .replace(/&gt;/g, ">").replace(/&amp;/g, "&");
        return body === "" ? notification.summary : `${notification.summary}\n${body}`;
    }

    // What a notice's sender does when it's chosen, or null, and what else
    // it offers, which its card lists.
    function opens(notification) {
        return notification.actions.find(action => action.identifier === "default") ?? null;
    }
    function offers(notification) {
        return notification.actions.filter(action => action.identifier !== "default");
    }

    // The front one done with: the verb chosen, or what it's about, or let
    // go when it's about nothing to open. What takes Joel out gives the
    // keys back, so what he types next reaches the window that came
    // forward; what only changes the stack keeps them, unless he says to
    // be done (Ctrl+Return).
    function run(done) {
        const notification = front;
        if (!notification)
            return;
        if (stepped) {
            const verb = verbs[chosen] ?? null;
            stepped = false;
            if (verb)
                verb.run();
            if (done || !(verb?.stays ?? false))
                holding = false;
            return;
        }
        Notices.mark(notification.id, true);
        const offer = opens(notification);
        if (offer) {
            Notices.open(offer, notification.appName);
            holding = false;
        } else {
            notification.dismiss();
            if (done)
                holding = false;
        }
    }

    // Turns the stack, so the next comes to the front, or the one before.
    function turn(by) {
        // A stack of one or none is left alone, so nothing it binds to moves.
        if (shown.length > 1)
            order = Stack.turned(shown, by);
    }

    // The keys while the stack holds them, as the launcher's are: Return
    // does the front one, the right arrow steps into its verbs, where Up
    // and Down choose one and the left arrow or Escape steps back, and
    // otherwise Up and Down move through the stack. Delete or Backspace
    // lets it go and with Shift all of them, the chord that opened the
    // stack hands on to the history (niri's, to `take` below), and Escape
    // gives the keys back. Tab does nothing, being only for completing.
    function pressed(event) {
        const all = (event.modifiers & Qt.ShiftModifier) !== 0;
        switch (event.key) {
        case Qt.Key_Return:
        case Qt.Key_Enter:
            run((event.modifiers & Qt.ControlModifier) !== 0);
            break;
        case Qt.Key_Right:
            if (!stepped && verbs.length > 0) {
                chosen = 0;
                stepped = true;
                Report.say(`notice verb ${verbs[0].name}`);
            }
            break;
        case Qt.Key_Left:
            stepped = false;
            break;
        case Qt.Key_Down:
            if (stepped)
                chosen = Math.min(chosen + 1, verbs.length - 1);
            else
                turn(1);
            break;
        case Qt.Key_Up:
            if (stepped)
                chosen = Math.max(chosen - 1, 0);
            else
                turn(-1);
            break;
        case Qt.Key_Delete:
        case Qt.Key_Backspace:
            if (all)
                putAway();
            else if (front)
                front.dismiss();
            break;
        case Qt.Key_Escape:
            if (stepped)
                stepped = false;
            else
                holding = false;
            break;
        default:
            return;
        }
        event.accepted = true;
    }

    function putAway() {
        for (const notification of server.trackedNotifications.values
            .filter(notification => root.shown.includes(notification.id)))
            notification.dismiss();
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
        const resting = server.trackedNotifications.values
            .filter(one => one.id in Notices.resting)
            .sort((a, b) => (root.arrived[a.id] ?? 0) - (root.arrived[b.id] ?? 0));
        for (const oldest of resting.slice(0, Math.max(0, resting.length - root.rests)))
            oldest.expire();
    }

    // The session's notices in the launcher, where past ones are too.
    function history() {
        holding = false;
        if (launcher)
            launcher.into("notices");
    }

    // The chord that opens the notices, key by key and as the launcher
    // draws chords.
    readonly property var reaching: Shortcuts.actions
        .find(action => action.opens === "notices")?.chord ?? []
    readonly property string reach: reaching.map(key => key.text).join(" + ")

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

        // Super+N: the stack takes the keys while there are notices, and
        // otherwise, or asked again, the history opens.
        function take(): void {
            if (root.holding || root.shown.length === 0)
                root.history();
            else
                root.holding = true;
        }

        // Every notice missed counted as seen, as the margin's Let Them Go
        // does.
        function seen(): void {
            Notices.seen();
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

    NotificationServer {
        id: server
        bodySupported: true
        actionsSupported: true
        keepOnReload: true
        onNotification: notification => {
            root.arrived[notification.id] = Date.now();
            // How far it reaches is its sender's level at most (Levels.js):
            // one never shown is held, as one is while notices are.
            const level = Levels.reached(Options.notices.senders[notification.appName
                .toLowerCase()], notification.urgency === NotificationUrgency.Critical);
            const reach = Levels.reach(level);
            const urgent = reach.urgent;
            const held = !reach.shown || (Quiet.on && !urgent);
            // One its sender marks transient asks not to be kept, as niri's
            // screenshot notice does.
            if (!notification.transient)
                Notices.arrived(notification, urgent, held, reach.counted);
            if (held)
                Report.say(`held ${notification.summary}`);
            const id = notification.id;
            if (!held)
                root.order = Stack.arrived(root.order, id, urgent);
            notification.closed.connect(reason => Notices.left(id,
                reason === NotificationCloseReason.Expired ? "expired"
                : reason === NotificationCloseReason.CloseRequested ? "withdrawn"
                : "dismissed"));
            // One held rests from the start, so it never shows but keeps
            // its actions, unless its sender asked not to keep it.
            notification.tracked = !held || !notification.transient;
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

        // Where the pointer reaches and what niri blurs: the notices alone,
        // not the gaps between them.
        property var areas: []
        readonly property Region notices: Region {
            regions: window.areas
        }

        function gather() {
            const found = root.playing ? [track.area] : [];
            for (let i = 0; i < cards.count; i++) {
                const card = cards.itemAt(i) as Floating;
                if (card)
                    found.push(card.area);
            }
            areas = found;
        }

        visible: (root.shown.length > 0 || root.playing)
            && !Notices.gathered && !root.remaking
        anchors.top: true
        anchors.right: true
        implicitWidth: stack.width + root.inside.right + Theme.floating.reach
        implicitHeight: stack.height + stack.y + Theme.floating.reach
        exclusionMode: ExclusionMode.Ignore
        color: "transparent"
        mask: notices
        BackgroundEffect.blurRegion: notices
        WlrLayershell.layer: WlrLayer.Overlay
        WlrLayershell.namespace: "notifications"
        WlrLayershell.keyboardFocus: root.holding ? WlrKeyboardFocus.Exclusive : WlrKeyboardFocus.None

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
            id: stack
            x: Theme.floating.reach
            y: Math.max(root.inside.top, Theme.space.m + Theme.space.s)
            onHeightChanged: reported.changed()
            // Where the pointer reaches and what niri blurs follow a card
            // only as it moves within the column, so they're gathered again
            // as the column moves, or they'd stay behind as a shadow where
            // it was.
            onYChanged: window.gather()

            Behavior on y {
                enabled: root.glides
                NumberAnimation {
                    duration: Theme.motion.short
                    easing.type: Easing.BezierSpline
                    easing.bezierCurve: Theme.motion.curve
                }
            }
            width: root.inside.width
            spacing: Theme.space.m

            Floating {
                id: track

                width: stack.width
                visible: root.playing
                onVisibleChanged: Qt.callLater(window.gather)
                onHeightChanged: Qt.callLater(window.gather)

                Column {
                    id: said
                    x: Theme.space.l
                    width: parent.width - 2 * x
                    topPadding: Theme.space.m - Theme.border
                    bottomPadding: topPadding + 2

                    Item {
                        width: parent.width
                        height: Math.max(cover.height, lines.implicitHeight)

                        // Two lines tall, its corners the theme's inner
                        // radius, and the raised colour until it arrives.
                        Rectangle {
                            id: cover
                            width: 2 * root.line
                            height: width
                            radius: Theme.inner
                            color: Theme.raised
                            visible: root.song.art !== ""

                            Image {
                                anchors.fill: parent
                                source: root.song.art
                                asynchronous: true
                                fillMode: Image.PreserveAspectCrop
                                sourceSize.width: 2 * width
                                sourceSize.height: 2 * height
                                layer.enabled: true
                                layer.effect: MultiEffect {
                                    maskEnabled: true
                                    maskSource: corners
                                }
                            }

                            Rectangle {
                                id: corners
                                anchors.fill: parent
                                radius: Theme.inner
                                visible: false
                                layer.enabled: true
                            }
                        }

                        Column {
                            id: lines
                            anchors.left: cover.visible ? cover.right : parent.left
                            anchors.leftMargin: cover.visible ? Theme.space.m : 0
                            anchors.right: parent.right

                            Text {
                                width: parent.width
                                text: root.song.title
                                color: Theme.strong
                                wrapMode: Text.Wrap
                                maximumLineCount: 2
                                elide: Text.ElideRight
                                lineHeight: root.line
                                lineHeightMode: Text.FixedHeight
                                font.family: Theme.sans.family
                                font.pointSize: Theme.sans.body
                                font.weight: Font.Medium
                            }

                            Text {
                                width: parent.width
                                text: root.song.artist
                                visible: text !== ""
                                color: Theme.text
                                elide: Text.ElideRight
                                lineHeight: root.line
                                lineHeightMode: Text.FixedHeight
                                font.family: Theme.sans.family
                                font.pointSize: Theme.sans.body
                            }
                        }
                    }

                    // The player and where in the track, a step below.
                    Item {
                        width: parent.width
                        height: root.line + Theme.space.xs

                        Text {
                            id: player
                            anchors.bottom: parent.bottom
                            height: root.line
                            verticalAlignment: Text.AlignVCenter
                            text: root.song.mark
                            color: Theme.program
                            font.family: Theme.mono.family
                            font.pointSize: Theme.mono.body
                        }

                        Text {
                            anchors.left: player.right
                            anchors.leftMargin: Theme.space.s
                            anchors.right: place.left
                            anchors.rightMargin: Theme.space.l
                            anchors.bottom: parent.bottom
                            height: root.line
                            verticalAlignment: Text.AlignVCenter
                            text: root.song.from
                            elide: Text.ElideRight
                            color: Theme.muted
                            font.family: Theme.mono.family
                            font.pointSize: Theme.mono.body
                        }

                        Text {
                            id: place
                            anchors.right: parent.right
                            anchors.bottom: parent.bottom
                            height: root.line
                            verticalAlignment: Text.AlignVCenter
                            text: root.song.where
                            color: Theme.muted
                            font.family: Theme.mono.family
                            font.pointSize: Theme.mono.body
                            font.features: { "tnum": 1 }
                        }
                    }
                }

                MouseArea {
                    width: parent.width
                    height: said.height
                    onClicked: Music.shown = false
                }
            }

            // The notices, one stack: the front one whole, and behind it
            // the next few as edges, each a step lower and a step narrower,
            // at the front one's height with nothing drawn in them, so its
            // cloudy surface shows only their frames beneath. It reaches the
            // lower of the front card and the last edge as they glide.
            Item {
                id: pile

                width: stack.width
                height: root.shown.length === 0 ? 0 : Math.max(root.frontHeight,
                    root.edgeHeight + Math.min(root.shown.length - 1, root.behind) * root.peek)
                visible: root.shown.length > 0

                // Keys reach it while the stack holds them, which niri
                // gives a frame after it's asked.
                Item {
                    focus: root.holding
                    Keys.onPressed: event => root.pressed(event)
                    onActiveFocusChanged: if (activeFocus) Report.say("keys notices")
                }

            Repeater {
                id: cards
                model: server.trackedNotifications
                onItemAdded: Qt.callLater(window.gather)
                onItemRemoved: Qt.callLater(window.gather)

                delegate: Floating {
                    id: card

                    required property Notification modelData
                    readonly property var from: Notices.sender(modelData.appName)
                    // Where it stands in the stack, 0 at the front.
                    readonly property int rank: root.shown.indexOf(modelData.id)
                    readonly property bool front: rank === 0

                    visible: rank >= 0 && rank <= root.behind
                    z: -rank
                    x: Math.max(rank, 0) * Theme.space.s
                    y: Math.max(rank, 0) * root.peek
                    width: stack.width - 2 * x
                    height: front ? implicitHeight : root.edgeHeight
                    urgent: modelData.urgency === NotificationUrgency.Critical
                    keys: front && root.holding

                    onFrontChanged: if (front) root.frontHeight = implicitHeight
                    onImplicitHeightChanged: if (front) root.frontHeight = implicitHeight
                    // Its frame moves as the stack turns, which its window's
                    // regions follow only when told.
                    onRankChanged: Qt.callLater(window.gather)
                    onHeightChanged: Qt.callLater(window.gather)

                    Component.onCompleted: {
                        if (!(card.modelData.id in Notices.resting))
                            Report.say(`shows card ${card.modelData.summary}`);
                    }

                    // What its sender does when the notice itself is
                    // chosen, such as opening the conversation it's about,
                    // or null for one that offers nothing.
                    readonly property var opens: root.opens(card.modelData)

                    // A click anywhere on the front card opens what it's
                    // about, or puts it away when it's about nothing to
                    // open, and one on an edge behind brings it to the
                    // front. It lies beneath the words, so an action's own
                    // area, drawn later, takes the click on the action.
                    MouseArea {
                        width: parent.width
                        height: card.front ? words.height : parent.height
                        onClicked: {
                            if (!card.front) {
                                root.order = Stack.forward(root.shown, card.modelData.id);
                            } else if (card.opens) {
                                Notices.mark(card.modelData.id, true);
                                Notices.open(card.opens, card.modelData.appName);
                            } else {
                                card.modelData.dismiss();
                            }
                        }
                    }

                    Column {
                        id: words
                        visible: card.front
                        x: Theme.space.l
                        width: parent.width - 2 * x
                        // The letters sit a step from the frame on every
                        // side, measured to the ink: the footer's glyph
                        // reaches lower than a line's letters, so the
                        // bottom keeps a little more.
                        topPadding: Theme.space.m - Theme.border
                        bottomPadding: topPadding + 2

                        Text {
                            width: parent.width
                            text: card.modelData.summary
                            // Plain, as the spec has a summary: only the
                            // body carries markup.
                            textFormat: Text.PlainText
                            color: Theme.strong
                            wrapMode: Text.Wrap
                            lineHeight: root.line
                            lineHeightMode: Text.FixedHeight
                            font.family: Theme.sans.family
                            font.pointSize: Theme.sans.body
                            font.weight: Font.Medium
                        }

                        Text {
                            width: parent.width
                            text: card.modelData.body
                            visible: text !== ""
                            color: Theme.text
                            wrapMode: Text.Wrap
                            maximumLineCount: 6
                            elide: Text.ElideRight
                            textFormat: Text.StyledText
                            lineHeight: root.line
                            lineHeightMode: Text.FixedHeight
                            font.family: Theme.sans.family
                            font.pointSize: Theme.sans.body
                        }

                        // What the sender offers, each run by a click, or
                        // by the keys among the verbs: the chord on the
                        // right gives the stack the keys.
                        Item {
                            id: offering
                            width: parent.width
                            height: root.line + Theme.space.s
                            // The default is the card's own click, not one
                            // of these.
                            readonly property var offers: root.offers(card.modelData)
                            visible: offers.length > 0 && !card.keys

                            Row {
                                anchors.bottom: parent.bottom
                                spacing: Theme.space.m

                                Repeater {
                                    model: offering.offers

                                    // The pointer's area beside the words,
                                    // not among them, so the Row never
                                    // lays out what takes its own size.
                                    Item {
                                        id: offer
                                        required property var modelData
                                        required property int index
                                        width: face.width
                                        height: face.height

                                        Row {
                                            id: face
                                            spacing: Theme.space.s

                                            Text {
                                                height: root.line
                                                verticalAlignment: Text.AlignVCenter
                                                text: Theme.glyph.offer
                                                color: Theme.action
                                                font.family: Theme.mono.family
                                                font.pointSize: Theme.mono.body
                                            }

                                            Text {
                                                height: root.line
                                                verticalAlignment: Text.AlignVCenter
                                                text: offer.modelData.text
                                                color: Theme.text
                                                font.family: Theme.sans.family
                                                font.pointSize: Theme.sans.body
                                            }
                                        }

                                        MouseArea {
                                            anchors.fill: parent
                                            onClicked: {
                                                Notices.mark(card.modelData.id, true);
                                                offer.modelData.invoke();
                                            }
                                        }
                                    }
                                }
                            }

                            Text {
                                anchors.right: parent.right
                                anchors.bottom: parent.bottom
                                height: root.line
                                verticalAlignment: Text.AlignVCenter
                                text: root.reach
                                visible: !card.keys
                                color: Theme.muted
                                font.family: Theme.mono.family
                                font.pointSize: Theme.mono.body
                            }
                        }

                        // Its verbs, in the offers' place while the stack
                        // holds the keys, so the right arrow only chooses
                        // among what's already there and stepping in,
                        // through and out moves nothing (Joel, 2026-10-26:
                        // no size jump). A launcher row's shape at the
                        // card's size: the glyph in a column of its own,
                        // the name, the key that does it straight away on
                        // the right, muted, and once stepped in, the chosen
                        // one filled a step round its words.
                        Column {
                            width: parent.width
                            topPadding: Theme.space.s
                            visible: card.keys

                            Repeater {
                                model: card.keys ? root.verbs : []

                                Item {
                                    id: verb

                                    required property var modelData
                                    required property int index
                                    readonly property bool chosen: root.stepped
                                        && root.chosen === index

                                    width: parent.width
                                    height: root.line + Theme.space.xs

                                    Rectangle {
                                        anchors.fill: parent
                                        anchors.leftMargin: -Theme.space.s
                                        anchors.rightMargin: -Theme.space.s
                                        radius: Theme.inner
                                        color: Qt.alpha(Theme.selection, Theme.floating.selection)
                                        visible: verb.chosen
                                    }

                                    Text {
                                        anchors.verticalCenter: parent.verticalCenter
                                        text: verb.modelData.glyph
                                        color: Theme.action
                                        font.family: Theme.mono.family
                                        font.pointSize: Theme.mono.body
                                    }

                                    Text {
                                        x: Theme.space.xl
                                        width: chord.x - x - Theme.space.l
                                        anchors.verticalCenter: parent.verticalCenter
                                        text: verb.modelData.name
                                        textFormat: Text.PlainText
                                        elide: Text.ElideRight
                                        color: verb.chosen ? Theme.strong : Theme.text
                                        font.family: Theme.mono.family
                                        font.pointSize: Theme.mono.body
                                        font.weight: verb.chosen ? Font.Medium : Font.Normal
                                    }

                                    Chord {
                                        id: chord
                                        anchors.right: parent.right
                                        anchors.verticalCenter: parent.verticalCenter
                                        keys: verb.modelData.chord
                                        size: Theme.mono.body
                                    }

                                    MouseArea {
                                        anchors.fill: parent
                                        onClicked: {
                                            root.stepped = true;
                                            root.chosen = verb.index;
                                            root.run();
                                        }
                                    }
                                }
                            }
                        }


                        // Where it came from, a step below the message,
                        // and on the right when it came; or while the stack
                        // holds the keys, what they do here, as a prompt's
                        // footer says it, in this footer's size.
                        Item {
                            width: parent.width
                            height: root.line + Theme.space.xs

                            Text {
                                id: mark
                                anchors.bottom: parent.bottom
                                height: root.line
                                verticalAlignment: Text.AlignVCenter
                                text: String.fromCodePoint(card.from.glyph)
                                color: Theme[card.from.hue]
                                font.family: Theme.mono.family
                                font.pointSize: Theme.mono.body
                            }

                            Text {
                                anchors.left: mark.right
                                anchors.leftMargin: Theme.space.s
                                anchors.right: card.keys ? hints.left : time.left
                                anchors.rightMargin: Theme.space.l
                                anchors.bottom: parent.bottom
                                height: root.line
                                verticalAlignment: Text.AlignVCenter
                                text: card.modelData.appName
                                elide: Text.ElideRight
                                color: Theme.muted
                                font.family: Theme.mono.family
                                font.pointSize: Theme.mono.body
                            }

                            Text {
                                id: time
                                anchors.right: parent.right
                                anchors.bottom: parent.bottom
                                height: root.line
                                verticalAlignment: Text.AlignVCenter
                                readonly property real now: clock.date.getTime()
                                // How many wait behind it, before when it came.
                                readonly property int more: root.shown.length - 1
                                // Its words just copied, said as the
                                // launcher says a copy as it closes: done,
                                // in the colour of what went well.
                                readonly property bool copied: root.copied === card.modelData.id
                                text: (more > 0 ? `${more} more \u00b7 ` : "")
                                    + (copied
                                        ? `<font color="${Theme.success}">${Theme.glyph.done}`
                                            + " copied</font>"
                                        : root.since(root.arrived[card.modelData.id] ?? now, now))
                                textFormat: Text.StyledText
                                // Said for the desktop test, which sees it
                                // say so.
                                onCopiedChanged: if (copied)
                                    Report.say(`copied card ${card.modelData.summary}`)
                                visible: !card.keys
                                color: Theme.muted
                                font.family: Theme.mono.family
                                font.pointSize: Theme.mono.body
                                font.features: { "tnum": 1 }
                            }

                            KeyHints {
                                id: hints
                                anchors.right: parent.right
                                anchors.bottom: parent.bottom
                                height: root.line
                                visible: card.keys
                                size: Theme.mono.body
                                // The arrows by their marks alone, as the
                                // launcher's foot says them (KeyHints.qml).
                                keys: root.stepped ? [["\u21b5", "do it"], ["\u2190", ""]]
                                    : [["\u21b5", Words.verb(card.opens ? "verb:notice:open"
                                        : "verb:notice:forget")], ["\u2192", ""]]
                            }
                        }
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
                            && !root.away
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
