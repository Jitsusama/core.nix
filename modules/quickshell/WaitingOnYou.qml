pragma Singleton
pragma ComponentBehavior: Bound
// What only Joel can do (For You). Each quest keeps a file of what it
// needs him for, ~/.local/state/mission/waiting/<quest>.json, written by
// the session that needs him through mesh_wait and replaced whole as it
// changes: each item's title, why only he can, how many minutes it takes,
// where and by when, its steps, its quest and session, whether he can do it
// now, and how it knows it's done and ready. The to do daemon alone writes
// what it found of each, mission/verdicts.json: done once its check passed
// or its signal came, or ready. The machine's mycelium node serves both,
// and the harness's index of the quests still open, as records over the
// daemon interface, so the shell reads no file of the agents' and runs
// nothing to: it's told each change as the node hears it. Waiting.js
// decides what of them is true: what Joel can do now and what waits until
// later, nothing of a quest that's ended, as the index says by no longer
// holding it, and nothing found or said done. His
// word on each, done or not now, is kept in mission/settled.json, the
// shell's own beside the agents' files, so it stays across restarts and an
// agent can read what he said; the shell never writes the agents' files.
// The orchestrator's own queue.json is no longer read: it raises its asks
// through mesh_wait under its own quest, as every session does (VKCF3Z,
// 2026-10-30). An item that can be done now
// is said once as a notice, the moment it's heard, and remembered as said
// across restarts, so the agents never have to wait on a turn to tell him;
// its notice goes, read, once it can't be done now any more.
// All of it is heard rather than read on a clock, and nothing goes by its
// age. Nothing is said, or withdrawn, until the node has answered for
// every kind, so nothing is said of an ended quest, or of one the to do
// daemon found done, in the moment before the node says so.
import qs
import Quickshell
import Quickshell.Io
import QtQuick
import "Waiting.js" as Waiting

