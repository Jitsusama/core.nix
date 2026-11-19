// The arrival line: the one notice that still waits on Joel, or what's playing for its moment,
// as one frame a step inside the top of the focused screen, centred on the launcher's line
// where Super+S's line and the launcher stand, so a notice comes where everything else of the
// shell does rather than in a window of its own to the side (Joel, 2026-11-13: "the
// notification thing should just be part of the launcher ... just make it all one system").
// It takes no keys and moves no focus: Super+N opens the launcher on it, and a click opens
// what it's about. NoticeLine.js chooses what it shows, steps it aside while the launcher
// shows the notices or the talk line is up, and counts the rest that wait.
//
// Its ink is the line's (VKCF3Z, 2026-11-13, attachments/line-colour-and-space.md): the
// sender's mark in its own hue, the accent nowhere since no keys come here, and colour on
// glyphs, never on text.
//
// Its layer stays mapped while it's quiet, as the launcher's and the talk line's do: off the
// screen but a clear pixel, taking no pointer, and drawn only as what it shows changes, so a
// quiet desktop draws nothing for it. A notice coming then draws on a layer niri already
// has, rather than waiting on a new one's first configure, which took most of the line's
// 117 to 329 ms in the desktop test (Y2YBTS, 2026-11-14). It's the notices' layer, named as
// the corner's was, so niri keeps it out of screen shares and its own screenshots
// (modules/niri's config.kdl). A notice stays for its time (NoticeLine.js's lasts), counted
// only while the line shows it and Joel's here, and then rests: off the screen but still
// open, where the launcher finds it. A critical one stays until it's dealt with. Only Joel's
// own click acts on it.
//
// What's playing (Music.qml) shows here too, for a moment when a track changes or a media key
// is pressed, as a notice's row: the cover beside the track and who plays it, and the player
// and where in the track. It's no message, so it isn't kept; a click lets it go.
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
    // Whether the talk line is up in the same place.
    property bool talking: false
    // Whether Joel's away, when a notice's time waits, so one that comes while he's gone is
    // there when he's back, counting again in full.
    property bool away: false
    // Whether the surface is let go for a turn to be made again, as a screen comes or goes.
    property bool remaking: false
    // What its signing line says on the right: how many more wait, given their number, and
    // then when the notice came.
    property var countOf: more => ""
    property var whenOf: notification => ""

    // The notice it shows, by id, or -1, and whether it's up.
    readonly property int chosen: Line.chosen(Notices.front, Quiet.on)
    readonly property var notice: chosen < 0 ? null : Notices.live[chosen] ?? null
    readonly property bool critical: Notices.made[chosen]?.urgency === "critical"
    readonly property string shows: Line.shows({
        noticeUp: notice !== null && Line.up(chosen, Notices.gathered, talking, critical),
        critical,
        playing: Music.arrival !== null,
        gathered: Notices.gathered,
        talking
    })
    readonly property bool up: shows === "notice"
    readonly property bool playing: shows === "playing"
    // Whether it shows anything, a notice or what's playing, and so stands on the screen.
    readonly property bool shown: shows !== ""
    readonly property int more: Line.more(Notices.front, chosen)
    // What's playing, or nothing while there's no player to say it of.
    readonly property var song: Music.card ?? {
        title: "", artist: "", art: "", from: "", mark: "", where: ""
    }

    // A click on the notice it shows.
    signal clicked(var notification)
    // A link in its words was clicked.
    signal linked(string href)
    // The notice it showed had its time, for Notifications.qml to rest it.
    signal rested(var notification)

    onUpChanged: {
        Report.say(`${up ? "lines" : "unlines"} notice ${notice?.summary ?? ""}`);
        Qt.callLater(root.count);
    }
    onCriticalChanged: Qt.callLater(root.count)
    onAwayChanged: Qt.callLater(root.count)
    // Come onto the screen from quiet, its first frame there is the one the timing waits for.
    onShownChanged: {
        if (!shown)
            return;
        shownAt = KeyTimes.now();
        if (timing !== null)
            surface.FramePresentation.ask();
    }
    // Another notice in front starts its own time in full.
    onChosenChanged: {
        Qt.callLater(root.count);
        reported.changed();
    }

    // Starts the time of the notice the line shows afresh, while it can go and Joel's here to
    // read it, or stops it. Said once what the line shows has settled, so the time is the new
    // notice's. Started by hand rather than bound: a timer stops itself once it fires, and a
    // binding that still reads true would never start it again for the next notice in front.
    function count() {
        resting.stop();
        if (!up || critical || away)
            return;
        resting.interval = Line.lasts(notice?.expireTimeout ?? 0, notice?.actions?.length ?? 0);
        resting.start();
    }

    Timer {
        id: resting
        onTriggered: root.rested(root.notice)
    }

    Connections {
        target: Music

        function onCardChanged() {
            reported.changed();
        }
    }

    // Where its card is drawn, as a fraction of its screen across and down, and what niri
    // blurs beside what's drawn, in the surface's points, for Notifications.qml's `front` to
    // give a test; null while it shows no notice.
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
    // show it. It's said apart, since a median of 86 ms and a p90 of 621 ms over the 30
    // notices Joel had in the day to 2026-11-14 didn't say which to make quicker: whether
    // it came to a quiet line or one up already, then until the first frame after began its
    // sync, then that frame's sync, render and swap, then until niri showed it.
    property var timing: null
    // When the line last came up from quiet, which happens as the front changes, just before
    // the notice it names is said to have come.
    property real shownAt: 0
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
    // The line's time said apart by the render thread's own times of its frame
    // (frame-presentation.patch's timesOf), or "" where the frame kept none.
    function split(frame, at) {
        const times = surface.FramePresentation.timesOf(frame);
        if (!times.synchronizing || !times.swapped)
            return "";
        const since = (from, to) => Math.round(to - from);
        // Up in the same moment it came, as a quiet line is, or up already.
        const was = shownAt > timing.heard - 50 ? "quiet till then" : "up already";
        return `: ${was}, ${since(timing.heard, times.synchronizing)} until its frame began, `
            + `${since(times.synchronizing, times.swapped)} drawing it and `
            + `${since(times.swapped, at)} until niri showed it`;
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
            console.info(`notice line: up ${took} ms after the daemon said it${split(frame, at)}`);
        timing = null;
    }

    PanelWindow {
        id: surface

        Counted { name: "notices" }
        // Unmade with no real screen to stand on: Qt's stand-in for one has no name and holds
        // no surface, so made there, niri refuses it and Quickshell warns, as it did on the
        // net's vm-screens-keep-the-launcher each time every screen went (2026-11-14 18:22 on).
        visible: !root.remaking && surface.standsOn !== null
        // Made again for another screen, its first frame is the one the timing waits for.
        onVisibleChanged: if (visible && root.timing !== null) surface.FramePresentation.ask()
        readonly property var standsOn: root.launcher !== null ? root.launcher.focusedScreen
            : Quickshell.screens.find(one => one.name !== "") ?? null
        screen: surface.standsOn
        anchors.top: true
        anchors.left: true
        // Off the screen while quiet, but its last clear pixel in the corner the launcher's
        // and the talk line's already keep theirs in.
        readonly property bool placed: root.shown && surface.screen !== null
        readonly property real top: surface.placed ? Theme.space.rung.screen - card.reach
            : 1 - surface.implicitHeight
        readonly property real left: surface.placed
            ? Math.round((surface.screen.width - card.width) / 2) - card.reach
            : 1 - surface.implicitWidth
        // Quickshell's type files can't place margins, so qmllint can't.
        margins.top: surface.top // qmllint disable unqualified
        // The same holds for the left margin.
        margins.left: surface.left // qmllint disable unqualified
        implicitWidth: card.width + 2 * card.reach
        implicitHeight: card.height + 2 * card.reach
        exclusionMode: ExclusionMode.Ignore
        color: "transparent"
        // A card of upright clips needs no depth or stencil buffer, which
        // would be as large as the window and held while it's quiet too,
        // since it stays mapped (window-drawn-flat.patch). A turned mark
        // turns inside the card's clip, which stays upright, and a cover's
        // corners are a layer drawn into a target of its own; only a clip
        // on a turned item would need a stencil
        // (flat-windows-hold-no-depth-on-niri).
        surfaceFormat.flat: true
        // Nothing to click or blur while quiet.
        mask: root.shown ? card.area : nowhere
        BackgroundEffect.blurRegion: root.shown ? card.area : nowhere
        WlrLayershell.layer: WlrLayer.Overlay
        // Never the keys, so a notice coming can't take a word Joel's typing.
        WlrLayershell.keyboardFocus: WlrKeyboardFocus.None
        WlrLayershell.namespace: "notifications"
        FramePresentation.onPresented: (frame, at) => root.presented(frame, at)
        FramePresentation.onDiscarded: frame => {
            if (root.timing !== null)
                surface.FramePresentation.ask();
        }

        Region {
            id: nowhere
        }

        // Said as what it shows changes while it stays up, as the corner's stack said it.
        Reported {
            id: reported
            name: "notifications"
            showing: root.shown
        }

        Card {
            id: card
            x: reach
            y: reach
            width: root.launcher?.width ?? 680
            urgent: root.up && root.critical
            onHeightChanged: reported.changed()

            whole: MouseArea {
                anchors.fill: parent
                onClicked: {
                    if (root.playing)
                        Music.dismiss();
                    else if (root.notice !== null)
                        root.clicked(root.notice);
                }
            }

            // The notice, or what's playing, as a notice's row (NoticeRow.qml), so it reads
            // as it did in the corner: who or what, what it says, and its sender's signature
            // with, on the right, how many more wait and when it came, or where in the track.
            NoticeRow {
                readonly property var sender: root.notice?.from
                    ?? { glyph: "", hue: "text", name: "" }

                width: card.contentWidth
                mark: root.playing ? root.song.mark : sender.glyph
                hue: root.playing ? Theme.program : Theme[sender.hue] ?? Theme.text
                picture: root.playing ? root.song.art : Notices.picture(root.notice)
                quiet: !root.playing && root.notice?.urgency === NotificationUrgency.Low
                title: root.playing ? root.song.title : root.notice?.summary ?? ""
                words: root.playing ? Line.escaped(root.song.artist)
                    : root.notice?.words?.styled ?? ""
                onLinked: href => root.linked(href)
                from: root.playing ? root.song.from : sender.name
                // Only while it's up, so a time ticking on draws nothing off the screen.
                count: !root.up ? "" : root.countOf(root.more)
                when: root.playing ? root.song.where : !root.up ? "" : root.whenOf(root.notice)
            }
        }
    }
}
