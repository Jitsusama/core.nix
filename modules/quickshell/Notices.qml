pragma Singleton
// Every notification, newest first, for the launcher's collection of them
// (launcher/PastNotices.qml), kept on disk in the shell's state so a restart
// loses none: an unread one until it's read, and the rest for good, the
// oldest read going once more than `keep` are kept (launcher/Lately.js's
// kept).
//
// One leaves the screen in one of three ways: clicked away; withdrawn by its
// program, as a chat app does once the message is read there; or by itself
// when its time is up. One that left by itself was missed, and what's missed
// is counted until the collection is opened. One that arrives while notices
// are held (Quiet.qml) is never shown, so it's missed from the start.
//
// One that left by itself, or was never shown, rests: it's off the screen
// but still open, so its program keeps its actions (Reply, Open) for the
// launcher's keys, until the program withdraws it or it's let go. Closing
// it would tell the program it's done with, and the program would forget
// what it offered.
//
// Apart from that, each is unread until it's acted on: gone to, one of its
// actions done, marked read, or withdrawn by its program. A click only puts
// it away, so one clicked by accident still waits, unread, in the list.
//
// The notices on screen and the collection are one home: while the
// collection is open the screen's step aside (`gathered`), still there to
// be answered from the launcher, and come back if it closes before they go.
import Quickshell
import Quickshell.Io
import QtQuick
import "Forward.js" as Forward
import "NoticeBody.js" as NoticeBody
import "ProgramGlyphs.js" as ProgramGlyphs
import "launcher/Lately.js" as Lately

