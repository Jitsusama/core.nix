// The notices, as the notices daemon serves them (Notices.qml): what each one does as it
// comes, rests and is chosen, and the shell's asks for them. The one that still waits on Joel
// shows on the arrival line (NoticeLine.qml), at the top centre where the launcher stands, a
// critical one before the rest; and Super+N opens them in the launcher, Now's Desktop
// those unread and History what's read, where the launcher's keys, acts and drawer do what
// can be done with each (PLAN-20261113-821NSZ). They're one system: there's no other place a
// notice shows (Joel, 2026-11-13: "just make it all one system"). While notices are held
// (Quiet.qml), only a critical one shows.
//
// A click on the line opens what its notice is about, or puts it away when it's about
// nothing. Its time up, a notice rests: off the screen but still open, so its program still
// answers its actions from the launcher; one its sender asked not to be kept closes.
//
// Each is kept in the history as it arrives, by the daemon, for the launcher to find later,
// where the ones still here can be answered or forgotten.
pragma ComponentBehavior: Bound
import Quickshell
import Quickshell.Io
import Quickshell.Services.Notifications
import Quickshell.Wayland
import QtQuick
import "When.js" as When

Scope {
    id: root

    // The notices open, as Notices.qml hands them, in the order they arrived: by id, which
    // the daemon never gives twice, as an object's whole-number keys are listed in rising
    // order.
    readonly property var tracked: Object.values(Notices.live)

    // When each arrived, by id, for the signing line's time and which rests longest, as the
    // daemon's records say it. Quickshell's JavaScript has no Object.fromEntries.
    readonly property var arrived: {
        const at = {};
        for (const one of tracked)
            at[one.id] = Notices.made[one.id]?.at ?? 0;
        return at;
    }

    function arrivedAt(id) {
        return root.arrived[id] ?? 0;
    }

    // The launcher, which the notices open in.
    property var launcher: null
    // How many may rest at once before the oldest is closed, so no program is asked to hold
    // its notices for ever.
    readonly property int rests: 30
    // Whether Joel's away: nothing typed or moved for a while, whatever keeps the screen
    // awake, since a call or Keep Awake does that while he's gone too. A notice's time waits
    // while he's away.
    readonly property bool away: idle.isIdle
    onAwayChanged: Report.say(away ? "away" : "back")

    IdleMonitor {
        id: idle
        timeout: Options.notices.away
        respectInhibitors: false
    }

    // A link in what a notice said opened in the program the desktop opens it with, through
    // niri, as every program is (the daemon's body.rs lets only the web's and mail's through).
    function follow(href) {
        Niri.spawn(["xdg-open", href]);
    }

    // What a notice's sender does when it's chosen, or null.
    function opens(notification) {
        return notification.actions.find(action => action.identifier === "default") ?? null;
    }

    // Every notice on screen away at once, as a click on each would, so each stays unread
    // where Super+N finds it: those open but not resting.
    function putAway() {
        for (const notification of tracked.filter(one => !(one.id in Notices.resting)))
            notification.dismiss();
    }

    // One whose time is up leaves the screen but stays open, resting, so its program still
    // answers its actions from the launcher; one its sender asked not to be kept closes. Past
    // `rests` of them, the oldest closes.
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

    // The notices in the launcher: those on screen while any wait on the arrival line, in
    // normal, so j and k go through them at once and Return acts on one, the cursor on the
    // one the line shows, as the daemon chose it (Joel, 2026-11-14: "super-n no longer lets
    // me interact with the one in focus"); with none, or under Do Not Disturb, For You's
    // Desktop, those unread and then History (launcher/Now.qml's leadsTo).
    function history() {
        const front = line.chosen;
        const onScreen = !Quiet.on && (Notices.front.order ?? []).length > 0;
        if (launcher)
            launcher.intoOn(onScreen ? "onScreen" : "notices",
                row => front >= 0 && row.notice === front, onScreen);
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

    IpcHandler {
        target: "notices"

        // Super+Shift+N puts every notice on screen away, as a click on each would, so each
        // stays unread where Super+N finds it.
        function putAway(): void {
            root.putAway();
        }

        // Super+N: the notices in the launcher, on the one in front; pressed again while the
        // launcher shows them, it closes.
        function take(): void {
            if (root.launcher?.open && Notices.gathered)
                root.launcher.toggled();
            else
                root.history();
        }

        // Every notice missed counted as seen, as the margin's Let Them Go does.
        function seen(): void {
            Notices.seen();
        }

        // What the arrival line shows, "notice" or "playing", or nothing while it's quiet, for
        // a test to wait on: its layer stays mapped either way (NoticeLine.qml), so niri's list
        // of layers can't say.
        function shows(): string {
            return line.shows;
        }

        // Where the line's notice is drawn, as a fraction of the screen across and down, for
        // a test to click on it, since no shot says where a card without a coloured mark is,
        // and what niri blurs beside what's drawn; what it's signed, the glyph it wears and
        // that glyph's hue; whether a picture of its own stands in its mark column; and
        // where its links go. Nothing while the line shows no notice.
        function front(): string {
            const lined = line.where();
            if (lined === null)
                return "{}";
            const from = line.notice.from;
            return JSON.stringify(Object.assign({
                signed: from.name,
                glyph: from.glyph,
                hue: from.hue,
                pictured: Notices.picture(line.notice) !== "",
                links: line.notice.words.links.map(link => link.href)
            }, lined));
        }
    }

    // A keyboard's Do Not Disturb key holds notices until it's pressed again, as a Mac's
    // does, or lets them show if they're held.
    IpcHandler {
        target: "quiet"

        function toggle(): void {
            if (Quiet.on)
                Quiet.end();
            else
                Quiet.keep(0);
        }
    }

    // One arriving is said to show, unless the daemon held it, as one is while notices are
    // (Quiet.qml) or when its sender's level never shows it; then it rests from the start,
    // never shown but keeping its actions. Which one the line shows is the daemon's front.
    Connections {
        target: Notices

        function onCame(notification, notice) {
            Report.say(`${notice.held ? "held" : "shows card"} ${notification.summary}`);
        }
    }

    // A screen coming or going (a dock, the lid, a monitor) can leave the line's surface
    // nowhere: with no screen left, Qt moves the window onto a stand-in that can hold no
    // surface, then onto the screen that comes back by itself, so Quickshell never makes the
    // surface again (Launcher.qml says more). A critical notice never goes by itself, so
    // nothing would bring it back. So once the screens change, a surface that's up is let go
    // for a turn and made again.
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

    // One said again in place that tells something new comes back on screen, as one arriving
    // would, unless notices are held and it isn't urgent, when it keeps resting. Arrivals
    // asks for it through Notices, and the daemon says it was raised.
    Connections {
        target: Notices

        function onRaisedChanged() {
            const id = Notices.raised?.id;
            const notification = root.tracked.find(one => one.id === id);
            if (!notification
                || Quiet.on && notification.urgency !== NotificationUrgency.Critical)
                return;
            Notices.wake(id);
            Report.say(`raises ${notification.summary}`);
        }
    }

    // A click on the line's notice: it opens what it's about, or puts it away when it's
    // about nothing to open.
    function choose(notification) {
        const opens = root.opens(notification);
        if (opens) {
            Notices.mark(notification.id, true);
            Notices.open(opens, notification.appName);
        } else {
            notification.dismiss();
        }
    }

    // What a notice's signing line says on the right after how many more wait: when it came,
    // or, for one saying how it stands, that and its time, "sent · 12:40" (VKCF3Z,
    // 2026-11-12).
    function whenOf(notification) {
        const now = clock.date.getTime();
        return notification.said !== ""
            ? `${notification.said} \u00b7 ${When.clock(notification.at)}`
            : root.since(root.arrivedAt(notification.id) || now, now);
    }

    // How many more wait behind the one in front, as its signing line counts them, or
    // nothing when none do.
    function countOf(more) {
        return more > 0 ? `${more} more` : "";
    }

    NoticeLine {
        id: line

        launcher: root.launcher
        talking: root.launcher?.talking ?? false
        away: root.away
        remaking: root.remaking
        whenOf: notification => root.whenOf(notification)
        countOf: more => root.countOf(more)
        onClicked: notification => root.choose(notification)
        onLinked: href => root.follow(href)
        onRested: notification => root.rest(notification)
    }
}
