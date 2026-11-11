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
// under the inbox's replies): his words, and beside them how it went, in
// one word that changes in place (delivered, reading, replying, replied,
// not delivered). The right arrow reads one: his words, how it went, and
// the reply below. The realm's node serves each inbox's newest 20, each
// talk a record of its lines, and tells each line as it's written, so a
// reply being written is seen as it's written with no file read here: a
// reader of Qt's own for each inbox ran the shell out of files on
// 2026-10-29.
pragma ComponentBehavior: Bound
import qs
import Quickshell
import Quickshell.Io
import QtQuick
import "Order.js" as Order
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
        void Sessions.present;
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

    // Each talk with a session, by its address, newest first, its lines
    // folded: { id, at, words, stage, after, reply, why, came }; from the
    // talks every agent source tells of (AgentSources), pi's as the
    // realm's node serves them, held while the node isn't answering.
    readonly property var talks: {
        const byAddress = {};
        for (const record of AgentSources.talks) {
            const address = String(record?.address ?? "").toUpperCase();
            const talk = { id: String(record?.id ?? ""), at: "", words: "", stage: "", after: "",
                reply: "", why: "", came: "" };
            for (const event of Array.isArray(record?.lines) ? record.lines : []) {
                if (event !== null && typeof event === "object")
                    folded(talk, event);
            }
            byAddress[address] = (byAddress[address] ?? []).concat([talk]);
        }
        for (const address in byAddress)
            byAddress[address] = Order.sorted(byAddress[address],
                Order.stream(talk => String(talk.at ?? "")));
        return byAddress;
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

    // Whether the quest index was written since it was last read, or
    // never read. It's 321 KB, written again about twice a minute, and
    // nothing a shut launcher shows is read from it, so it's read only
    // where its rows are made: as the launcher opens or catalogues, or as
    // it's written while either is (RO9YDD, 2026-11-11 [30210dd4]). The
    // agents daemon takes it on Fri 13 Nov, serving the closed
    // orchestrators' rows ready.
    property bool questsUnread: true
    // Whether an opening's read of the index is still to land, which
    // orders the closed ones as it does: ordered before it, they'd keep
    // the order the orchestrators' file lists them in, which the first
    // opening after the shell starts always did (vm-desktop-looks,
    // 2026-11-11: Beacon then Lamplight). It lands before a key can move
    // the cursor.
    property bool orderAsRead: false
    function showing() {
        return !!provider.launcher && (provider.launcher.open || provider.launcher.cataloguing);
    }
    // Whether the index is being read.
    function readQuests() {
        if (questsUnread)
            questIndex.reload();
        return questsUnread;
    }

    // Read again as the launcher opens too, since a file that wasn't there
    // when the shell started can't be watched until it's read. The closed
    // ones are ordered again as it shuts, by what was read as it opened.
    Connections {
        target: provider.launcher
        ignoreUnknownSignals: true

        function onOpenChanged() {
            if (provider.launcher.open) {
                orchestrators.reload();
                provider.orderAsRead = provider.readQuests();
                if (!provider.orderAsRead)
                    provider.orderClosed();
                provider.lookForFolders();
            } else {
                provider.orderClosed();
            }
        }
        function onCataloguingChanged() {
            if (provider.launcher.cataloguing)
                provider.readQuests();
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
        onFileChanged: {
            provider.questsUnread = true;
            if (provider.showing())
                reload();
        }
        onLoadFailed: {
            provider.questsUnread = true;
            if (provider.orderAsRead)
                provider.orderClosed();
            provider.orderAsRead = false;
        }
        onLoaded: {
            provider.questsUnread = false;
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
            // Ordered afresh while the launcher is shut, or as the read
            // its opening began lands, never under the cursor while it's
            // open.
            if (!provider.launcher?.open || provider.orderAsRead)
                provider.orderClosed();
            provider.orderAsRead = false;
            provider.lookForFolders();
        }
    }
}
