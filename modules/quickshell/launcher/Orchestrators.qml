// The orchestrators, a collection in Agents: each quest whose session
// orchestrates others, as the harness lists them (orchestrators.json under
// its quest workflow, written whole whenever one changes), so Joel can pick
// one and talk to it (Joel, 2026-10-25: "it needs a good name and should
// allow me to pick an orchestrator"). One waiting on him comes first, then
// those listening, those busy, those paused and those not running, each
// saying so beside its name, as its session's presence says.
//
// Return or the right arrow steps into one, to what can be done to it:
// Talk, which asks for words in the field and leaves them in its session's
// steer inbox, as the person's own message; Open its window; Pause or
// Resume; and Stop, which ends the run in progress and leaves it
// listening. One not running offers Start a session on its quest instead.
// What's written is a file renamed into the inbox, so its session takes it
// whole; nothing waits on it here, and nothing is read for a key.
//
// After its actions come the talks Joel has had with it, newest first, as
// its session writes what became of each (the harness's talk files, one a
// talk, under the inbox's replies): his words, and beside them how it went,
// in one word that changes in place (delivered, reading, replying,
// replied, not delivered). The right arrow reads one: his words, how it
// went, and the reply below. The newest 20 are read, and watched while the
// shell runs, so a reply being written is seen as it's written.
pragma ComponentBehavior: Bound
import qs
import Qt.labs.folderlistmodel
import Quickshell
import Quickshell.Io
import QtQuick

Provider {
    id: provider

    what: Words.of("orchestrators").what

    collection: Words.collection("orchestrators")

    // As the harness wrote them, or none before it has.
    property var listed: []
    // Where each quest starts a session, as the quest index says.
    property var starts: ({})

    readonly property string harness: Sessions.state
    readonly property string inboxes: `${harness}/steer`

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

    // Its session's window, by its address, or null.
    function windowOf(address) {
        const session = Sessions.running.find(one => one.address === address) ?? null;
        return session ? session.window : null;
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

    function made(one, stands) {
        const presence = Sessions.present[one.address] ?? null;
        return member({
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

    function verbs(one, stands) {
        if (stands === "off")
            return [act("start", { run: () => provider.start(one) })];
        const window = windowOf(one.address);
        const rows = [act("talk", {
            ask: Words.of("verb:orchestrator:talk").ask ?? "",
            chip: one.short || one.address,
            state: () => provider.told,
            given: words => provider.talk(one, words)
        })];
        if (window)
            rows.push(act("open", { run: () => Niri.act({ FocusWindow: { id: window.id } }) }));
        rows.push(stands === "paused"
            ? act("resume", { run: () => provider.steer(one, { act: "resume" }) })
            : act("pause", { run: () => provider.steer(one, { act: "pause" }) }));
        if (stands === "busy")
            rows.push(act("stop", { run: () => provider.steer(one, { act: "stop" }) }));
        return rows.concat(talksOf(one.address));
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

    function talksOf(address) {
        return (talks[address] ?? []).filter(talk => talk.words !== "").map(talk => thing({
            key: `talk:${address}:${talk.id}`,
            verb: "read",
            name: talk.words,
            glyph: Theme.glyph[stageMarks[talk.stage] ?? "pending"],
            hue: "voice",
            side: stageSaid(talk),
            reads: talk.reply !== "" ? talk.reply : talk.why,
            nested: true,
            verbs: () => provider.talkRows(address, talk.id)
        }));
    }

    // A talk read: his words, how it went, and what came of it, each a row
    // Return copies, the reply below the rows.
    function talkRows(address, id) {
        const talk = (talks[address] ?? []).find(one => one.id === id);
        if (!talk)
            return [];
        const said = Words.of("orchestrators:talk").said ?? {};
        const row = (key, name, side, copy) => copyRow({
            key: `talk:${id}:${key}`,
            name: name,
            side: side,
            copy: copy,
            glyph: "",
            reads: talk.reply
        });
        return [
            row("words", said.yours ?? "", talk.words, talk.words),
            row("stage", said.went ?? "", stageSaid(talk),
                talk.reply !== "" ? talk.reply : talk.why),
            talk.came !== "" ? row("came", said.came ?? "", talk.came, talk.came) : null
        ].filter(one => one !== null);
    }

    // What the foot says once words have gone.
    property string told: ""
    // How many times the launcher has opened, so the talks are listed
    // again each time.
    property int looked: 0

    function talk(one, words) {
        const text = words.trim();
        if (text === "")
            return;
        // An id opens a talk its session writes what became of.
        steer(one, { text: text, id: `talk-${Date.now()}` });
        told = (Words.of("orchestrators").said ?? {}).sent?.replace("{name}", one.short
            || one.address) ?? "";
    }

    // Leaves a steer in its session's inbox, under a name it skips until
    // it's whole.
    function steer(one, fields) {
        const name = `${Date.now()}-${Math.random().toString(36).slice(2, 10)}.json`;
        const body = JSON.stringify(Object.assign({ at: new Date().toISOString(), from: "joel" },
            fields));
        Niri.spawn(["sh", "-c",
            'mkdir -p "$1" && printf %s "$3" > "$1/$2.tmp" && mv "$1/$2.tmp" "$1/$2"',
            "steer", `${inboxes}/${one.address.toUpperCase()}`, name, body]);
    }

    // A session on its quest, in the folder the quest index says a new one
    // starts in, never the quest's own.
    function start(one) {
        Niri.spawn(["kitty", "--directory", starts[one.quest] ?? Sessions.home, "env",
            `QUEST_WORKFLOW_AUTOLOAD_ID=${one.quest}`, "pi"]);
    }

    function everything() {
        return all;
    }

    // Each orchestrator's talks: its replies folder listed, and its newest
    // files read and watched.
    Instantiator {
        model: provider.listed

        delegate: Scope {
            id: inbox

            required property var modelData
            readonly property string address: String(modelData.address ?? "").toUpperCase()
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
                    required property var modelData
                    path: String(modelData)
                    watchChanges: true
                    printErrors: false
                    onFileChanged: reload()
                    onLoaded: provider.heardTalk(inbox.address, path, text())
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
                for (const quest of JSON.parse(text()).quests ?? [])
                    if (quest.start)
                        out[quest.id] = quest.start;
                provider.starts = out;
            } catch (e) {
                provider.starts = {};
            }
        }
    }
}
