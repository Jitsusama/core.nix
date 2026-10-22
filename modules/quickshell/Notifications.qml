// The notification server: the notices on screen are one stack in the top
// right corner, the oldest not yet dealt with in front and up to three
// newer behind it as edges, a critical one going to the front. A click on
// the front one opens what it's about, or puts it away when it's about
// nothing; one on an edge brings that one forward. The front one leaves by
// itself when its time is up, unless it's critical. While notices are held
// (Quiet.qml), only a critical one shows.
//
// Super+N gives the stack the keys while there are notices (`pressed` says
// what each does, and the front card's footer shows them), and opens the
// history in the launcher when there are none or it's pressed again.
//
// The notices sit a step inside the window under the top right corner, so
// a notice lies on one window rather than across a border, or a step inside
// where a window's corner would be when there's none (`inside` says how),
// and they make way below the margin's drawer while it's out.
//
// Each is kept in the session's history as it arrives (Notices.qml), for
// the launcher to find later, where the ones still here can be answered
// or let go, and while it's open the corner leaves them to it. A notice
// offering something to do (Reply, Play it) lists it above its footer with
// the chord that takes the stack beside it, and the right arrow chooses
// one.
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

    // The notices on screen, as one stack: their ids, the front first. A
    // critical one goes to the front and the rest in the order they came,
    // so a newer one waits its turn behind the one before it. One kept
    // across a reload of the shell, which arrives unheard, joins the back.
    property var order: []
    readonly property var shown: {
        const live = server.trackedNotifications.values.map(notification => notification.id);
        const known = order.filter(id => live.includes(id));
        return known.concat(live.filter(id => !known.includes(id)));
    }
    readonly property var front: {
        const id = shown[0];
        return server.trackedNotifications.values.find(notification => notification.id === id)
            ?? null;
    }
    // How many behind the front show their edge, a step lower and a step
    // narrower each, and how far.
    readonly property int behind: 3
    readonly property int peek: Theme.space.s
    // The front card's height, which the cards behind it take.
    property real frontHeight: 0

    // Whether the stack holds the keys, which Super+N gives it while there
    // are notices, and which of the front one's offers is chosen, -1 for
    // the notice itself.
    property bool holding: false
    property int chosen: -1
    onShownChanged: {
        chosen = -1;
        if (shown.length === 0)
            holding = false;
    }
    onHoldingChanged: Report.say(holding ? "holds notices" : "lets notices go")

    // What a notice's sender does when it's chosen, or null, and what else
    // it offers, which its card lists.
    function opens(notification) {
        return notification.actions.find(action => action.identifier === "default") ?? null;
    }
    function offers(notification) {
        return notification.actions.filter(action => action.identifier !== "default");
    }

    // The front one done with, by its offer chosen, or what it's about, or
    // let go when it's about nothing to open.
    function run() {
        const notification = front;
        if (!notification)
            return;
        Notices.mark(notification.id, true);
        const offer = chosen >= 0 ? offers(notification)[chosen] ?? null : opens(notification);
        if (offer)
            offer.invoke();
        else
            notification.dismiss();
    }

    // Turns the stack, so the next comes to the front, or the one before.
    function turn(by) {
        const ids = shown.slice();
        if (ids.length < 2)
            return;
        order = by > 0 ? ids.slice(1).concat(ids.slice(0, 1)) : ids.slice(-1).concat(ids.slice(0, -1));
    }

    // The keys while the stack holds them: Return does the front one,
    // the arrows choose among what it offers and move through the stack,
    // Delete or Backspace lets it go and with Shift all of them, Tab hands
    // on to the history, and Escape gives the keys back.
    function pressed(event) {
        const all = (event.modifiers & Qt.ShiftModifier) !== 0;
        switch (event.key) {
        case Qt.Key_Return:
        case Qt.Key_Enter:
            run();
            break;
        case Qt.Key_Right:
            chosen = Math.min(chosen + 1, front ? offers(front).length - 1 : -1);
            break;
        case Qt.Key_Left:
            chosen = Math.max(chosen - 1, -1);
            break;
        case Qt.Key_Down:
            turn(1);
            break;
        case Qt.Key_Up:
            turn(-1);
            break;
        case Qt.Key_Delete:
        case Qt.Key_Backspace:
            if (all)
                putAway();
            else if (front)
                front.dismiss();
            break;
        case Qt.Key_Tab:
            history();
            break;
        case Qt.Key_Escape:
            holding = false;
            break;
        default:
            return;
        }
        event.accepted = true;
    }

    function putAway() {
        for (const notification of server.trackedNotifications.values.slice())
            notification.dismiss();
    }

    // The session's notices in the launcher, where past ones are too.
    function history() {
        holding = false;
        if (launcher)
            launcher.into("notices");
    }

    // The chord that opens the notices, as the launcher draws chords.
    readonly property string reach: {
        const opener = Shortcuts.actions.find(action => action.opens === "notices");
        return opener ? opener.chord.map(key => key.text).join(" + ") : "";
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
            // One its sender marks transient asks not to be kept, as niri's
            // screenshot notice does.
            const urgent = notification.urgency === NotificationUrgency.Critical;
            const held = Quiet.on && !urgent;
            if (!notification.transient)
                Notices.arrived(notification, urgent, held);
            if (held)
                Report.say(`held ${notification.summary}`);
            const id = notification.id;
            if (!held)
                root.order = urgent ? [id].concat(root.order) : root.order.concat([id]);
            notification.closed.connect(reason => Notices.left(id,
                reason === NotificationCloseReason.Expired ? "expired"
                : reason === NotificationCloseReason.CloseRequested ? "withdrawn"
                : "dismissed"));
            // One held is let go at once, so it never shows.
            notification.tracked = !held;
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

        visible: (server.trackedNotifications.values.length > 0 || root.playing)
            && !Notices.gathered
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
            y: Math.max(root.inside.top, Theme.space.m + Theme.space.s + Notices.below)
            onHeightChanged: reported.changed()
            // Where the pointer reaches and what niri blurs follow a card
            // only as it moves within the column, so they're gathered again
            // as the column moves, or they'd stay behind as a shadow where
            // it was.
            onYChanged: window.gather()

            Behavior on y {
                NumberAnimation {
                    duration: Notices.below > 0 ? Theme.motion.medium : Theme.motion.short
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
            // cloudy surface shows only their frames beneath.
            Item {
                id: pile

                width: stack.width
                height: root.shown.length === 0 ? 0
                    : root.frontHeight + Math.min(root.shown.length - 1, root.behind) * root.peek
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
                    height: front ? implicitHeight : root.frontHeight
                    urgent: modelData.urgency === NotificationUrgency.Critical
                    keys: front && root.holding

                    onFrontChanged: if (front) root.frontHeight = implicitHeight
                    onImplicitHeightChanged: if (front) root.frontHeight = implicitHeight
                    // Its frame moves as the stack turns, which its window's
                    // regions follow only when told.
                    onRankChanged: Qt.callLater(window.gather)
                    onHeightChanged: Qt.callLater(window.gather)

                    Component.onCompleted: Report.say(`shows card ${modelData.summary}`)

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
                                root.order = [card.modelData.id].concat(
                                    root.shown.filter(id => id !== card.modelData.id));
                            } else if (card.opens) {
                                Notices.mark(card.modelData.id, true);
                                card.opens.invoke();
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

                        // What the sender offers, each run by a click or by
                        // the keys: the chord on the right opens the
                        // launcher on them, the first held for Return.
                        Item {
                            id: offering
                            width: parent.width
                            height: root.line + Theme.space.s
                            // The default is the card's own click, not one
                            // of these.
                            readonly property var offers: root.offers(card.modelData)
                            visible: offers.length > 0

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

                                        // Chosen by the arrows: a part's
                                        // fill, a step round the words.
                                        Rectangle {
                                            anchors.fill: parent
                                            anchors.margins: -Theme.space.xs
                                            anchors.leftMargin: -Theme.space.s
                                            anchors.rightMargin: -Theme.space.s
                                            radius: Theme.inner
                                            color: Theme.selection
                                            visible: card.keys && root.chosen === offer.index
                                        }

                                        Row {
                                            id: face
                                            spacing: Theme.space.s

                                            Text {
                                                height: root.line
                                                verticalAlignment: Text.AlignVCenter
                                                text: String.fromCodePoint(0xf0ce0)
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

                        // Where it came from and when, a step below the message.
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
                                anchors.right: time.left
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
                                text: (more > 0 ? `${more} more \u00b7 ` : "")
                                    + root.since(root.arrived[card.modelData.id] ?? now, now)
                                color: Theme.muted
                                font.family: Theme.mono.family
                                font.pointSize: Theme.mono.body
                                font.features: { "tnum": 1 }
                            }
                        }

                        // While the stack holds the keys, what they do here,
                        // as a prompt's footer says, a step below where it
                        // came from, in that footer's size.
                        Item {
                            width: parent.width
                            height: root.line + Theme.space.xs
                            visible: card.keys

                            KeyHints {
                                anchors.right: parent.right
                                anchors.bottom: parent.bottom
                                size: Theme.mono.body
                                keys: [
                                    ["\u21b5", root.chosen >= 0 ? "do it" : card.opens ? "open" : "let go"],
                                    ["\u232b", "let go"],
                                    ["\u21e7\u232b", "all"],
                                    ["\u21e5", "all notices"]
                                ]
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
                    // go from under them, starting again in full after.
                    Timer {
                        running: !card.urgent && !Notices.gathered && card.front && !root.holding
                        onRunningChanged: Report.say(
                            `${running ? "counts" : "waits"} card ${card.modelData.summary}`)
                        interval: card.modelData.expireTimeout > 0
                            ? card.modelData.expireTimeout
                            : card.modelData.actions.length > 0 ? 10000 : 5000
                        onTriggered: card.modelData.expire()
                    }
                }
            }
            }
        }
    }
}
