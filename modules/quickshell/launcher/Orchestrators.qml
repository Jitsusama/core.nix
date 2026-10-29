// The orchestrators, a collection in Agents: each quest whose session
// orchestrates others, as the harness lists them (orchestrators.json under
// its quest workflow, written whole whenever one changes), so Joel can pick
// one and talk to it (Joel, 2026-10-25: "it needs a good name and should
// allow me to pick an orchestrator"). One waiting on him comes first, then
// those listening, those busy, those paused and those not running, each
// saying so beside its name, as its session's presence says.
//
// One running is its session, drawn as every session is (Agents.qml,
// SessionRow.js): Return goes to its window, and stepping in gives its acts,
// Talk, Pause or Resume and Stop, each left in its steer inbox
// (Sessions.qml), then what's known of it. One not running resumes where
// it left off instead.
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
import "../Fuzzy.js" as Fuzzy

Provider {
    id: provider

    what: Words.of("orchestrators").what

    collection: Words.collection("orchestrators")

    // As the harness wrote them, or none before it has.
    property var listed: []
    // Where each quest starts a session, and the last session it had, as
    // the quest index says. Not named `starts`, which the launcher asks a
    // source for as the function saying which row it opens on.
    property var startFolders: ({})
    property var lastSessions: ({})

    readonly property string harness: Sessions.state
    readonly property string inboxes: Sessions.inboxes

    // How each stands, in the order they're listed: what only Joel can
    // move first (collection-order.md under the launcher's quest).
    readonly property var standings: ["waiting", "listening", "busy", "paused", "off"]

    function standing(presence) {
        if (!presence)
            return "off";
        if (presence.state === "paused")
            return "paused";
        if (presence.wants || presence.state === "waiting")
            return "waiting";
        if (presence.state === "listening")
            return "listening";
        return presence.busy || presence.state === "busy" ? "busy" : "listening";
    }

    // Its running session, by its address, or null.
    function sessionOf(address) {
        return Sessions.running.find(one => one.address === address) ?? null;
    }

    readonly property var all: {
        Sessions.present;
        Niri.revision;
        const rows = listed.map((one, at) => ({ one: one, at: at,
            stands: standing(Sessions.present[one.address] ?? null) }));
        rows.sort((a, b) => standings.indexOf(a.stands) - standings.indexOf(b.stands)
            || a.at - b.at);
        return rows.map(row => made(row.one, row.stands));
    }
    onAllChanged: changedTo(all)

    function said(stands, presence) {
        const words = Words.of("orchestrators").said ?? {};
        const step = stands === "busy" && presence && presence.step ? `, ${presence.step}` : "";
        return `${words[stands] ?? stands}${step}`;
    }

    // Another provider, by its name, as Agents finds its own.
    function source(name) {
        return (provider.launcher?.sources ?? []).find(other => other.settings.name === name)
            ?? null;
    }

    function made(one, stands) {
        const presence = Sessions.present[one.address] ?? null;
        // A running one is its session, the one row a session is wherever
        // it shows (SessionRow.js): its acts, its drawer and what's inside are
        // Agents' own, under the orchestrator's name.
        const session = stands === "off" ? null : sessionOf(one.address);
        const own = session ? source("agents")?.made(session) ?? null : null;
        // Its talks with Joel follow what the session holds, as only an
        // orchestrator's inbox keeps them.
        const inside = own?.verbs ?? null;
        if (own)
            return Object.assign(own, {
                key: `orchestrator:${one.address}`,
                name: one.short || one.title || one.address,
                glyph: Theme.glyph[stands === "waiting" ? "waitsOnJoel" : "agent"],
                stands: stands,
                verbs: () => (inside ? inside() : []).concat(provider.talksOf(one.address))
            });
        const row = member({
            key: `orchestrator:${one.address}`,
            verb: "open",
            name: one.short || one.title || one.address,
            glyph: Theme.glyph[stands === "waiting" ? "waitsOnJoel" : "agent"],
            hue: "program",
            side: said(stands, presence),
            note: one.title ?? "",
            stands: stands,
            verbs: () => provider.verbs(one, stands)
        });
        // A closed one resumes on Return, its Resume leading the step in.
        if (stands === "off")
            Object.assign(row, {
                verb: Words.verb("verb:orchestrator:resume"),
                run: () => provider.start(one)
            });
        return row;
    }

    function act(id, fields) {
        return thing(Object.assign({
            key: `verb:orchestrator:${id}`,
            verb: Words.verb(`verb:orchestrator:${id}`),
            name: Words.name(`verb:orchestrator:${id}`),
            glyph: Words.glyph(`verb:orchestrator:${id}`),
            hue: "action"
        }, fields));
    }

    // A closed one's only act is to resume it; a running one's are its
    // session's, as made() gives them, and one whose session isn't found
    // yet offers its talks alone.
    function verbs(one, stands) {
        if (stands === "off")
            return [act("resume", { run: () => provider.start(one) })];
        return talksOf(one.address);
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
    // Return copies, the reply below the rows.
    function talkRows(address, id) {
        const talk = (talks[String(address).toUpperCase()] ?? []).find(one => one.id === id);
        if (!talk)
            return [];
        const row = (key, side, copy) => copyRow({
            key: `talk:${id}:${key}`,
            name: Words.name(`orchestrators:talk:${key}`),
            side: side,
            copy: copy,
            glyph: "",
            reads: talk.reply
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

    // Each orchestrator not running as a row whose Return starts its
    // session, for Agents' Start a Session, named as its own row is.
    readonly property var startable: {
        Sessions.present;
        return listed.filter(one => standing(Sessions.present[one.address] ?? null) === "off")
            .map(one => thing({
                key: `orchestrator:start:${one.address}`,
                verb: Words.verb("verb:orchestrator:resume"),
                name: one.short || one.title || one.address,
                glyph: Theme.glyph.agent,
                hue: "program",
                note: one.title ?? "",
                run: () => provider.start(one)
            }));
    }

    // A closed orchestrator resumed where it left off: its quest's last
    // session, in the folder that session ran in (Joel, 2026-10-28: "i
    // want to be able to easily restart an orchestrator that is closed,
    // continuing from where it left off"); one with no session yet is
    // started on its quest, in the folder the quest index says a new one
    // starts in, never the quest's own.
    function start(one) {
        const folder = startFolders[one.quest] ?? Sessions.home;
        const last = lastSessions[one.quest];
        if (last)
            Sessions.resume(last, one.quest, folder);
        else
            Sessions.start(one.quest, folder);
    }

    function everything() {
        return all;
    }

    // Found by its name, or by its quest's title, best first.
    function search(typed) {
        const words = Fuzzy.words(typed);
        const found = [];
        for (const row of all) {
            const byName = Fuzzy.match(row.prepared, words, false);
            const byTitle = row.note !== "" ? Fuzzy.match(Fuzzy.kept(row.note), words, false)
                : byName;
            const best = byName.score >= byTitle.score ? byName : byTitle;
            if (best.score >= 0)
                found.push({ score: best.score, grade: best.grade, row: row });
        }
        found.sort((a, b) => b.score - a.score);
        return { rows: found.map(f => f.row), grade: 0 };
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
                const last = {};
                for (const quest of JSON.parse(text()).quests ?? []) {
                    if (quest.start)
                        out[quest.id] = quest.start;
                    if ((quest.sessions ?? []).length > 0)
                        last[quest.id] = quest.sessions[quest.sessions.length - 1];
                }
                provider.startFolders = out;
                provider.lastSessions = last;
            } catch (e) {
                provider.startFolders = {};
                provider.lastSessions = {};
            }
        }
    }
}
