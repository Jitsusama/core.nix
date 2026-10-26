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
import qs
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
        return rows;
    }

    // What the foot says once words have gone.
    property string told: ""

    function talk(one, words) {
        const text = words.trim();
        if (text === "")
            return;
        steer(one, { text: text });
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

    // Read again as the launcher opens too, since a file that wasn't there
    // when the shell started can't be watched until it's read.
    Connections {
        target: provider.launcher
        ignoreUnknownSignals: true

        function onOpenChanged() {
            if (provider.launcher.open) {
                orchestrators.reload();
                questIndex.reload();
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
