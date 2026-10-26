pragma Singleton
pragma ComponentBehavior: Bound
// What the agents have waiting on Joel. The agent steering the others, the
// steward, keeps one file of what
// waits on him, ~/.local/state/mission/queue.json, and each quest keeps its
// own beside it, waiting/<quest>.json, each replaced whole as it changes:
// the steward's address and each item's title, why it waits, how many
// minutes it takes, when, the title of the window it's done in (or none,
// when it's done with the steward itself), its quest, and whether Joel can
// do it now. An item that can be done now is said once as a notice, the
// moment it's heard, and remembered as said across restarts, so the
// agents never have to wait on a turn to tell him. All of it is heard
// rather than read on a clock. Both folders are made at login (this
// module's home.nix), so the watch hears the first file.
import qs
import Qt.labs.folderlistmodel
import Quickshell
import Quickshell.Io
import QtQuick

Singleton {
    id: root

    readonly property string home: Quickshell.env("HOME")
    readonly property string state: Quickshell.env("XDG_STATE_HOME") || `${home}/.local/state`
    readonly property string queue: `${state}/mission/queue.json`
    readonly property string quests: `${state}/mission/waiting`

    // The steward's file as written, or null when there's none that reads.
    property var said: null
    // Each quest's file as written, by its path.
    property var kept: ({})
    // The steward's address, whose window an item with none is done in.
    readonly property string steward: typeof said?.steward === "string" ? said.steward : ""

    // A file's items, each with a title and the quest it's from: the
    // item's own, else the file's, else the file's name.
    function itemsOf(read, quest) {
        const listed = Array.isArray(read) ? read : read?.items;
        const from = typeof read?.quest === "string" ? read.quest : quest;
        return (Array.isArray(listed) ? listed : [])
            .filter(item => typeof item?.title === "string" && item.title !== "")
            .map(item => Object.assign({}, item, {
                quest: typeof item.quest === "string" ? item.quest : from,
                ready: item.ready === true
            }));
    }

    // What waits on Joel: the steward's first, in its order, then each
    // quest's, by its file's name, so the first is the steward's choice.
    readonly property var items: Object.keys(kept).sort().reduce((all, path) => all.concat(
        itemsOf(kept[path], path.replace(/^.*\//, "").replace(/\.json$/, ""))),
        itemsOf(said, steward))
    // The first Joel can do now, or else the first.
    readonly property var foremost: items.find(item => item.ready) ?? items[0] ?? null

    // An item as one line: its title, and how long it takes when it says.
    function titled(item) {
        const said = (Words.of("on:waiting").said ?? {}).first ?? "{title}";
        return typeof item.minutes === "number"
            ? said.replace("{title}", item.title).replace("{minutes}", item.minutes) : item.title;
    }

    function capitalised(word) {
        return word.charAt(0).toUpperCase() + word.slice(1);
    }

    // Who an item is, across every file: its quest and its id.
    function identity(item) {
        return `${item.quest}:${item.id ?? item.title}`;
    }

    // The steward's own session, by its address or another it answers to.
    function session() {
        const presence = Sessions.present[steward] ?? null;
        return Sessions.running.find(one => one.address === steward
            || (presence !== null && one.presence === presence)) ?? null;
    }

    // Where an item is done: the window it names, by its title, or the
    // steward's own when it names none; null when neither is open.
    function windowOf(item) {
        const titled = typeof item.window === "string" && item.window !== ""
            ? Object.values(Niri.windows).find(window => window.title === item.window) : undefined;
        if (titled)
            return titled;
        const own = session();
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

    function notice() {
        if (!remembered)
            return;
        const fresh = items.filter(item => item.ready && !noticed[identity(item)]);
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
    onItemsChanged: Qt.callLater(notice)
    onRememberedChanged: Qt.callLater(notice)

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
                `--action=go=${root.capitalised(Words.verb("on:waiting"))}`,
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

    FolderListModel {
        id: folder
        folder: `file://${root.quests}`
        showDirs: false
        nameFilters: ["*.json"]
    }

    Instantiator {
        model: folder
        // A quest whose file goes has nothing waiting any more.
        onObjectRemoved: (index, object) => root.forget((object as FileView).path)

        FileView {
            required property string filePath
            path: filePath
            watchChanges: true
            printErrors: false
            onFileChanged: reload()
            onLoaded: root.heard(filePath, text())
        }
    }

    function forget(path) {
        const next = Object.assign({}, kept);
        delete next[path];
        kept = next;
    }

    function heard(path, text) {
        // A folder Qt fell back to when the quests' folder isn't there.
        if (!path.startsWith(quests))
            return;
        const next = Object.assign({}, kept);
        try {
            next[path] = JSON.parse(text);
        } catch (error) {
            delete next[path];
        }
        kept = next;
    }

    FileView {
        id: file
        path: root.queue
        watchChanges: true
        printErrors: false
        onFileChanged: reload()
        onLoaded: {
            try {
                const read = JSON.parse(file.text());
                root.said = typeof read === "object" && read !== null ? read : null;
            } catch (error) {
                root.said = null;
            }
        }
        onLoadFailed: root.said = null
    }
}
