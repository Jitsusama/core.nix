pragma Singleton
pragma ComponentBehavior: Bound
// What only Joel can do (Needs You). Each quest keeps a file of what it
// needs him for, ~/.local/state/mission/waiting/<quest>.json, written by
// the session that needs him through mesh_wait and replaced whole as it
// changes: each item's title, why only he can, how many minutes it takes,
// where and by when, its steps, its quest and session, whether he can do it
// now, and how it knows it's done and ready. The to do daemon alone writes
// what it found of each, mission/verdicts.json: done once its check passed
// or its signal came, or ready. Waiting.js decides what of them is true:
// what Joel can do now and what waits until later, nothing of a quest
// that's ended, as its README says, and nothing found or said done. His
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
// age. Both folders are made at login (this module's home.nix), so the
// watch hears the first file.
import qs
import Qt.labs.folderlistmodel
import Quickshell
import Quickshell.Io
import QtQuick
import "Waiting.js" as Waiting

Singleton {
    id: root

    readonly property string home: Quickshell.env("HOME")
    readonly property string state: Quickshell.env("XDG_STATE_HOME") || `${home}/.local/state`
    readonly property string quests: `${state}/mission/waiting`
    readonly property string settledAt: `${state}/mission/settled.json`
    readonly property string verdictsAt: `${state}/mission/verdicts.json`

    // Each quest's file as written, by its path.
    property var kept: ({})
    // What the to do daemon found, or none while it has said nothing.
    property var verdicts: null

    // Each quest's status and title as its README says, by its id: null
    // once its README is found missing, and absent until it's read.
    property var owners: ({})
    // What Joel has said of each item: { done, notNow }, by identity.
    property var settled: ({})

    // What waits on Joel, as Waiting.js weighs it: `now`, what he can do
    // now, and `later`, what he can't yet or has put off, each anything for
    // a time first, then the quickest.
    readonly property var weighed: Waiting.sorted(kept, owners, settled, verdicts)
    readonly property var now: weighed.now
    readonly property var later: weighed.later
    readonly property var items: now.concat(later)
    // The first Joel can do now, or else the first.
    readonly property var foremost: now[0] ?? later[0] ?? null
    // Who has something Joel can do now, so a session's want to the same
    // end is true (Sessions.qml).
    readonly property var standing: Waiting.standing(now)

    // The quest an item is from, by its README's title, else its id.
    function questOf(item) {
        return owners[item.quest]?.title || item.quest;
    }

    // Joel's word on an item, "done" or "notNow", kept at once.
    function settle(item, how) {
        settled = Waiting.settle(settled, item, how, Date.now());
        settledFile.setText(JSON.stringify(settled));
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

    // Nothing is said until every quest the files name has been read, so an
    // ended quest's item is never said in the moment before its README is.
    function notice() {
        if (!remembered || named.split(" ").some(quest => quest !== "" && !(quest in owners)))
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
    onOwnersChanged: Qt.callLater(notice)
    onRememberedChanged: Qt.callLater(notice)

    // A notice goes, marked read, a moment after its item stops being one
    // Joel can do now: done, put off, gone or of an ended quest. The
    // moment lets a file replaced by rename, which takes its items away
    // for an instant, pass. What's remembered as said forgets one that no
    // longer waits once it was said a week ago, so the memory never grows;
    // a week, rather than at once, since the files are read a moment after
    // the shell starts and every item would look gone until they are.
    Timer {
        id: withdrawing
        interval: 1000
        onTriggered: {
            if (!root.remembered)
                return;
            const standing = {};
            for (const item of root.now)
                standing[root.identity(item)] = true;
            for (const id in Notices.live) {
                const notice = Notices.live[id];
                const said = notice.hints["x-waiting"];
                if (notice.appName === "waiting" && said !== undefined && !standing[said]) {
                    Notices.mark(notice.id, true);
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
    // when Joel takes it. It's named by what's asked, its minutes below,
    // and its sender says it waits on him: named "Waiting on you", the
    // notices kept since were a second thing of that name beside Now's row.
    function minutes(item) {
        const said = (Words.of("on:waiting").said ?? {}).minutes ?? "{minutes} min";
        return typeof item.minutes === "number" ? said.replace("{minutes}", item.minutes) : "";
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
                item.title, root.minutes(item)]
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

    // The quests' files, each written beside and renamed in, which the
    // folder's one watch hears, so every change reads them all in one run.
    FolderListModel {
        id: folder
        folder: `file://${root.quests}`
        showDirs: false
        nameFilters: ["*.json"]
        onRowsInserted: root.read()
        onRowsRemoved: root.read()
        onDataChanged: root.read()
        onModelReset: root.read()
    }

    // Every file read in one run, each after a record separator and its
    // path on a line of its own, which JSON never holds unescaped.
    Process {
        id: reader
        command: ["sh", "-c", "for file in \"$1\"/*.json; do [ -f \"$file\" ] || continue;"
            + " printf '\\036%s\\n' \"$file\"; cat \"$file\"; done", "sh", root.quests]
        onExited: {
            if (root.again) {
                root.again = false;
                reader.running = true;
            }
        }
        stdout: StdioCollector {
            onStreamFinished: root.heard(text)
        }
    }

    // Asked to read while reading, so the last change always counts.
    property bool again: false

    function read() {
        if (reader.running)
            again = true;
        else
            reader.running = true;
    }

    // The quests the files name, as one string, so a file heard again that
    // names the same leaves their READMEs' watch as it was.
    readonly property string named: Waiting.quests(kept).join(" ")
    readonly property string readmes: `${Quickshell.env("XDG_DATA_HOME") || `${home}/.local/share`}/pi/agentic-harness.pi/quest-workflow/quests`

    // Each named quest's README, watched, so one concluded or retired takes
    // its items with it, and one whose folder goes does too. A status once
    // read is kept as the watch is remade, so nothing ended stands again
    // for the moment between.
    Instantiator {
        model: root.named === "" ? [] : root.named.split(" ")

        FileView {
            required property string modelData
            path: `${root.readmes}/${modelData}/README.md`
            watchChanges: true
            printErrors: false
            onFileChanged: reload()
            onLoaded: root.owned(modelData, Waiting.readme(text()))
            onLoadFailed: root.owned(modelData, null)
        }
    }

    function owned(quest, readme) {
        if (JSON.stringify(owners[quest]) === JSON.stringify(readme))
            return;
        const next = Object.assign({}, owners);
        next[quest] = readme;
        owners = next;
    }

    // Each file by its path, one that doesn't parse left out, and a quest
    // whose file went has nothing waiting any more. Kept as it was when
    // nothing changed, so nothing weighed from it is made again.
    function heard(text) {
        const next = {};
        for (const part of text.split("\x1e")) {
            const end = part.indexOf("\n");
            if (end < 0)
                continue;
            try {
                next[part.slice(0, end)] = JSON.parse(part.slice(end + 1));
            } catch (error) {
                continue;
            }
        }
        if (JSON.stringify(next) !== JSON.stringify(kept))
            kept = next;
    }

    // The to do daemon's file, replaced whole by rename and only when a
    // verdict changes, so a change heard is news.
    FileView {
        id: found
        path: root.verdictsAt
        watchChanges: true
        printErrors: false
        onFileChanged: reload()
        onLoaded: {
            try {
                const read = JSON.parse(found.text());
                root.verdicts = typeof read === "object" && read !== null ? read : null;
            } catch (error) {
                root.verdicts = null;
            }
        }
        onLoadFailed: root.verdicts = null
    }
}