Singleton {
    id: root

    // How many are kept: far past what Joel would scroll, so History keeps
    // what's read for good, and its file stays small, about 300 KB (VKCF3Z,
    // 2026-11-09). One unread stays until it's read or forgotten.
    readonly property int keep: 1000

    // { id, at, app, entry, summary, body, styled, links, picture, glyph,
    // hue, urgency, category, low, urgent, held, missed, read, waits, left,
    // leftAt }: `body` is its words plain and `styled` with the markup a
    // card draws, `links` each { text, href }, `urgency` what its sender
    // asked (low, normal or critical) and `urgent` whether it stays, and
    // `left` how it left the screen once it has (expired, dismissed or
    // withdrawn), and when.
    property var history: []
    readonly property int missed: history.filter(notice => notice.missed).length
    readonly property int unread: history.filter(notice => !notice.read).length
    // Those missed and gone from the screen, which wait for Joel.
    readonly property var waiting: history.filter(notice => notice.missed
        && (!(notice.id in live) || notice.id in resting))

    // The notifications still open, by id, on screen or resting, whose own
    // actions (Reply, Mark as read) can be done from the launcher's keys
    // while they last.
    property var live: ({})
    // Those of them resting off the screen, by id.
    property var resting: ({})
    // Whether the launcher's collection of them is open.
    property bool gathered: false

    // A sender's glyph, the hue of what it brings, a person's message
    // wearing the person's, and the name Joel knows it by, which a card
    // signs it with, never its program's. A program's notice wears its
    // program's own mark, the one its row in the launcher does
    // (ProgramGlyphs.js), named by its desktop entry's id as `program`;
    // the rest are the desktop's own, each its theme mark. One that only
    // answers what Joel just did, as the recorder's saying where a file
    // went, is never missed.
    readonly property var senders: ({
        "slack": { program: "slack", hue: "person", name: "Slack" },
        "tuple": { glyph: Theme.glyph.call.codePointAt(0), hue: "person", name: "Tuple" },
        "google chrome": { program: "google-chrome", hue: "program", name: "Google Chrome" },
        "chromium": { program: "chromium", hue: "program", name: "Chromium" },
        // The compositor, saying a screenshot was taken or its settings
        // didn't load.
        "niri": { glyph: 0xf0e51, hue: "machine", name: "Desktop" },
        // notify-send is the tool a notice came through, not who sent it, so
        // a notice it names no sender for is the desktop's, on its card and
        // its plate alike (VKCF3Z, 2026-11-03).
        "notify-send": { glyph: 0xf0e51, hue: "machine", name: "Desktop" },
        // A pi session wanting Joel (Sessions.qml), in its mark. It's
        // never missed, since the margin's mark says it while it holds.
        "pi": {
            glyph: Theme.glyph.agent.codePointAt(0), hue: "program", answers: true,
            name: Words.name("agents")
        },
        // Something an agent has ready for Joel to do (WaitingOnYou.qml),
        // sent by For You, so its margin mark is For You's own tray, the
        // mark its row in Now and its head wear (VKCF3Z, 2026-11-08).
        "waiting": {
            glyph: Theme.glyph.forYou.codePointAt(0), hue: "program",
            name: Words.name("on:waiting")
        },
        "recorder": {
            glyph: Theme.glyph.recording.codePointAt(0), hue: "machine", answers: true,
            name: "Capture"
        },
        // What arrives by itself (Arrivals.qml), in its margin mark, which
        // says it while it holds, so it's never missed either.
        "camera": {
            glyph: Theme.glyph.camera.codePointAt(0), hue: "machine", answers: true,
            name: "Camera"
        },
        // A call being written down, or written (call-transcripts'
        // notices.rs), a call's as Tuple's are, since each is about a
        // call with others. Each says itself whether it answers what Joel
        // is doing, so it's never missed.
        "calls": {
            glyph: Words.glyph("notices:calls").codePointAt(0), hue: "person",
            name: Words.name("notices:calls")
        },
        // A meeting about to start (Starting.qml), with others in it.
        "calendar": {
            glyph: Theme.glyph.meeting.codePointAt(0), hue: "person", answers: true,
            name: "Calendar"
        },
        // A timer ringing (timers/src/main.rs), in a timer's mark, the
        // desktop's as niri's are, missed until seen, since it rang for
        // Joel whether or not he was there.
        "timers": {
            glyph: Theme.glyph.timer.codePointAt(0), hue: "machine",
            name: Words.name("timers")
        },
        "battery": {
            glyph: Theme.glyph.low.codePointAt(0), hue: "machine", answers: true,
            name: Words.name("power")
        },
        // The AI tokens running out (pieces/Tokens.qml), the agents' own.
        "tokens": {
            glyph: Theme.glyph.tokens.codePointAt(0), hue: "program", answers: true,
            name: Words.name("spend")
        },
        // A firmware update fwupd offers (modules/firmware-notice), said
        // once a release, and missed until seen, since nothing else says it.
        "firmware": {
            glyph: Theme.glyph.firmware.codePointAt(0), hue: "machine",
            name: Words.name("firmware")
        },
        // What became of a save into Google Drive (modules/google's
        // Saves.qml): one that failed is missed until seen, since the file
        // waits on Joel, and one that went up answers what he did.
        "google drive": { program: "google-drive", hue: "program", name: "Google Drive" }
    })

    // The bell, worn only by a sender no program's mark is known for.
    readonly property int bell: 0xf0f3
    // A machine's own program marks, as the launcher's Programs are given
    // them (jitsusama.quickshell.launcher.providers.applications.glyphs).
    readonly property var ownGlyphs: Providers.providers
        .find(listed => listed.name === "applications")?.glyphs ?? {}

    // Who sent a notice, by the name it gave and the desktop entry it names,
    // if any: one the desktop knows by name as it's listed; any other is
    // signed by its program's own name for itself, as its launcher row is,
    // or else as it named itself, and wears its program's mark, the bell
    // only where no program's is known.
    function sender(name, entry) {
        const known = senders[name.toLowerCase()];
        if (known && known.glyph !== undefined)
            return known;
        const program = known?.program
            ?? ((entry ?? "") !== "" ? DesktopEntries.byId(entry) : null)
            ?? DesktopEntries.heuristicLookup(name);
        const id = typeof program === "string" ? program : program?.id ?? name;
        const mark = ProgramGlyphs.glyph(id, ownGlyphs, Theme.glyph);
        return {
            glyph: mark === "" ? bell : mark.codePointAt(0),
            hue: known?.hue ?? "program",
            name: known?.name ?? (typeof program === "string" ? null : program?.name) ?? name
        };
    }

    // A notice's own picture, as an avatar or a thumbnail: the image it was
    // sent with, never its program's icon, which its mark says. One given as
    // an icon's name, or the same as its program's icon, is an icon, so its
    // mark stays (VKCF3Z, 2026-11-07). Quickshell serves one sent as data
    // only while the notice is open, so the history keeps a picture only
    // when it's a file.
    // Quickshell hands an image-path hint that isn't a file: URL to its
    // icon provider, so a plain path arrives as image://icon/ and the path,
    // which is a file, and a name there is an icon's.
    function picture(notification) {
        const image = notification?.image ?? "";
        const iconed = image.startsWith("image://icon/") ? image.slice("image://icon/".length)
            : null;
        if (iconed !== null)
            return iconed.startsWith("/") && iconed !== notification.appIcon
                ? `file://${iconed}` : "";
        if (image === "" || image === notification.appIcon)
            return "";
        return image.startsWith("/") ? `file://${image}` : image;
    }

    // A notice's picture as the history has it: its own while it's open,
    // else the file it was kept as, if any.
    function pictureOf(notice) {
        const still = live[notice.id] ?? null;
        return still ? picture(still) : notice.picture ?? "";
    }

    // Those kept: every one unread, and at most `keep` in all, the oldest
    // read going first (Lately.js's kept).
    function fresh(notices) {
        return Lately.kept(notices, keep);
    }

    // Keeps a notification as it arrives, missed until it's seen unless
    // it's never counted, and held when it's never shown. One sent at low
    // urgency is never missed, since its sender says it can pass unseen.
    function arrived(notification, urgent, held, counted, urgency) {
        const low = urgency === "low";
        const from = sender(notification.appName, notification.desktopEntry);
        const said = NoticeBody.read(notification.body);
        const pictured = picture(notification);
        const notice = {
            id: notification.id,
            at: Date.now(),
            app: notification.appName,
            entry: notification.desktopEntry ?? "",
            summary: notification.summary,
            // Its words plain for the launcher's rows, the find and a copy,
            // and styled for a card and a drawer, with its links.
            body: said.plain,
            styled: said.styled,
            links: said.links,
            picture: pictured.startsWith("file://") ? pictured : "",
            glyph: from.glyph,
            hue: from.hue,
            urgency: urgency,
            // What kind of notice its sender says it is, in the spec's
            // words, as im.received.
            category: String(notification.hints?.category ?? ""),
            low: low,
            urgent: urgent,
            held: held,
            // A sender whose every notice is missed can still send one that
            // only answers what Joel just did, saying so with the x-answers
            // hint, as a save Drive took does.
            missed: counted && !low && !from.answers
                && notification.hints?.["x-answers"] === undefined,
            read: false,
            // The to-do it says, if it's one (WaitingOnYou.qml), so
            // it's settled with the item even once gone from the screen.
            waits: String((notification.hints ?? {})["x-waiting"] ?? "")
        };
        // Live before it's kept, so it never reads as missed and gone from
        // the screen for the moment between, which would flash the margin.
        // One held rests from the start; one its program sends again with
        // the same id comes back to the screen.
        live = Object.assign({}, live, { [notice.id]: notification });
        const still = Object.assign({}, resting);
        if (held)
            still[notice.id] = true;
        else
            delete still[notice.id];
        resting = still;
        history = fresh([notice].concat(history.filter(kept => kept.id !== notice.id)));
    }

    // One said again in place, as an arrival is while its words follow the
    // minutes (Arrivals.qml), keeps its place and whether it's read, with
    // its new words.
    function reword(id, summary, body) {
        const said = NoticeBody.read(body);
        history = history.map(notice => notice.id !== id ? notice
            : Object.assign({}, notice, {
                summary: summary, body: said.plain, styled: said.styled, links: said.links
            }));
    }

    // The last one said again in place that tells something new, so the
    // stack (Notifications.qml) brings it back to the front: { id, at }.
    property var raised: null

    // One said again in place that tells something new, as an arrival is
    // as it steps down (Arrivals.qml), comes back to the screen unread,
    // the same card rather than another.
    function raise(id) {
        history = history.map(notice => notice.id !== id ? notice
            : Object.assign({}, notice, { read: false }));
        raised = { id: id, at: Date.now() };
    }

    // One resting comes back to the screen.
    function wake(id) {
        if (id in resting) {
            const others = Object.assign({}, resting);
            delete others[id];
            resting = others;
        }
    }

    // One whose time on screen is up rests off it, still open.
    function rest(id) {
        if (id in live)
            resting = Object.assign({}, resting, { [id]: true });
    }

    // How one left, as "expired", "dismissed" or "withdrawn": only one that
    // went by itself was missed, and one its program withdrew was read there.
    // It's no longer missed before it leaves the screen, for the same reason.
    // How and when each left is kept, for its drawer.
    function left(id, how) {
        const at = Date.now();
        history = history.map(notice => notice.id !== id ? notice
            : Object.assign({}, notice, { left: how, leftAt: at },
                how === "expired" ? {} : { missed: false },
                how === "withdrawn" ? { read: true } : {}));
        const still = Object.assign({}, live);
        delete still[id];
        live = still;
        if (id in resting) {
            const others = Object.assign({}, resting);
            delete others[id];
            resting = others;
        }
    }

    // Opens what a notice is about, by its sender's own action, and brings
    // the sender forward when it doesn't come by itself. The shell hands
    // it a token to raise its window with, but Slack and Chrome often ask
    // niri for one of their own, which niri refuses, so the conversation
    // would open behind what's in front (Forward.js). A second is given
    // for it to come, and then its window is focused, if it has one.
    function open(action, app) {
        action.invoke();
        const entry = DesktopEntries.heuristicLookup(app);
        forwarding = [app.toLowerCase()].concat(entry ? [entry.id.toLowerCase()] : []);
        forwarded.restart();
    }

    // The names the last opened notice's sender goes by, which its window
    // is found by.
    property var forwarding: []

    // The window of the program that sent a notice, as windows are now, or
    // null, found by its name and its desktop entry's, as its card and its
    // row in the launcher both go to it.
    function senderWindow(app) {
        const entry = DesktopEntries.heuristicLookup(app);
        const ids = [app.toLowerCase()].concat(entry ? [entry.id.toLowerCase()] : []);
        return Object.values(Niri.windows)
            .find(w => w.app_id && ids.includes(w.app_id.toLowerCase())) ?? null;
    }

    Timer {
        id: forwarded
        interval: 1000
        onTriggered: {
            const id = Forward.toFocus(Niri.windows, Niri.focused, root.forwarding);
            if (id !== null)
                Niri.act({ FocusWindow: { id: id } });
        }
    }

    // Marks one read, or unread again. The shell marks one `withdrawn`
    // once what it said no longer stands, an arrival gone or a want
    // answered, so History never says Joel read what he never saw.
    function mark(id, read, how) {
        const now = read ? { missed: false, readAt: Date.now() } : { readAt: null };
        history = history.map(notice => notice.id !== id ? notice
            : Object.assign({}, notice, { read: read, withdrawn: read && how === "withdrawn" },
                now));
    }

    // Marks read every notice of `ids` at once, as For You's Desktop row's
    // Mark Them Read does (launcher/Now.qml).
    function markRead(ids) {
        if (history.some(notice => ids.includes(notice.id) && !notice.read))
            history = history.map(notice => !ids.includes(notice.id) ? notice
                : Object.assign({}, notice, { read: true, missed: false,
                    readAt: notice.read ? notice.readAt ?? null : Date.now() }));
    }

    // Marks read every notice of a to-do that no longer stands, so
    // one that came and went unseen isn't missed once its item is done or
    // gone: `standing` holds the items Joel can still do now.
    function settleWaiting(standing) {
        const stale = notice => notice.waits && !standing[notice.waits]
            && (notice.missed || !notice.read);
        if (history.some(stale))
            history = history.map(notice => stale(notice)
                ? Object.assign({}, notice, { read: true, missed: false }) : notice);
    }

    // Everything kept has been seen, though not necessarily read.
    function seen() {
        if (missed > 0)
            history = history.map(notice => Object.assign({}, notice, { missed: false }));
    }

    // The history on disk. What's read back joins anything that arrived
    // first; its ids are made its own, since the server's start again from
    // one each time the shell starts. Nothing is written until it's read, so
    // an early arrival never overwrites what was kept.
    property bool restored: false

    FileView {
        id: kept
        path: Quickshell.statePath("notices.json")
        printErrors: false
        onLoaded: {
            let saved = [];
            try {
                saved = JSON.parse(text());
            } catch (error) {
                console.warn(`Notices: ${path} doesn't read as JSON, so it starts over`);
            }
            root.history = root.fresh(root.history.concat(saved.map((notice, index) =>
                Object.assign({}, notice, { id: `kept:${notice.at}:${index}` }))));
            root.restored = true;
        }
        onLoadFailed: root.restored = true
    }

    onHistoryChanged: {
        if (restored)
            saving.restart();
    }

    // A burst of changes is written once.
    Timer {
        id: saving
        interval: 1000
        onTriggered: kept.setText(JSON.stringify(root.history))
    }
}
