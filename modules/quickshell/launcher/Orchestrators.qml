// What Agents knows of the orchestrators, and no place of its own: each
// quest whose session orchestrates others, as the harness lists them
// (orchestrators.json under its quest workflow, written whole whenever one
// changes), which of them is closed, and how a closed one reopens. One
// running or parked is its session, a row of Agents' tree like any other;
// one closed stands dimmed at the tree's end, as Agents draws it, the one
// last live first (VKCF3Z, 2026-11-07 [9a4b1dd0]: the Orchestrators
// collection, a second listing of the same sessions, goes).
//
// After those come the talks Joel has had with a session, newest first, as
// it writes what became of each (the harness's talk files, one a talk,
// under the inbox's replies), read here for every session running and
// every orchestrator listed: his words, and beside them how it went,
// in one word that changes in place (delivered, reading, replying,
// replied, not delivered). The right arrow reads one: his words, how it
// went, and the reply below. The newest 20 are read, the newest of them
// watched while the shell runs, so a reply being written is seen as it's
// written, and all read again as the launcher opens.
//
// Only an inbox that holds talks is read: which do is listed once as the
// shell starts and again as the launcher opens, by one command over the
// steer folder. Each reader costs Qt a thread and a file watcher of its
// own, and on 2026-10-29 a reader for every session and orchestrator,
// made again whenever a window was focused, ran the shell out of files
// once a few dozen sessions had started, crashing it every half minute.
pragma ComponentBehavior: Bound
import qs
import Qt.labs.folderlistmodel
import Quickshell
import Quickshell.Io
import QtQuick
import "SessionRow.js" as SessionRow