Singleton {
    id: root

    readonly property string home: Quickshell.env("HOME")
    readonly property string state: Quickshell.env("XDG_STATE_HOME") || `${home}/.local/state`
    readonly property string settledAt: `${state}/mission/settled.json`

    // The node's records of each kind For You reads (mycelium's
    // PROTOCOL.md, The Daemon Interface): each quest's items, the to do
    // daemon's verdicts and the quests still open, each watched whole and
    // then told each change as only what differs. What was held stays
    // held while the node isn't answering, as while a switch restarts it,
    // and what went in the gap goes once it answers again.
    readonly property var waitingKind: waitingLink.item
    readonly property var verdictKind: verdictLink.item
    readonly property var questKind: questLink.item
    LazyLoader {
        id: waitingLink
        active: Sessions.nodePath !== ""

        Daemon {
            socket: Sessions.nodePath
            kind: "waiting"
            changesDiffer: true
            answersWhole: true
        }
    }
    LazyLoader {
        id: verdictLink
        active: Sessions.nodePath !== ""

        Daemon {
            socket: Sessions.nodePath
            kind: "verdict"
            changesDiffer: true
            answersWhole: true
        }
    }
    LazyLoader {
        id: questLink
        active: Sessions.nodePath !== ""

        Daemon {
            socket: Sessions.nodePath
            kind: "quest"
            changesDiffer: true
            answersWhole: true
        }
    }
    // Whether the node has answered for every kind, so all of it is known.
    readonly property bool heardAll: waitingKind?.answered === true
        && verdictKind?.answered === true && questKind?.answered === true

    // What each of the node's kinds last said, kept while it's told again
    // saying the same (Waiting.js's kept), so nothing below is weighed
    // again for it. A plain object, never reassigned.
    readonly property var toldLast: ({})

    // Each item the quests keep, as the node serves them.
    readonly property var asked: {
        void waitingKind?.revision;
        return Waiting.kept(toldLast, "asked", Waiting.served(waitingKind?.held ?? {}));
    }
    // What the to do daemon found, or none while the node hasn't said.
    readonly property var verdicts: {
        void verdictKind?.revision;
        return Waiting.kept(toldLast, "verdicts",
            verdictKind?.answered ? Waiting.verdictsOf(verdictKind.held) : null);
    }
    // The quests the items name, as one string, so the items heard again
    // naming the same leave their owners as they were.
    readonly property string named: Waiting.quests(asked).join(" ")
    // Each named quest's status and title as the index says, by its id:
    // null once the index no longer holds it, and absent until the node
    // has answered.
    readonly property var owners: {
        void questKind?.revision;
        return Waiting.kept(toldLast, "owners", Waiting.owners(questKind?.answered
            ? questKind.held : null, named === "" ? [] : named.split(" ")));
    }
    // What Joel has said of each item: { done, notNow }, by identity.
    property var settled: ({})

    // What waits on Joel, as Waiting.js weighs it: `now`, what he can do
    // now, and `later`, what he can't yet or has put off, each anything for
    // a time first, then the quickest.
    readonly property var weighed: Waiting.sorted(asked, owners, settled, verdicts)
    readonly property var now: weighed.now
    readonly property var later: weighed.later
    readonly property var items: now.concat(later)
    // The first Joel can do now, or else the first.
    readonly property var foremost: now[0] ?? later[0] ?? null
    // Who has something Joel can do now, so a session's want to the same
    // end is true (Sessions.qml).
    readonly property var standing: Waiting.standing(now)

    // The quest an item is from, by its title in the index, else its id.
    function questOf(item) {
        return owners[item.quest]?.title || item.quest;
    }

    // Any quest's status and title as the index says, by its id, as a look
    // names the quest that pointed at it: null where the index doesn't
    // hold it, and absent until the node has answered. Read as a drawer
    // is shown, so the index needn't be weighed for each look. The index
    // changes in place, so its revision is read first, as owners does, and
    // a drawer held open follows a quest's new title or status.
    function ownerOf(quest) {
        void questKind?.revision;
        return Waiting.owners(questKind?.answered ? questKind.held : null, [quest])[quest];
    }

    // Joel's word on an item, "done" or "notNow", shown at once and kept
    // by the to-do daemon, which writes settled.json from then on (Y2YBTS,
    // 2026-11-10), so the shell writes no file for it. One the daemon
    // can't keep, as while it's down or for an item it doesn't hold, is
    // written here as before, so nothing he said is lost.
    function settle(item, how) {
        settled = Waiting.settle(settled, item, how, Date.now());
        const word = settled;
        toDo.ask({ ask: "act", act: how, item: identity(item) }, said => {
            if (said?.done !== true)
                settledFile.setText(JSON.stringify(word));
        });
    }

    // The to-do daemon, asked only to keep Joel's word, so never watched
    // here: what waits on him is heard above as it was.
    Daemon {
        id: toDo

        socket: `${Quickshell.env("XDG_RUNTIME_DIR")}/daemons/to-do.sock`
        kind: "to-do"
        live: false
    }

    FileView {
        id: settledFile
        path: root.settledAt
        printErrors: false
        atomicWrites: true
        // The orchestrator may clear what Joel said, so a change is read.
        watchChanges: true
        onFileChanged: reload()
        onLoaded: {
            try {
                const read = JSON.parse(settledFile.text());
                root.settled = typeof read === "object" && read !== null ? read : {};
            } catch (error) {
                root.settled = {};
            }
        }
    }

    // An item as one line: its title, and how long it takes when it says.
    function titled(item) {
        const said = (Words.of("on:waiting").said ?? {}).first ?? "{title}";
        return typeof item.minutes === "number"
            ? said.replace("{title}", item.title).replace("{minutes}", item.minutes) : item.title;
    }

    // Who an item is, across every file: its quest and its id.
    function identity(item) {
        return Waiting.identity(item);
    }

    // The session an item is from, by its owner's address, else its file's,
    // or another address that session answers to.
    function session(item) {
        const address = item.owner?.address ?? item.from;
        const presence = Sessions.present[address] ?? null;
        return Sessions.running.find(one => one.address === address
            || (presence !== null && one.presence === presence)) ?? null;
    }

    // What one not ready yet waits for (Waiting.js).
    function waitsFor(item) {
        return Waiting.waitsFor(item);
    }

    // How an item will be known done (Waiting.js).
    function doneBy(item) {
        return Waiting.doneBy(item);
    }

    // Whether anything but Joel will see an item done (Waiting.js).
    function watched(item) {
        const live = {};
        for (const one of Sessions.running)
            live[one.address] = true;
        return Waiting.watched(item, live);
    }

    // Where an item is done: the window it names, by its exact title, or
    // the window of the session it's from when it names none; null when
    // neither is open.
    function windowOf(item) {
        const named = item.where?.window ?? item.window;
        const titled = typeof named === "string" && named !== ""
            ? Object.values(Niri.windows).find(window => window.title === named) : undefined;
        if (titled)
            return titled;
        const own = session(item);
        return own ? own.window : null;
    }

    function go(item) {
        const window = windowOf(item);
        if (window)
            Niri.act({ FocusWindow: { id: window.id } });
    }

    // Those said as a notice, by identity, kept so a restart says none
    // twice.
    property var noticed: ({})
    property bool remembered: false
    readonly property real forgetting: 7 * 24 * 3600 * 1000

    // Nothing is said until the node has answered for every kind, so an
    // ended quest's item is never said in the moment before the index is.
    function notice() {
        if (!remembered || !heardAll)
            return;
        const fresh = now.filter(item => !noticed[identity(item)]);
        if (fresh.length === 0)
            return;
        const next = Object.assign({}, noticed);
        for (const item of fresh) {
            next[identity(item)] = Date.now();
            asking.createObject(root, { item: item });
        }
        noticed = next;
        memory.setText(JSON.stringify(noticed));
    }
    onNowChanged: {
        Qt.callLater(notice);
        withdrawing.restart();
    }
    onHeardAllChanged: {
        Qt.callLater(notice);
        withdrawing.restart();
    }
    onRememberedChanged: Qt.callLater(notice)

    // A notice goes, marked read, a moment after its item stops being one
    // Joel can do now: done, put off, gone or of an ended quest. The
    // moment lets a file replaced by rename, which takes its items away
    // for an instant, pass. What's remembered as said forgets one that no
    // longer waits once it was said a week ago, so the memory never grows;
    // a week, rather than at once, so one that went while the shell wasn't
    // running is still known as said should it come back. Nothing is
    // withdrawn before the node has answered, while every item looks gone.
    Timer {
        id: withdrawing
        interval: 1000
        onTriggered: {
            if (!root.remembered || !root.heardAll)
                return;
            const standing = {};
            for (const item of root.now)
                standing[root.identity(item)] = true;
            for (const id in Notices.live) {
                const notice = Notices.live[id];
                const said = notice.hints["x-waiting"];
                if (notice.appName === "waiting" && said !== undefined && !standing[said]) {
                    Notices.mark(notice.id, true, "withdrawn");
                    notice.dismiss();
                }
            }
            // And one already gone from the screen unseen, so it isn't
            // counted missed once its item is done.
            Notices.settleWaiting(standing);
            const waits = {};
            for (const item of root.items)
                waits[root.identity(item)] = true;
            const forgotten = Date.now() - root.forgetting;
            const kept = Object.keys(root.noticed)
                .filter(said => waits[said] || root.noticed[said] > forgotten);
            if (kept.length === Object.keys(root.noticed).length)
                return;
            const next = {};
            for (const said of kept)
                next[said] = root.noticed[said];
            root.noticed = next;
            memory.setText(JSON.stringify(next));
        }
    }

    // A notice offering to go where it's done, which notify-send prints
    // when Joel takes it. It's named by what's asked, its minutes and its
    // quest below, and its sender says it waits on him: named "Waiting on
    // you", the notices kept since were a second thing of that name beside
    // Now's row. It's the only notice of an action, since a session's want
    // posts none (Sessions.qml), so the quest it's for is said here, as
    // plain text, since a body is read as markup.
    function told(item) {
        const said = Words.of("on:waiting").said ?? {};
        const minutes = typeof item.minutes === "number"
            ? (said.minutes ?? "{minutes} min").replace("{minutes}", item.minutes) : "";
        return [minutes, questOf(item)].filter(part => part !== "").join(" \u00b7 ")
            .replace(/&/g, "&amp;").replace(/</g, "&lt;").replace(/>/g, "&gt;");
    }

    Component {
        id: asking

        Process {
            id: process

            property var item: null

            running: item !== null
            command: item === null ? [] : ["notify-send", "--app-name=waiting",
                `--action=go=${Words.name("verb:on:waiting:go")}`,
                `--hint=string:x-waiting:${root.identity(item)}`,
                item.title, root.told(item)]
            stdout: SplitParser {
                onRead: line => {
                    if (line.trim() === "go")
                        root.go(process.item);
                }
            }
            onExited: process.destroy()
        }
    }

    FileView {
        id: memory
        path: Quickshell.statePath("waiting-noticed.json")
        printErrors: false
        onLoaded: {
            try {
                const read = JSON.parse(memory.text());
                root.noticed = typeof read === "object" && read !== null ? read : {};
            } catch (error) {
                root.noticed = {};
            }
            root.remembered = true;
        }
        onLoadFailed: root.remembered = true
    }
}
