// The notification server: the notices on screen are one stack in the top
// right corner, the oldest not yet dealt with in front and up to three
// newer behind it as edges, a critical one going to the front. A click on
// the front one opens what it's about, or puts it away when it's about
// nothing; one on an edge brings that one forward. The front one leaves by
// itself when its time is up, unless it's critical. While notices are held
// (Quiet.qml), only a critical one shows.
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
// Each is kept in the session's history as it arrives (Notices.qml), for
// the launcher to find later, where the ones still here can be answered
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
import QtQuick.Effects
import "Levels.js" as Levels
import "Stack.js" as Stack

Scope {
    id: root

    // Wide enough that what a notice says holds about 44 letters of the
    // reading face at the launcher's size on a line, inside its room on
    // each side: a short message in one or two lines.
    readonly property int width: 440
    // The launcher's line, which every line of a card keeps at its size:
    // six units.
    readonly property int pitch: Theme.space.xl
    // How far the ink of every card stands from its frame on every side,
    // inside the hairline: an inset and a step, as the launcher's card.
    readonly property int room: Theme.inset + Theme.space.l

    // A capital in the face that says who a notice is from, so the room
    // above is measured to its top rather than to its line's.
    TextMetrics {
        id: capital
        font.family: Theme.level.name.family
        font.pointSize: Theme.level.name.pointSize
        font.weight: Theme.level.name.weight
        text: "H"
    }

    // Whether what's playing is called for. The card's own visibility can't
    // say, since an item in a hidden window counts as hidden.
    readonly property bool playing: Music.shown && Music.card !== null
    // What the card says, or nothing while there's no player to say it of.
    readonly property var song: Music.card ?? {
        title: "", artist: "", art: "", from: "", mark: "", where: ""
    }

    // When each notification arrived, by its id, for the footer's time.
    property var arrived: ({})

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
        const corner = { right: Theme.gap + step, top: Theme.gap + step, width: root.width };
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
            .filter(id => !(id in Notices.resting) && !held.awayIds.includes(id)))
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
    readonly property var live: server.trackedNotifications.values
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
        const body = root.plain(notification.body);
        return body === "" ? notification.summary : `${notification.summary}\n${body}`;
    }

    function plain(body) {
        return body.replace(/<[^>]*>/g, "").replace(/&lt;/g, "<")
            .replace(/&gt;/g, ">").replace(/&amp;/g, "&");
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
        for (const notification of server.trackedNotifications.values
            .filter(notification => ids.includes(notification.id)))
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
        // otherwise the history opens; asked again, it lets them go, as
        // Vim's popups close (VKCF3Z's vi-paradigm spec, Notices).
        function take(): void {
            if (root.holding)
                root.holding = false;
            else if (root.shown.length === 0)
                root.history();
            else
                root.holding = true;
        }

        // Every notice missed counted as seen, as the margin's Let Them Go
        // does.
        function seen(): void {
            Notices.seen();
        }

        // Where the front card is drawn, as a fraction of the screen across
        // and down, for a test to click on it, since no photograph says
        // where a card without a coloured mark is; and what it's signed,
        // the glyph it wears and that glyph's hue.
        function front(): string {
            const card = cards.itemAt(server.trackedNotifications.values
                .findIndex(one => one.id === root.shown[0])) as Floating;
            const screen = window.screen;
            if (!card || !screen || !root.front)
                return "{}";
            const at = card.mapToItem(null, 0, 0);
            const left = screen.width - window.width + at.x;
            const from = Notices.sender(root.front.appName);
            return JSON.stringify({
                signed: from.name,
                glyph: String.fromCodePoint(from.glyph),
                hue: from.hue,
                left: left / screen.width,
                top: at.y / screen.height,
                right: (left + card.width) / screen.width,
                bottom: (at.y + card.height) / screen.height
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

    NotificationServer {
        id: server
        bodySupported: true
        actionsSupported: true
        // A reply typed on the card, as the held list's :reply writes one.
        inlineReplySupported: true
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

    // One said again in place that tells something new comes back to the
    // front, as one arriving would, unless notices are held and it isn't
    // urgent, when it keeps resting. The server tells nothing of a notice
    // replaced by its id, so Arrivals says it through Notices.
    Connections {
        target: Notices

        function onRaisedChanged() {
            const id = Notices.raised?.id;
            const notification = server.trackedNotifications.values
                .find(one => one.id === id);
            if (!notification
                || Quiet.on && notification.urgency !== NotificationUrgency.Critical)
                return;
            Notices.wake(id);
            root.order = Stack.forward(root.order.includes(id) ? root.order
                : root.order.concat([id]), id);
            Report.say(`raises ${notification.summary}`);
        }
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

        function gather() {
            const found = root.playing ? [track.area] : [];
            if (root.holding)
                found.push(held.area);
            for (let i = 0; i < cards.count; i++) {
                const card = cards.itemAt(i) as Floating;
                if (card)
                    found.push(card.area);
            }
            areas = found;
        }

        visible: (root.shown.length > 0 || root.playing || root.holding)
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
            id: stack
            x: Theme.floating.reach
            y: Math.max(root.inside.top, Theme.gap + Theme.space.s)
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

                // A notice's three groups and room, as its neighbours'
                // are: the track, who plays it, and the player and where
                // in the track.
                Column {
                    id: said
                    x: root.room
                    width: parent.width - 2 * x
                    topPadding: root.room - title.baselineOffset - capital.tightBoundingRect.y
                    bottomPadding: root.room - playedBy.height + playedBy.inkBottom

                    Item {
                        width: parent.width
                        height: Math.max(cover.y + cover.height, lines.implicitHeight)

                        // Two lines tall from the title's capital, its
                        // corners the theme's inner radius, and the raised
                        // colour until it arrives.
                        Rectangle {
                            id: cover
                            y: title.baselineOffset + capital.tightBoundingRect.y
                            width: 2 * root.pitch
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
                                id: title
                                width: parent.width
                                text: root.song.title
                                color: Theme.strong
                                wrapMode: Text.Wrap
                                maximumLineCount: 2
                                elide: Text.ElideRight
                                lineHeight: root.pitch
                                lineHeightMode: Text.FixedHeight
                                font.family: Theme.level.name.family
                                font.pointSize: Theme.level.name.pointSize
                                font.weight: Theme.level.name.weight
                            }

                            Text {
                                width: parent.width
                                text: root.song.artist
                                visible: text !== ""
                                topPadding: Theme.space.xs
                                color: Theme.text
                                elide: Text.ElideRight
                                lineHeight: root.pitch
                                lineHeightMode: Text.FixedHeight
                                font.family: Theme.level.words.family
                                font.pointSize: Theme.level.words.pointSize
                                font.weight: Theme.level.words.weight
                                font.variableAxes: Theme.level.words.variableAxes
                            }
                        }
                    }

                    // The player in a column of its own and where in the
                    // track, a step below, as a notice's signature.
                    Item {
                        id: playedBy
                        readonly property real inkBottom: player.y + player.baselineOffset
                        width: parent.width
                        height: Theme.space.m + root.pitch

                        Text {
                            y: Theme.space.m
                            lineHeight: root.pitch
                            lineHeightMode: Text.FixedHeight
                            text: root.song.mark
                            color: Theme.program
                            font.family: Theme.level.note.family
                            font.pointSize: Theme.level.note.pointSize
                            font.weight: Theme.level.note.weight
                        }

                        Text {
                            id: player
                            x: Theme.space.xxl
                            y: Theme.space.m
                            width: place.x - x - Theme.space.l
                            lineHeight: root.pitch
                            lineHeightMode: Text.FixedHeight
                            text: root.song.from
                            elide: Text.ElideRight
                            color: Theme.level.note.colour
                            font.family: Theme.level.note.family
                            font.pointSize: Theme.level.note.pointSize
                            font.weight: Theme.level.note.weight
                        }

                        Text {
                            id: place
                            anchors.right: parent.right
                            y: Theme.space.m
                            lineHeight: root.pitch
                            lineHeightMode: Text.FixedHeight
                            text: root.song.where
                            color: Theme.level.note.colour
                            font.family: Theme.level.note.family
                            font.pointSize: Theme.level.note.pointSize
                            font.weight: Theme.level.note.weight
                        }
                    }
                }

                MouseArea {
                    width: parent.width
                    height: said.height
                    onClicked: Music.shown = false
                }
            }

            // Held, the stack is the quickfix list, in its place.
            HeldNotices {
                id: held

                stack: root
                live: root.live
                width: stack.width
                visible: root.holding
                screenHeight: window.screen ? window.screen.height : 768
                onVisibleChanged: Qt.callLater(window.gather)
                onHeightChanged: Qt.callLater(window.gather)
            }

            // The notices, one stack: the front one whole, and behind it
            // the next few as edges, each a step lower and a step narrower,
            // at the front one's height with nothing drawn in them, so its
            // cloudy surface shows only their frames beneath. It reaches the
            // lower of the front card and the last edge as they glide.
            Item {
                id: pile

                width: stack.width
                height: root.shown.length === 0 || root.holding ? 0 : Math.max(root.frontHeight,
                    root.edgeHeight + Math.min(root.shown.length - 1, root.behind) * root.peek)
                visible: root.shown.length > 0 && !root.holding

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
                    // front. What else it offers the keys reach (Joel
                    // wants keys first), and an act's own area, drawn
                    // later, takes a click on it.
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

                    // Three groups, as Joel reads a notice: who or what
                    // it's about, what it says, and the signature of what
                    // sent it with when, each at its level and every line
                    // on the one pitch, so the sizes rank rather than
                    // jostle: who and what at the launcher's size, the
                    // signature a step smaller (Joel, 2026-10-24: "the
                    // differing font sizes, and too much information on
                    // lines really felt gross and unrefined"; 2026-11-04:
                    // "we are not levraging different weights and sizes
                    // enough"). The frame stands as far from the ink on
                    // every side, to the capital's top and the
                    // signature's baseline (Joel, 2026-11-03: "feeling
                    // quite cluttered").
                    Column {
                        id: words
                        visible: card.front
                        x: root.room
                        width: parent.width - 2 * x
                        topPadding: root.room - who.baselineOffset - capital.tightBoundingRect.y
                        bottomPadding: root.room - signature.height + signature.inkBottom

                        Text {
                            id: who
                            width: parent.width
                            text: card.modelData.summary
                            // Plain, as the spec has a summary: only the
                            // body carries markup.
                            textFormat: Text.PlainText
                            color: Theme.strong
                            wrapMode: Text.Wrap
                            lineHeight: root.pitch
                            lineHeightMode: Text.FixedHeight
                            font.family: Theme.level.name.family
                            font.pointSize: Theme.level.name.pointSize
                            font.weight: Theme.level.name.weight
                        }

                        Text {
                            width: parent.width
                            text: card.modelData.body
                            visible: card.modelData.body !== ""
                            // A line that belongs with the one above.
                            topPadding: Theme.space.xs
                            color: Theme.text
                            wrapMode: Text.Wrap
                            maximumLineCount: 6
                            elide: Text.ElideRight
                            textFormat: Text.StyledText
                            lineHeight: root.pitch
                            lineHeightMode: Text.FixedHeight
                            font.family: Theme.level.words.family
                            font.pointSize: Theme.level.words.pointSize
                            font.weight: Theme.level.words.weight
                            font.variableAxes: Theme.level.words.variableAxes
                        }

                        // What sent it, a step below: its mark in a column
                        // of its own and the name Joel knows it by, never
                        // its program's; on the right when it came.
                        Item {
                            id: signature
                            // Where its line's baseline falls, which ends
                            // the card.
                            readonly property real inkBottom: sent.y + sent.baselineOffset
                            width: parent.width
                            height: Theme.space.m + root.pitch

                            Text {
                                id: mark
                                y: Theme.space.m
                                lineHeight: root.pitch
                                lineHeightMode: Text.FixedHeight
                                text: String.fromCodePoint(card.from.glyph)
                                color: Theme[card.from.hue]
                                font.family: Theme.level.note.family
                                font.pointSize: Theme.level.note.pointSize
                                font.weight: Theme.level.note.weight
                            }

                            Text {
                                id: sent
                                x: Theme.space.xxl
                                y: Theme.space.m
                                width: time.x - x - Theme.space.l
                                lineHeight: root.pitch
                                lineHeightMode: Text.FixedHeight
                                text: card.from.name
                                textFormat: Text.PlainText
                                elide: Text.ElideRight
                                color: Theme.level.note.colour
                                font.family: Theme.level.note.family
                                font.pointSize: Theme.level.note.pointSize
                                font.weight: Theme.level.note.weight
                            }

                            Text {
                                id: time
                                anchors.right: parent.right
                                y: Theme.space.m
                                lineHeight: root.pitch
                                lineHeightMode: Text.FixedHeight
                                readonly property real now: clock.date.getTime()
                                // How many wait behind it, before when it came.
                                readonly property int more: root.shown.length - 1
                                text: (more > 0 ? `${more} more \u00b7 ` : "")
                                    + root.since(root.arrived[card.modelData.id] ?? now, now)
                                textFormat: Text.PlainText
                                color: Theme.level.note.colour
                                font.family: Theme.level.note.family
                                font.pointSize: Theme.level.note.pointSize
                                font.weight: Theme.level.note.weight
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