Provider {
    id: provider

    // As the harness wrote them, or none before it has.
    property var listed: []
    // Where each quest starts a session, as the quest index says. Not
    // named `starts`, which the launcher asks a source for as the function
    // saying which row it opens on.
    property var startFolders: ({})
    // The session the quest index names to resume each quest by: its
    // newest that was last on that quest, named only while its folder is
    // there. Never the index's `sessions`, which can hold one long dead
    // whose pid was used again (Joel's Aquifer orchestrator, 2026-10-29).
    property var resumable: ({})
    // The folder each of those ran in, as the index says, looked for as the
    // launcher opens, so a closed one whose folder has gone says so, and
    // where it would start instead, before Return (VKCF3Z, 2026-11-07
    // [4a00987e]).
    property var resumeFolders: ({})
    // When each of those was last live, in milliseconds, which orders the
    // closed ones (SessionRow.liveAt).
    property var resumedAt: ({})
    function lookForFolders() {
        if (provider.launcher?.open)
            Sessions.lookForFolders(Object.values(resumeFolders)
                .concat(Object.values(startFolders)));
    }

    readonly property string harness: Sessions.state
    readonly property string inboxes: Sessions.inboxes

    // Its session, running or parked, by its address, or null: one parked
    // is woken, never resumed in a second window beside its own.
    function sessionOf(address) {
        return Sessions.running.find(one => one.address === address)
            ?? Sessions.parked.find(one => one.address === address) ?? null;
    }

    // Those closed, neither running nor parked nor present at all, the one
    // last live first, in the order fixed as the launcher opened, so none
    // moves under the cursor while it's open; one closed since comes last.
    readonly property var closed: {
        Sessions.present;
        const shut = listed.filter(one => !sessionOf(one.address)
            && !Sessions.present[one.address]);
        const at = one => {
            const placed = closedOrder.indexOf(one.address);
            return placed < 0 ? closedOrder.length : placed;
        };
        return shut.slice().sort((a, b) => at(a) - at(b));
    }
    // Their addresses, the one last live first, as the launcher last opened.
    property var closedOrder: []
    function orderClosed() {
        closedOrder = SessionRow.lastLiveFirst(listed.map(one => ({
            address: one.address,
            at: resumedAt[one.quest] ?? 0
        })));
    }

    // How a closed one's folder is gone, as last looked for: what its row
    // says, and the start afresh in its quest's start folder where that's
    // there, or null while its folder is there or not looked for yet.
    function goneFrom(one) {
        const cwd = resumeFolders[one.quest] ?? "";
        if (cwd === "" || !Sessions.folderGone(cwd))
            return null;
        const folder = startFolders[one.quest] ?? "";
        const said = Words.of("verb:orchestrator:afresh").said ?? {};
        const starts = folder !== "" && !Sessions.folderGone(folder);
        return {
            side: String(said[starts ? "gone" : "nowhere"] ?? "")
                .replace("{cwd}", homely(cwd)).replace("{folder}", homely(folder)),
            afresh: starts ? () => Sessions.start(one.quest, folder) : null
        };
    }
    function homely(folder) {
        const home = Sessions.home;
        return folder === home || folder.startsWith(`${home}/`)
            ? `~${folder.slice(home.length)}` : folder;
    }

    // Each talk with an orchestrator, by its address, newest first, as
    // its file says: { id, at, words, stage, after, reply, why, came }.
    property var talks: ({})

    function heardTalk(address, file, text) {
        // A folder Qt fell back to when the replies folder isn't there.
        if (!file.startsWith(inboxes))
            return;
        const id = file.replace(/^.*\//, "").replace(/\.jsonl$/, "");
        const talk = { id: id, at: "", words: "", stage: "", after: "", reply: "", why: "",
            came: "" };
        for (const line of text.split("\n")) {
            let event;
            try {
                event = JSON.parse(line);
            } catch (e) {
                event = null;
            }
            if (event)
                folded(talk, event);
        }
        const kept = (talks[address] ?? []).filter(one => one.id !== id).concat([talk])
            .sort((a, b) => String(b.at).localeCompare(String(a.at)));
        const next = Object.assign({}, talks);
        next[address] = kept;
        talks = next;
    }

    // One event of a talk folded into it, as the harness's talk.ts writes
    // them in turn.
    function folded(talk, event) {
        if (event.stage === "delivered") {
            talk.at = event.at ?? talk.at;
            talk.words = event.text ?? "";
        } else if (event.stage === "reading") {
            talk.after = event.after ?? "";
        } else if (event.stage === "replying" || event.stage === "replied") {
            talk.reply = event.text ?? talk.reply;
        } else if (event.stage === "failed") {
            talk.why = event.why ?? "";
        } else if (event.stage === "came") {
            talk.came = event.what ?? "";
            return;
        }
        talk.stage = event.stage ?? talk.stage;
    }

    // The mark each stage wears: hollow to full, as the queue's do.
    readonly property var stageMarks: ({
        delivered: "pending",
        reading: "active",
        replying: "active",
        replied: "complete",
        failed: "failed"
    })

    function stageSaid(talk) {
        const words = (Words.of("orchestrators:talk").said ?? {})[talk.stage] ?? talk.stage;
        return talk.stage === "reading" && talk.after !== "" ? `${words}, ${talk.after}` : words;
    }

    // A session's talks, by its address in either case, as Agents' rows and
    // an orchestrator's show them.
    function talksOf(asked) {
        const address = String(asked).toUpperCase();
        return (talks[address] ?? []).filter(talk => talk.words !== "").map(talk => thing({
            thing: true,
            key: `talk:${address}:${talk.id}`,
            verb: "read",
            name: talk.words,
            glyph: Theme.glyph[stageMarks[talk.stage] ?? "pending"],
            hue: "voice",
            side: stageSaid(talk),
            reads: talk.reply !== "" ? talk.reply : talk.why,
            nested: true,
            // A step's rows open by their targets, so it gives both, as a
            // part holding more does (Provider.qml's holderRow).
            verbs: () => provider.talkRows(address, talk.id),
            targets: () => provider.talkRows(address, talk.id)
        }));
    }

    // A talk read: his words, how it went, and what came of it, each a row
    // Return copies, the reply in the talk's own drawer.
    function talkRows(address, id) {
        const talk = (talks[String(address).toUpperCase()] ?? []).find(one => one.id === id);
        if (!talk)
            return [];
        const row = (key, side, copy) => copyRow({
            key: `talk:${id}:${key}`,
            name: Words.name(`orchestrators:talk:${key}`),
            side: side,
            copy: copy,
            glyph: ""
        });
        return [
            row("yours", talk.words, talk.words),
            row("went", stageSaid(talk), talk.reply !== "" ? talk.reply : talk.why),
            talk.came !== "" ? row("came", talk.came, talk.came) : null
        ].filter(one => one !== null);
    }

    // How many times the launcher has opened, so the talks are listed
    // again each time.
    property int looked: 0

    // The inboxes holding talks, by address, as the steer folder was last
    // listed.
    property var holdingTalks: []

    // Whose talks are read: every orchestrator listed and every session
    // running whose inbox holds talks, each once, in the order of their
    // addresses. Said as one line first, which changes only when who it
    // is does: the sessions running are made again and reordered as any
    // window is focused, and a new list each time would make every
    // inbox's readers again.
    readonly property string talkingSaid: {
        const addresses = listed.map(one => String(one.address ?? "").toUpperCase())
            .concat(Sessions.running.map(session => String(session.address).toUpperCase()));
        return addresses.filter((address, at) => address !== "" && addresses.indexOf(address)
            === at && holdingTalks.includes(address)).sort().join(" ");
    }
    readonly property var talking: talkingSaid === "" ? [] : talkingSaid.split(" ")

    // Lists which inboxes hold talks, unless it already is.
    function lookForTalks() {
        if (!talkFolders.running)
            talkFolders.running = true;
    }

    Process {
        id: talkFolders
        command: ["sh", "-c",
            'for d in "$1"/*/replies; do [ -d "$d" ] && basename "${d%/replies}"; done; true',
            "talks", provider.inboxes]
        stdout: StdioCollector {
            onStreamFinished: {
                const found = text.split("\n").filter(line => line !== "")
                    .map(line => line.toUpperCase()).sort();
                if (found.join(" ") !== provider.holdingTalks.join(" "))
                    provider.holdingTalks = found;
            }
        }
    }

    Component.onCompleted: lookForTalks()

    // A closed orchestrator reopened where it left off: the session the
    // quest index names to resume, in the folder
    // that session ran in (Joel, 2026-10-28: "i want to be able to easily
    // restart an orchestrator that is closed, continuing from where it
    // left off"); one with no session yet is started on its quest, in the
    // folder the quest index says a new one starts in, never the quest's
    // own. When the session's folder went since it was listed, it starts
    // afresh in that start folder, never the home folder, and the foot
    // says so once the launcher opens next, never a notice.
    function start(one) {
        const last = resumable[one.quest];
        if (!last) {
            Sessions.start(one.quest, startFolders[one.quest] ?? Sessions.home);
            return;
        }
        const name = one.short || one.title || one.address;
        const said = Words.of("verb:orchestrator:reopen").said ?? {};
        const told = (cwd, startedIn) => startedIn !== ""
            ? [name, String(said.afresh ?? "").replace("{folder}", homely(startedIn))]
            : cwd === "" ? [name, String(said.unfound ?? ""), Theme.alert]
            : [homely(cwd), String(said.isGone ?? ""), Theme.alert];
        Sessions.resume(last, one.quest, startFolders[one.quest] ?? "", (cwd, startedIn) =>
            provider.launcher?.tellOnOpen([told(cwd, startedIn)]));
    }

    // Each inbox's talks: its replies folder listed, its newest files
    // read, and the newest watched.
    Instantiator {
        model: provider.talking

        delegate: Scope {
            id: inbox

            required property var modelData
            readonly property string address: String(modelData)
            property var newest: []

            FolderListModel {
                id: replies
                // Looked at again as the launcher opens, since a folder
                // made since can't be watched until it's listed: the
                // trailing slash comes and goes, so the folder is new.
                folder: `file://${provider.inboxes}/${inbox.address}/replies`
                    + (provider.looked % 2 === 1 ? "/" : "")
                showDirs: false
                nameFilters: ["*.jsonl"]
                sortField: FolderListModel.Time
                onCountChanged: inbox.newest = Array.from({ length: Math.min(count, 20) },
                    (_, at) => String(get(at, "filePath")))
            }

            Instantiator {
                model: inbox.newest

                delegate: FileView {
                    id: talkFile

                    required property var modelData
                    required property int index
                    path: String(modelData)
                    // The newest alone, where a reply is being written.
                    watchChanges: index === 0
                    printErrors: false
                    onFileChanged: reload()
                    onLoaded: provider.heardTalk(inbox.address, path, text())

                    // Read again as the launcher opens, the newest's
                    // watch aside, since an older talk may still be
                    // answered.
                    readonly property Connections reopened: Connections {
                        target: provider

                        function onLookedChanged() {
                            talkFile.reload();
                        }
                    }
                }
            }
        }
    }

    // Read again as the launcher opens too, since a file that wasn't there
    // when the shell started can't be watched until it's read.
    Connections {
        target: provider.launcher
        ignoreUnknownSignals: true

        function onOpenChanged() {
            if (provider.launcher.open) {
                orchestrators.reload();
                questIndex.reload();
                provider.orderClosed();
                provider.lookForFolders();
                provider.looked += 1;
                provider.lookForTalks();
            }
        }
    }

    FileView {
        id: orchestrators
        path: `${provider.harness}/quest-workflow/orchestrators.json`
        watchChanges: true
        printErrors: false
        onFileChanged: reload()
        onLoaded: {
            try {
                provider.listed = JSON.parse(text()).orchestrators ?? [];
            } catch (e) {
                provider.listed = [];
            }
            if (!provider.launcher?.open)
                provider.orderClosed();
        }
        onLoadFailed: provider.listed = []
    }

    FileView {
        id: questIndex
        path: `${provider.harness}/quest-workflow/quests.json`
        watchChanges: true
        printErrors: false
        onFileChanged: reload()
        onLoaded: {
            try {
                const out = {};
                const resumes = {};
                const folders = {};
                const lastLive = {};
                for (const quest of JSON.parse(text()).quests ?? []) {
                    if (quest.start)
                        out[quest.id] = quest.start;
                    if (typeof quest.resume?.id === "string")
                        resumes[quest.id] = quest.resume.id;
                    if (typeof quest.resume?.cwd === "string")
                        folders[quest.id] = quest.resume.cwd;
                    if (quest.resume)
                        lastLive[quest.id] = SessionRow.liveAt(quest.resume);
                }
                provider.startFolders = out;
                provider.resumable = resumes;
                provider.resumeFolders = folders;
                provider.resumedAt = lastLive;
            } catch (e) {
                provider.startFolders = {};
                provider.resumable = {};
                provider.resumeFolders = {};
                provider.resumedAt = {};
            }
            // Ordered afresh while the launcher is shut, never under the
            // cursor while it's open.
            if (!provider.launcher?.open)
                provider.orderClosed();
            provider.lookForFolders();
        }
    }
}
