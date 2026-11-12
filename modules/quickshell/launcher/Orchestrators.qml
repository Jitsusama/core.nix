// What Agents knows of the orchestrators, and no place of its own: each
// quest whose session orchestrates others, as the harness lists them
// (orchestrators.json under its quest workflow, written whole whenever one
// changes), which Ask and Quests read, and how a closed one reopens. One
// running or parked is its session, a row of Agents' tree like any other;
// one closed stands dimmed at the tree's end, as Agents draws it from the
// agents daemon's records, which say what reopening it needs, the one last
// live first (VKCF3Z, 2026-11-07 [9a4b1dd0]: the Orchestrators collection,
// a second listing of the same sessions, goes).
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

Provider {
    id: provider

    // As the harness wrote them, or none before it has.
    property var listed: []

    readonly property string harness: Sessions.state

    // How a closed one's folder is gone, as the daemon last looked (its
    // record's `gone`): what its row says, and the start afresh in its
    // quest's start folder where that's there, or null while its folder is
    // there, so it says so before Return (VKCF3Z, 2026-11-07 [4a00987e]).
    function goneFrom(one) {
        if (!one.gone)
            return null;
        const folder = one.gone.start ?? "";
        const said = Words.of("verb:orchestrator:afresh").said ?? {};
        return {
            side: String(said[folder !== "" ? "gone" : "nowhere"] ?? "")
                .replace("{cwd}", homely(one.gone.cwd ?? ""))
                .replace("{folder}", homely(folder)),
            afresh: folder !== "" ? () => Sessions.start(one.quest, folder) : null
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

    // A closed orchestrator reopened where it left off, from the daemon's
    // record of it: the session its quest names to resume, in the folder
    // that session ran in (Joel, 2026-10-28: "i want to be able to easily
    // restart an orchestrator that is closed, continuing from where it
    // left off"); one with no session yet is started on its quest, in the
    // folder its quest says a new one starts in, never the quest's own.
    // Never by the quest's `sessions`, which can hold one long dead whose
    // pid was used again (Joel's Aquifer orchestrator, 2026-10-29). When
    // the session's folder went since it was listed, it starts afresh in
    // that start folder, never the home folder, and the foot says so once
    // the launcher opens next, never a notice.
    function start(one) {
        const last = one.resume ?? "";
        const folder = one.start ?? "";
        if (last === "") {
            Sessions.start(one.quest, folder || Sessions.home);
            return;
        }
        const name = one.name;
        const said = Words.of("verb:orchestrator:reopen").said ?? {};
        const told = (cwd, startedIn) => startedIn !== ""
            ? [name, String(said.afresh ?? "").replace("{folder}", homely(startedIn))]
            : cwd === "" ? [name, String(said.unfound ?? ""), Theme.alert]
            : [homely(cwd), String(said.isGone ?? ""), Theme.alert];
        Sessions.resume(last, one.quest, folder, (cwd, startedIn) =>
            provider.launcher?.tellOnOpen([told(cwd, startedIn)]));
    }

    // Read again as the launcher opens too, since a file that wasn't there
    // when the shell started can't be watched until it's read.
    Connections {
        target: provider.launcher
        ignoreUnknownSignals: true

        function onOpenChanged() {
            if (provider.launcher.open)
                orchestrators.reload();
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
}
