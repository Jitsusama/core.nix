pragma Singleton
// Every notification, newest first, for the launcher's collection of them
// (launcher/PastNotices.qml), as the notices daemon (modules/notices) holds
// them over $XDG_RUNTIME_DIR/daemons/notices.sock: it serves the session
// bus's notifications name, judges each as it arrives, and keeps them on
// disk, so a restart of the shell loses none, and one sent while the shell
// is down is still shown once it's up. Each is one record, read here into
// the history and, while it's open, into a live notice the cards and the
// launcher's acts read as Quickshell's own (LiveNotice.js); every change
// is an ask of the daemon, whose answer is the record changed.
//
// Its words styled and its sender's mark, hue and name come on its record
// as the daemon made them for its card, the same it makes its rows of
// (modules/notices' row.rs, the launcher's PastNotices.qml), so the card
// and the row never differ and neither is worked out here.
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
// The notices on screen and For You's Desktop are one home: while the
// launcher holds it the screen's step aside (`gathered`), still there to
// be answered from the launcher, and come back if it closes before they go.
import Quickshell
import QtQuick
import "Forward.js" as Forward
import "LiveNotice.js" as LiveNotice
import "launcher/ReadyRows.js" as Ready

Singleton {
    id: root

    Daemon {
        id: daemon

        socket: `${Quickshell.env("XDG_RUNTIME_DIR")}/daemons/notices.sock`
        kind: "notice"
        // The watch is every notice the daemon holds, so one held here
        // that it no longer says went while the shell wasn't hearing.
        answersWhole: true
    }

    // Whether the daemon is there to ask.
    readonly property bool answering: daemon.wire !== null

    // { id, at, app, entry, summary, body, urgency, category, low, urgent,
    // held, missed, read, readHow, waits, left, leftAt }: `body` is its
    // words plain, `urgency` what its sender asked (low, normal or
    // critical) and `urgent` whether it stays, `readHow` how it was read where that wasn't here (markRead),
    // and `left` how it left the screen once it has (expired, dismissed or
    // withdrawn), and when. Each is made again only as the daemon says its
    // record changed, so the launcher's rows of the rest are kept.
    property var history: []
    readonly property int missed: history.filter(notice => notice.missed).length
    readonly property int unread: history.filter(notice => !notice.read).length
    // Those missed and gone from the screen, which wait for Joel.
    readonly property var waiting: history.filter(notice => notice.missed
        && (!(notice.id in live) || notice.id in resting))

    // The notifications still open, by id, on screen or resting, whose own
    // actions (Reply, Mark as read) can be done from the launcher's keys
    // while they last, each as LiveNotice.js hands it.
    property var live: ({})
    // Those of them resting off the screen, by id.
    property var resting: ({})
    // Said once a notice arrives, as it's first heard open: not for those
    // open as the shell starts, which the stack (Notifications.qml) puts
    // at the back as it finds them, as Quickshell's kept across a reload.
    signal came(var notification, var notice)
    // Whether the launcher's collection of them is open.
    property bool gathered: false

    // What the daemon made of a notice for its card, on its record: who
    // sent it, the name Joel knows the sender by, never its program's, in
    // its mark, a theme role's or a program's own served as its code
    // point, and the hue of what it brings (modules/notices/senders.json,
    // the machine's program marks); and its words plain and styled, with
    // their links.
    function cardOf(record) {
        const sender = record?.sender ?? {};
        return {
            from: {
                glyph: Ready.glyphOf(sender.glyph ?? "", Theme.glyph),
                hue: sender.hue ?? "program",
                name: sender.name ?? record?.notice?.app ?? ""
            },
            words: {
                plain: record?.body?.plain ?? "",
                styled: record?.body?.styled ?? "",
                links: record?.body?.links ?? []
            }
        };
    }

    // A notice's own picture, as an avatar or a thumbnail: the image it was
    // sent with, never its program's icon, which its mark says, as a file
    // the daemon names, one sent as pixels written to one it keeps
    // (modules/notices' picture.rs), or "" where it has none.
    function picture(notification) {
        return notification?.image ?? "";
    }

    // Each notice as the history has it, by id, made again only as its
    // record changes, so a reader keeping its rows by the notice keeps
    // those of the rest.
    property var made: ({})
    // The ids heard open, so one first heard open is known to have come.
    property var heardOpen: ({})

    // A record as the history has it, its words plain, as the shell's own
    // readers of it find and withdraw notices by what they said; its rows
    // and their drawers are the daemon's.
    function madeOf(notice, card) {
        return {
            id: notice.id, at: notice.at, app: notice.app, entry: notice.entry,
            summary: notice.summary, body: card.words.plain,
            urgency: notice.urgency, category: notice.category, low: notice.low,
            urgent: notice.urgent, held: notice.held, missed: notice.missed,
            read: notice.read, readHow: notice.readHow, readAt: notice.readAt,
            withdrawn: notice.withdrawn, waits: notice.waits, transient: notice.transient,
            left: notice.left ?? undefined, leftAt: notice.leftAt ?? undefined,
            raisedAt: notice.raisedAt
        };
    }

    // Takes what the daemon said changed and let go: each changed notice
    // made again, its live notice too while it's open, and the history
    // put in order once, newest first. One first heard open after the
    // daemon's first whole answer came while the shell was hearing.
    function took(changed, gone) {
        const arriving = daemon.answered;
        const nowLive = Object.assign({}, live);
        const nowResting = Object.assign({}, resting);
        const arrivals = [];
        for (const key of gone) {
            const id = Number(key.slice("notice:".length));
            delete made[id];
            delete nowLive[id];
            delete nowResting[id];
            delete heardOpen[id];
        }
        for (const key of changed) {
            const notice = daemon.held[key]?.notice;
            if (!notice)
                continue;
            const id = notice.id;
            const was = made[id];
            // Kept as it was while what the history holds of it is the
            // same, so a change it doesn't hold, such as its card resting,
            // remakes no reader's row of it.
            const card = cardOf(daemon.held[key]);
            const now = madeOf(notice, card);
            made[id] = was && JSON.stringify(was) === JSON.stringify(now) ? was : now;
            if (notice.live) {
                nowLive[id] = LiveNotice.of(notice, said => daemon.ask(said),
                    then => Quickshell.activationToken(then), card);
                if (!(id in heardOpen)) {
                    heardOpen[id] = true;
                    if (arriving)
                        arrivals.push(id);
                }
            } else {
                delete nowLive[id];
                delete heardOpen[id];
            }
            if (notice.live && notice.resting)
                nowResting[id] = true;
            else
                delete nowResting[id];
            // Raised since it was last heard, not as the shell first
            // hears it, so a restart raises none.
            if (was && notice.raisedAt && notice.raisedAt !== was.raisedAt)
                raised = { id: id, at: notice.raisedAt };
        }
        // Open before it's in the history, so it never reads as missed and
        // gone from the screen for the moment between, which would flash
        // the margin.
        live = nowLive;
        resting = nowResting;
        // One its sender marks transient asks not to be kept, as niri's
        // screenshot notice does, so it's on screen but never in the
        // history.
        history = Object.values(made).filter(notice => !notice.transient)
            .sort((a, b) => b.at - a.at || b.id - a.id);
        for (const id of arrivals)
            came(live[id], made[id]);
    }

    Connections {
        target: daemon

        function onHeard(changed, gone) {
            root.took(changed, gone);
        }
    }

    // Whether the daemon holds notices back while the screen is shared,
    // which only the shell knows (Quiet.qml), said again each time it
    // connects; what Quiet keeps by Joel's asking, the daemon reads from
    // Quiet's own file.
    function tellQuiet() {
        daemon.ask({ ask: "quiet", sharing: Quiet.shared });
    }

    Connections {
        target: Quiet

        function onSharedChanged() {
            root.tellQuiet();
        }
    }

    onAnsweringChanged: {
        if (answering)
            tellQuiet();
    }

    // The last one said again in place that tells something new, so the
    // stack (Notifications.qml) brings it back to the front: { id, at }.
    property var raised: null

    // One said again in place that tells something new, as an arrival is
    // as it steps down (Arrivals.qml), comes back to the screen unread,
    // the same card rather than another. Its new words came with it, as
    // the daemon took it in place of the one before.
    function raise(id) {
        daemon.ask({ ask: "raise", notice: id });
    }

    // One resting comes back to the screen.
    function wake(id) {
        if (id in resting)
            daemon.ask({ ask: "wake", notice: id });
    }

    // One whose time on screen is up rests off it, still open.
    function rest(id) {
        if (id in live && !(id in resting))
            daemon.ask({ ask: "rest", notice: id });
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
        daemon.ask({ ask: "mark", notices: [id], read: read, how: how ?? null });
    }

    // Marks read every notice of `ids` at once, as For You's Desktop row's
    // Mark Them Read does (launcher/Now.qml), in one change. `how` says it
    // was read elsewhere, as "readInSlack" for Slack's own notices of what
    // its daemon says was read there (SlackUnreads.qml), so History says
    // where rather than "read here"; one already read keeps how it was.
    function markRead(ids, how) {
        if (history.some(notice => ids.includes(notice.id) && !notice.read))
            daemon.ask({ ask: "mark", notices: ids, read: true, how: how ?? null });
    }

    // Marks read every notice of a to-do that no longer stands, so
    // one that came and went unseen isn't missed once its item is done or
    // gone: `standing` holds the items Joel can still do now.
    function settleWaiting(standing) {
        const stale = notice => notice.waits && !standing[notice.waits]
            && (notice.missed || !notice.read);
        if (history.some(stale))
            daemon.ask({
                ask: "settle", standing: Object.keys(standing).filter(item => standing[item])
            });
    }

    // Everything kept has been seen, though not necessarily read. Only one
    // missed changes, so the rest stay as they were and the launcher's
    // rows of them are kept (launcher/PastNotices.qml): the launcher opening
    // with one missed made every notice's row again, 488 on Joel's shell
    // (2026-11-10).
    function seen() {
        if (missed > 0)
            daemon.ask({ ask: "seen" });
    }
}
