// The margin: what's on that Joel would otherwise forget, in the top right
// corner, without covering a window (docs/design.md, Shape and Kinds, the
// drawer). Each overview piece with a mark that's true right now (Keep
// Awake, do not disturb, the microphone muted, the screen shared or
// recorded, notices waiting, the battery low) hangs a pull from the
// screen's top edge, as wide as its mark, in the gap above the windows.
// Nothing true, nothing drawn.
//
// Super+O draws the drawer out of the top edge, its marks riding its front
// edge with the pulls beneath them, the one that wants Joel selected, and
// its panel below: the launcher holding that thing, with what can be done
// to it. The arrows move between things and their actions, Return does one,
// and Escape puts it all away. The pointer does the same: at the top edge
// it draws the drawer out, resting on a mark says what it is, and a click
// selects it and opens its panel.
//
// A mark wears its kind's colour only while its thing wants Joel, so what
// has colour is what matters. While the overview is open the margin goes,
// and its pieces are rows of the overview's status instead.
import Quickshell
import Quickshell.Io
import Quickshell.Wayland
import QtQuick
import QtQuick.Effects

Scope {
    id: root

    // The launcher, for the actions that open it.
    required property var launcher
    readonly property var shell: ({ launcher: root.launcher })

    // The output the keys brought it out on, or "" while they haven't.
    property string keyed: ""

    // The right edge everything in the corner shares: a step inside the
    // windows' own, so nothing lines up with a window's border by chance.
    readonly property int corner: Theme.space.m + Theme.space.s

    // What's on now: every piece with a mark that's true, as { entry,
    // piece, glyph }, in the overview's order.
    readonly property var on: Facts.all
        .filter(fact => fact.entry.mark !== null && fact.piece
            && fact.piece.present && fact.piece.active)
        .map(fact => ({ entry: fact.entry, piece: fact.piece,
            glyph: Theme.glyph[fact.entry.mark] }))

    // The colour a thing wears: its kind's, or its role's, while it wants
    // Joel, and the text's otherwise.
    function hue(thing) {
        if (!thing.piece.wants)
            return Theme.text;
        return thing.piece.role !== "text" ? Theme[thing.piece.role] : Theme[thing.piece.hue];
    }

    IpcHandler {
        target: "margin"

        // Brings the drawer out on the focused output with its keys, or
        // puts it away.
        function toggle(): void {
            const focused = Niri.workspace();
            root.keyed = root.keyed !== "" ? "" : focused ? focused.output : "*";
        }
    }

    Variants {
        model: Quickshell.screens

        Scope {
            id: here

            required property ShellScreen modelData

            readonly property bool keyed: root.keyed === "*" || root.keyed === modelData.name
            // Which thing is selected, its panel open, and which of its
            // actions, or -1.
            property int selected: -1
            property int action: 0
            // The mark the pointer rests on, for its hint.
            property int resting: -1
            property bool rested: false

            readonly property var thing: selected >= 0 && selected < root.on.length
                ? root.on[selected] : null
            readonly property bool panelled: keyed || thing !== null
            readonly property bool out: (root.on.length > 0 && pointed) || panelled
            property bool pointed: false

            // Brought out by the keys: what wants Joel first, or what's
            // nearest the corner.
            onKeyedChanged: {
                if (keyed) {
                    const wanting = root.on.findIndex(thing => thing.piece.wants);
                    selected = wanting >= 0 ? wanting : root.on.length - 1;
                    action = 0;
                } else {
                    selected = -1;
                }
            }

            function away() {
                root.keyed = "";
                selected = -1;
                resting = -1;
                rested = false;
            }

            function select(index) {
                selected = Math.max(0, Math.min(index, root.on.length - 1));
                action = 0;
            }

            function run() {
                if (!thing || thing.piece.actions.length === 0)
                    return;
                const actions = thing.piece.actions;
                const chosen = actions[Math.min(action, actions.length - 1)];
                away();
                chosen.run(root.shell);
            }

            // A thing that stops being true takes its selection with it.
            Connections {
                target: root

                function onOnChanged() {
                    if (here.selected >= root.on.length)
                        here.selected = root.on.length - 1;
                    if (root.on.length === 0 && !here.keyed)
                        here.away();
                }
            }

            // While the panel is open a click anywhere else puts it away,
            // as a menu does; the margin's own window sits above this.
            PanelWindow {
                screen: here.modelData
                visible: here.panelled
                anchors.top: true
                anchors.bottom: true
                anchors.left: true
                anchors.right: true
                exclusionMode: ExclusionMode.Ignore
                color: "transparent"
                WlrLayershell.layer: WlrLayer.Top
                WlrLayershell.namespace: "outside"
                WlrLayershell.keyboardFocus: WlrKeyboardFocus.None

                MouseArea {
                    anchors.fill: parent
                    onClicked: here.away()
                }
            }

            PanelWindow {
                id: window

                readonly property int reach: Theme.floating.reach
                // Where the drawer, the hint and the panel end on the right.
                readonly property real edge: width - root.corner

                // Measured to the ink: the deepest a mark reaches above its
                // baseline and below it.
                readonly property real up: Math.max(0, ...[...marks.children]
                    .filter(slot => slot.up !== undefined).map(slot => slot.up))
                readonly property real down: Math.max(0, ...[...marks.children]
                    .filter(slot => slot.down !== undefined).map(slot => slot.down))
                // Drawn out, the drawer: a step from the screen's edge to the
                // ink, and a step from the ink to its frame's outer edge.
                readonly property real open: Theme.space.m + up + down + Theme.space.m
                readonly property real drawerWidth: marks.width + 2 * Theme.space.m

                screen: here.modelData
                visible: !Niri.overview && (root.on.length > 0 || here.keyed)
                anchors.top: true
                anchors.right: true
                implicitWidth: 440 + root.corner + reach
                implicitHeight: open + Theme.space.m + 320 + reach
                exclusionMode: ExclusionMode.Ignore
                color: "transparent"
                WlrLayershell.layer: WlrLayer.Overlay
                WlrLayershell.namespace: "margin"
                WlrLayershell.keyboardFocus: here.panelled
                    ? WlrKeyboardFocus.Exclusive : WlrKeyboardFocus.None

                // A pull coming or going changes what's drawn while the
                // margin stays up.
                Reported {
                    id: reported
                    name: "margin"
                }

                Connections {
                    target: root

                    function onOnChanged() {
                        reported.changed();
                        const marks = root.on.map(fact => fact.entry.mark);
                        Report.say(`holds margin ${marks.join(" ")}`);
                    }
                }

                // The pointer reaches the strip above the windows over the
                // pulls, a column wider at its left, and once the drawer is
                // out, the drawer and what hangs below it.
                mask: Region {
                    regions: [
                        Region { item: strip },
                        Region { item: drawer.height > 0 ? frame : null },
                        Region { item: panel.visible ? panel : null }
                    ]
                }
                BackgroundEffect.blurRegion: Region {
                    regions: [
                        Region { item: drawer.height > 0 ? frame : null },
                        Region { item: hint.visible ? hint.area.item : null },
                        Region { item: panel.visible ? panel.area.item : null }
                    ]
                }

                // The notices make way below whatever hangs here.
                Binding {
                    target: Notices
                    property: "below"
                    value: !here.out ? 0
                        : window.open + Theme.space.m
                            + (panel.visible ? panel.height : hint.height)
                }

                Item {
                    id: strip
                    x: window.edge - window.drawerWidth - Theme.space.xl
                    width: window.width - x
                    height: Theme.space.m
                }

                // Passive, so it hears the pointer over the marks too.
                HoverHandler {
                    onHoveredChanged: here.pointed = hovered
                }

                // The drawer's shadows, outside its frame.
                Repeater {
                    model: Theme.floating.shadows

                    RectangularShadow {
                        required property var modelData

                        visible: drawer.height > 0
                        anchors.fill: frame
                        offset.y: modelData.offset
                        blur: modelData.blur
                        radius: Theme.radius
                        color: Qt.alpha(Theme.shadow, modelData.opacity)
                    }
                }

                // Its frame starts above the screen's edge, so its top
                // border and top corners are never seen: it hangs from the
                // edge, rounded only below.
                Rectangle {
                    id: frame
                    x: window.edge - window.drawerWidth
                    y: -Theme.radius
                    width: window.drawerWidth
                    height: drawer.height + Theme.radius
                    visible: drawer.height > 0
                    radius: Theme.radius
                    border.width: Theme.border
                    border.color: Theme.rule
                    // Deeper toward the back, where it runs under the edge.
                    gradient: Gradient {
                        GradientStop {
                            position: 0
                            color: Qt.alpha(Theme.deepest, Theme.floating.opacity)
                        }
                        GradientStop {
                            position: 1
                            color: Qt.alpha(Theme.background, Theme.floating.opacity)
                        }
                    }
                }

                Item {
                    id: drawer
                    x: frame.x
                    width: frame.width
                    height: here.out ? window.open : 0
                    clip: true

                    Behavior on height {
                        NumberAnimation {
                            duration: here.out ? Theme.motion.medium : Theme.motion.short
                            easing.type: Easing.BezierSpline
                            easing.bezierCurve: Theme.motion.curve
                        }
                    }

                    // What's inside rides the front edge out from under the
                    // screen's.
                    Item {
                        id: content
                        y: drawer.height - window.open
                        width: drawer.width
                        height: window.open

                        // The selected mark's fill: a part, a step inside
                        // the frame and a little round the ink.
                        Rectangle {
                            readonly property var slot: {
                                slots.count;
                                return here.selected >= 0 ? slots.itemAt(here.selected) : null;
                            }
                            visible: slot !== null
                            x: visible ? marks.x + slot.x - Theme.space.xs : 0
                            y: Theme.space.m - Theme.space.xs
                            width: visible ? slot.width + 2 * Theme.space.xs : 0
                            height: window.up + window.down + 2 * Theme.space.xs
                            radius: Theme.inner
                            color: Qt.alpha(Theme.selection, Theme.floating.selection)
                        }

                        Row {
                            id: marks
                            x: Theme.space.m
                            spacing: Theme.space.m

                            Repeater {
                                id: slots
                                model: root.on

                                // A mark set by its ink, and its badge a
                                // little after, their tops level.
                                Item {
                                    id: slot

                                    required property var modelData
                                    required property int index
                                    readonly property real up: -glyphInk.tightBoundingRect.y
                                    readonly property real down: glyphInk.tightBoundingRect.y
                                        + glyphInk.tightBoundingRect.height
                                    readonly property real glyphWidth:
                                        glyphInk.tightBoundingRect.width
                                    // Where the marks' baselines sit, level across them.
                                    readonly property real line: Theme.space.m + window.up

                                    width: glyphWidth + (badge.text === "" ? 0
                                        : Theme.space.xs + badgeInk.tightBoundingRect.width)
                                    height: window.open

                                    TextMetrics {
                                        id: glyphInk
                                        font: glyph.font
                                        text: glyph.text
                                    }

                                    TextMetrics {
                                        id: badgeInk
                                        font: badge.font
                                        text: badge.text
                                    }

                                    Text {
                                        id: glyph
                                        x: -glyphInk.tightBoundingRect.x
                                        y: slot.line - baselineOffset
                                        text: slot.modelData.glyph
                                        color: root.hue(slot.modelData)
                                        font.family: Theme.mono.family
                                        font.pointSize: Theme.mono.body
                                    }

                                    Text {
                                        id: badge
                                        x: slot.glyphWidth + Theme.space.xs
                                            - badgeInk.tightBoundingRect.x
                                        y: slot.line + glyphInk.tightBoundingRect.y
                                            - badgeInk.tightBoundingRect.y - baselineOffset
                                        text: slot.modelData.piece.badge
                                        color: slot.modelData.piece.wants
                                            ? Theme.strong : Theme.text
                                        font.family: Theme.mono.family
                                        font.pointSize: Theme.mono.small
                                        font.features: { "tnum": 1 }
                                    }

                                    MouseArea {
                                        x: -Theme.space.m / 2
                                        width: parent.width + Theme.space.m
                                        height: parent.height
                                        hoverEnabled: true
                                        onContainsMouseChanged: {
                                            if (containsMouse) {
                                                here.resting = slot.index;
                                                if (!here.rested)
                                                    rest.restart();
                                            } else if (here.resting === slot.index) {
                                                here.resting = -1;
                                            }
                                        }
                                        onClicked: {
                                            if (here.selected === slot.index && !here.keyed)
                                                here.away();
                                            else
                                                here.select(slot.index);
                                        }
                                    }
                                }
                            }
                        }

                        // The shade toward the back, along the edge it runs
                        // under.
                        Rectangle {
                            width: parent.width
                            height: Theme.space.m
                            gradient: Gradient {
                                GradientStop {
                                    position: 0
                                    color: Qt.alpha(Theme.deepest, 0.9)
                                }
                                GradientStop {
                                    position: 1
                                    color: "transparent"
                                }
                            }
                        }
                    }
                }

                // A hint waits for the pointer to rest, then follows it.
                Timer {
                    id: rest
                    interval: 400
                    onTriggered: here.rested = here.resting >= 0
                }

                Connections {
                    target: here

                    function onOutChanged() {
                        if (!here.out)
                            here.rested = false;
                    }
                }

                // The pulls: one under each mark, as wide as its ink, hanging
                // from the screen's edge at rest and from the drawer's once
                // it's out. A hairline in the drawer's border colour runs
                // round each, its top hidden above whichever edge it hangs
                // from, lifted off the picture by the theme's text shadows.
                Lifted {
                    id: pulls
                    x: drawer.x + marks.x - Theme.border
                    y: drawer.height - Theme.border

                    // Lifted draws from its own corner, so the pulls sit in
                    // one piece that starts there.
                    Item {
                        width: marks.width + 2 * Theme.border
                        height: Theme.space.xs + 2 * Theme.border

                        Repeater {
                            model: root.on

                            Rectangle {
                                required property var modelData
                                required property int index
                                readonly property var slot: {
                                    slots.count;
                                    return slots.itemAt(index);
                                }

                                x: slot ? slot.x : 0
                                width: (slot ? slot.glyphWidth : 0) + 2 * Theme.border
                                height: Theme.space.xs + 2 * Theme.border
                                color: Theme.rule

                                Rectangle {
                                    x: Theme.border
                                    y: Theme.border
                                    width: parent.width - 2 * Theme.border
                                    height: Theme.space.xs
                                    color: root.hue(parent.modelData)
                                }
                            }
                        }
                    }
                }

                // What the mark rested on is, in words, below the drawer.
                Floating {
                    id: hint

                    readonly property var thing: here.resting >= 0 && here.resting < root.on.length
                        ? root.on[here.resting] : null

                    visible: here.out && here.rested && thing !== null && !panel.visible
                    x: window.edge - width
                    y: window.open + Theme.space.m
                    width: words.width + 2 * Theme.space.s
                    height: capital.tightBoundingRect.height + 2 * Theme.space.s

                    TextMetrics {
                        id: capital
                        font: words.font
                        text: "H"
                    }

                    Text {
                        id: words
                        x: Theme.space.s - Theme.border
                        y: Theme.space.s - Theme.border
                            - capital.tightBoundingRect.y - baselineOffset
                        text: hint.thing ? hint.thing.piece.hint : ""
                        color: Theme.text
                        font.family: Theme.sans.family
                        font.pointSize: Theme.sans.body
                    }
                }

                // The panel is the launcher holding the thing: its glyph,
                // its name in a chip and how it stands; a hairline; what can
                // be done, in the launcher's rows; and the launcher's footer.
                Floating {
                    id: panel

                    readonly property var thing: here.thing

                    visible: here.panelled
                    keys: true
                    x: window.edge - width
                    y: (root.on.length > 0 ? window.open : 0) + Theme.space.m
                    width: 440
                    focus: visible

                    Keys.onEscapePressed: here.away()
                    Keys.onLeftPressed: here.select(here.selected - 1)
                    Keys.onRightPressed: here.select(here.selected + 1)
                    Keys.onUpPressed: here.action = Math.max(0, here.action - 1)
                    Keys.onDownPressed: here.action = panel.thing
                        ? Math.min(here.action + 1, panel.thing.piece.actions.length - 1) : 0
                    Keys.onReturnPressed: here.run()
                    Keys.onEnterPressed: here.run()

                    Column {
                        width: parent.width

                        // Nothing on says so, in the launcher's quiet row.
                        Item {
                            visible: panel.thing === null
                            width: parent.width
                            height: Theme.space.xxxl + 2 * Theme.inset

                            Choice {
                                x: Theme.inset
                                y: Theme.inset
                                width: parent.width - 2 * Theme.inset
                                glyph: String.fromCodePoint(0xf0674)
                                hue: Theme.voice
                                quiet: true
                                name: "Nothing's on, and nothing's waiting."
                            }
                        }

                        Item {
                            visible: panel.thing !== null
                            width: parent.width
                            height: Theme.space.xxxl + 2 * Theme.inset

                            Text {
                                id: lead
                                x: Theme.inset + Theme.space.l
                                anchors.verticalCenter: parent.verticalCenter
                                text: panel.thing ? panel.thing.glyph : ""
                                color: panel.thing ? (panel.thing.piece.role !== "text"
                                    ? Theme[panel.thing.piece.role] : Theme[panel.thing.piece.hue])
                                    : Theme.text
                                font.family: Theme.mono.family
                                font.pointSize: Theme.mono.large
                            }

                            Rectangle {
                                id: chip
                                x: Theme.inset + Theme.space.l + Theme.space.xxl
                                anchors.verticalCenter: parent.verticalCenter
                                width: name.implicitWidth + 2 * Theme.space.m
                                height: Theme.space.xxl
                                radius: Theme.inner
                                color: Theme.raised

                                Text {
                                    id: name
                                    anchors.centerIn: parent
                                    text: panel.thing ? panel.thing.piece.title : ""
                                    color: Theme.strong
                                    font.family: Theme.mono.family
                                    font.pointSize: Theme.mono.large
                                }
                            }

                            Text {
                                x: chip.x + chip.width + Theme.space.m
                                width: parent.width - x - Theme.inset - Theme.space.l
                                anchors.verticalCenter: parent.verticalCenter
                                elide: Text.ElideRight
                                text: panel.thing ? panel.thing.piece.value : ""
                                color: Theme.muted
                                font.family: Theme.mono.family
                                font.pointSize: Theme.mono.large
                                font.features: { "tnum": 1 }
                            }
                        }

                        Rectangle {
                            visible: panel.thing !== null
                            width: parent.width
                            height: Theme.border
                            color: Theme.edge
                        }

                        Column {
                            visible: panel.thing !== null
                            x: Theme.inset
                            width: parent.width - 2 * Theme.inset
                            topPadding: Theme.inset
                            bottomPadding: Theme.inset

                            Repeater {
                                model: panel.thing ? panel.thing.piece.actions : []

                                Choice {
                                    required property var modelData
                                    required property int index

                                    selected: index === here.action
                                    glyph: modelData.glyph
                                    hue: Theme.action
                                    name: modelData.name
                                    chord: modelData.chord ?? []
                                    onClicked: {
                                        here.action = index;
                                        here.run();
                                    }
                                    onPointed: here.action = index
                                }
                            }

                            // What more there is to know, and what Return does.
                            Item {
                                width: parent.width
                                height: Theme.space.xxl + Theme.space.s

                                Text {
                                    x: Theme.space.l
                                    width: done.x - x - Theme.space.xl
                                    anchors.verticalCenter: parent.verticalCenter
                                    elide: Text.ElideRight
                                    text: panel.thing ? panel.thing.piece.context : ""
                                    color: Theme.muted
                                    font.family: Theme.mono.family
                                    font.pointSize: Theme.mono.large
                                }

                                Text {
                                    id: done
                                    anchors.right: parent.right
                                    anchors.rightMargin: Theme.space.l
                                    anchors.verticalCenter: parent.verticalCenter
                                    visible: panel.thing !== null
                                        && panel.thing.piece.actions.length > 0
                                    text: `<font color="${Theme.text}">\u21b5</font> run`
                                    textFormat: Text.StyledText
                                    color: Theme.muted
                                    font.family: Theme.mono.family
                                    font.pointSize: Theme.mono.large
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
