// The arrival line: the one notice that still waits on Joel, as one frame a step inside the
// top of the focused screen, centred on the launcher's line where Super+S's line and the
// launcher stand, so a notice comes where everything else of the shell does rather than in
// a window of its own to the side (Joel, 2026-11-13: "the notification thing should just be
// part of the launcher"). It takes no keys and moves no focus: Super+N opens the launcher on
// it, and a click does what its card's did. NoticeLine.js chooses it, steps it aside while
// the launcher shows the notices or the talk line is up, and counts the rest that wait.
//
// Its ink is the line's (VKCF3Z, 2026-11-13, attachments/line-colour-and-space.md): the
// sender's mark in its own hue, the accent nowhere since no keys come here, and colour on
// glyphs, never on text.
//
// Its layer is mapped only while it shows a notice, so a quiet desktop draws nothing and
// wakes nothing for it (Y2YBTS, 2026-11-13). A notice stays as long as its card would have,
// the stack's time for it, which keeps counting while the line shows it; a critical one
// stays until it's dealt with. Only Joel's own click acts on it.
pragma ComponentBehavior: Bound
import Quickshell
import Quickshell.Services.Notifications
import Quickshell.Wayland
import QtQuick
import KeyTimes
import "NoticeLine.js" as Line

Scope {
    id: root

    // The launcher, whose width and focused screen it stands on, or null in a shell without
    // one, where it stands on the first screen at the launcher's own width.
    property var launcher: null
    // The notices on screen, the front first, as Stack.js keeps them (Notifications.qml).
    property var shown: []
    // Whether the talk line is up in the same place.
    property bool talking: false
    // Whether the held list has the notices and the keys, until it goes with the stack.
    property bool holding: false
    // What its signing line says on the right: how many more wait, given their number, and
    // then when the notice came.
    property var countOf: more => ""
    property var whenOf: notification => ""

    // The notice it shows, by id, or -1, and whether it's up.
    readonly property int chosen: Line.chosen(shown, Notices.made, Quiet.on)
    readonly property var notice: chosen < 0 ? null : Notices.live[chosen] ?? null
    readonly property bool critical: Notices.made[chosen]?.urgency === "critical"
    readonly property bool up: notice !== null
        && Line.up(chosen, Notices.gathered || holding, talking, critical)
    readonly property int more: Line.more(shown, Notices.made, chosen)

    // A click on it, which Notifications.qml answers as a click on the front card did.
    signal clicked(var notification)
    // A link in its words was clicked.
    signal linked(string href)

    onUpChanged: Report.say(`${up ? "lines" : "unlines"} notice ${notice?.summary ?? ""}`)

    // Where its card is drawn, as a fraction of its screen across and down, and what niri
    // blurs beside what's drawn, in the surface's points, for Notifications.qml's `front` to
    // give a test; null while it's down.
    function where() {
        const screen = surface.screen;
        if (!up || !screen)
            return null;
        // Quickshell's type files can't place margins, so qmllint can't.
        const left = surface.margins.left + card.x; // qmllint disable unqualified
        // The same holds for the top margin.
        const top = surface.margins.top + card.y; // qmllint disable unqualified
        // Each a frame in the surface's points, on the whole points it covers.
        const framed = one => {
            const at = one.mapToItem(null, 0, 0);
            const x = Math.floor(at.x);
            const y = Math.floor(at.y);
            return [x, y, Math.ceil(at.x + one.width) - x, Math.ceil(at.y + one.height) - y];
        };
        return {
            left: left / screen.width,
            top: top / screen.height,
            right: (left + card.width) / screen.width,
            bottom: (top + card.height) / screen.height,
            foot: (top + card.height) / screen.height,
            blurred: [framed(card.area.item)],
            drawn: [framed(card)]
        };
    }

    // From the daemon saying a notice came to niri presenting the line with it, held to Joel's
    // 50 ms (2026-11-12: "time from trigger to visible on the screen"), said once for each
    // notice the line shows as it comes; null once said, or once it's plain the line won't
    // show it.
    property var timing: null
    Connections {
        target: Notices

        function onCame(notification, notice) {
            root.heard(notification.id);
        }
    }
    function heard(id) {
        // Read once as it comes, in a function rather than a binding, so there's nothing for
        // a notify to update.
        const after = surface.FramePresentation.framesBegun; // qmllint disable stale-property-read
        timing = { heard: KeyTimes.now(), id, after };
        surface.FramePresentation.ask();
    }
    function presented(frame, at) {
        if (timing === null)
            return;
        if (frame <= timing.after) {
            surface.FramePresentation.ask();
            return;
        }
        const took = Math.round(at - timing.heard);
        if (up && chosen === timing.id)
            console.info(`notice line: up ${took} ms after the daemon said it`);
        timing = null;
    }

    PanelWindow {
        id: surface

        Counted { name: "notice-line" }
        visible: root.up
        // Mapped as it comes up, its first frame is the one the timing waits for.
        onVisibleChanged: if (visible && root.timing !== null) surface.FramePresentation.ask()
        screen: root.launcher?.focusedScreen ?? Quickshell.screens[0] ?? null
        anchors.top: true
        anchors.left: true
        // Quickshell's type files can't place margins, so qmllint can't.
        margins.top: Theme.space.rung.screen - card.reach // qmllint disable unqualified
        // The same holds for the left margin.
        margins.left: !surface.screen ? 0 // qmllint disable unqualified
            : Math.round((surface.screen.width - card.width) / 2) - card.reach
        implicitWidth: card.width + 2 * card.reach
        implicitHeight: card.height + 2 * card.reach
        exclusionMode: ExclusionMode.Ignore
        color: "transparent"
        mask: card.area
        BackgroundEffect.blurRegion: card.area
        WlrLayershell.layer: WlrLayer.Overlay
        // Never the keys, so a notice coming can't take a word Joel's typing.
        WlrLayershell.keyboardFocus: WlrKeyboardFocus.None
        WlrLayershell.namespace: "notice-line"
        FramePresentation.onPresented: (frame, at) => root.presented(frame, at)
        FramePresentation.onDiscarded: frame => {
            if (root.timing !== null)
                surface.FramePresentation.ask();
        }

        Card {
            id: card
            x: reach
            y: reach
            width: root.launcher?.width ?? 680
            urgent: root.critical

            whole: MouseArea {
                anchors.fill: parent
                onClicked: {
                    if (root.notice !== null)
                        root.clicked(root.notice);
                }
            }

            // The notice as its card's row (NoticeRow.qml), so it reads as it did in the
            // corner: who or what, what it says, and its sender's signature with, on the
            // right, how many more wait and when it came.
            NoticeRow {
                readonly property var sender: root.notice?.from
                    ?? { glyph: "", hue: "text", name: "" }

                width: card.contentWidth
                mark: sender.glyph
                hue: Theme[sender.hue] ?? Theme.text
                picture: Notices.picture(root.notice)
                quiet: root.notice?.urgency === NotificationUrgency.Low
                title: root.notice?.summary ?? ""
                words: root.notice?.words?.styled ?? ""
                onLinked: href => root.linked(href)
                from: sender.name
                count: root.notice === null ? "" : root.countOf(root.more)
                when: root.notice === null ? "" : root.whenOf(root.notice)
            }
        }
    }
}
