// The safe word in For You: each stop that stands, each saying that
// nearly was one (a joke, an ask without the phrase, the phrase without
// Joel's tag) and each ask to resume, as `pineapple serve` serves them
// (modules/pineapple/src/serve.rs, tell.rs). The daemon serves only live
// ones, so nothing shows while it's in shadow. Its rows are To Do's, as
// Sound's offer is, read whole from the daemon's records, which hold every
// fact and a title in the launcher's own words; nothing here words a stop.
// Return opens where it was said; its verbs ask the daemon back on the
// same socket, and the daemon acts only for this shell's own process and
// only on a row it serves now, so a press here is the one way in.
import qs
import Quickshell
import QtQuick
import "ForYou.js" as ForYou
import "../Confirm.js" as Confirm

Provider {
    id: provider

    what: "pineapple"

    Daemon {
        id: daemon
        socket: `${Quickshell.env("XDG_RUNTIME_DIR")}/daemons/pineapple.sock`
        kind: "pineapple"
    }

    // The daemon's rows, as it orders them; For You weighs them by since.
    readonly property var records: {
        void daemon.revision;
        return Object.values(daemon.held)
            .filter(one => one && typeof one.key === "string");
    }

    // A verb the daemon offers on a row: its words the launcher's, by the
    // act's key, and Return on it the daemon's ask. One the daemon serves
    // costly, a resume, which undoes someone's safe word, arms on its first
    // Return, saying what a second does, as secret asks' Release It does;
    // a command's flags can't say it, since resume's are every Resume's.
    // What the daemon refused is said in the foot, since the row it was on
    // stays and nothing else would show why nothing moved, in the
    // vocabulary's words for its code: its own message is for its journal.
    function verbOf(act) {
        const arms = act.costly === true ? Words.of(act.key).said?.arms ?? "" : "";
        return thing(Object.assign({
            key: act.key,
            name: Words.name(act.key),
            verb: Words.verb(act.key),
            glyph: Words.glyph(act.key),
            hue: "action",
            arms,
            run: () => daemon.ask(act.ask, answer => {
                if (answer?.error)
                    provider.refused(act, answer.error);
            })
        }, Confirm.asking(act)));
    }

    // Says in the foot that the daemon refused a press, and why, as Joel
    // would say it; a code with no words of its own is said as a failure.
    function refused(act, error) {
        const said = Words.of("for-you:source:pineapple").said ?? {};
        const why = said[`refused:${error.code}`] ?? said["refused:failed"];
        console.warn(`pineapple refused ${act.key}: ${error.message ?? ""}`);
        provider.launcher?.tellOnOpen([[Words.name("for-you:source:pineapple"),
            ForYou.filled(said.refused, { verb: Words.name(act.key), why }), Theme.alert]]);
    }

    function drawerOf(one) {
        const now = Date.now();
        const say = Object.assign({}, Words.of("on:waiting").said ?? {}, {
            clock: at => Qt.formatDateTime(new Date(at), "ddd d MMM, HH:mm")
        });
        const told = Words.of("for-you:source:pineapple").said ?? {};
        const fact = (id, value) => ({ label: Words.name(id), value: String(value ?? "") });
        const count = Number(one.count ?? 1);
        return {
            eyebrow: ForYou.filled(say.from, { from: Words.name("for-you:source:pineapple"),
                kind: say.todo }),
            eyebrowNames: true,
            title: String(one.title ?? ""),
            facts: [
                fact("pineapple:fact:said", one.words),
                fact("pineapple:fact:where", one.link),
                fact("on:waiting:since", one.since ? say.clock(one.since) : ""),
                fact("pineapple:fact:count", count > 1 ? ForYou.filled(told.count, { count }) : "")
            ].filter(each => each.value !== ""),
            body: String((Words.of("for-you:source:pineapple").tells ?? {})[one.what] ?? "")
        };
    }

    function rowOf(one) {
        const verbs = (Array.isArray(one.acts) ? one.acts : []).map(act => provider.verbOf(act));
        return thing({
            key: `on:waiting:${one.key}`,
            what: "on",
            forYou: { kind: "do", source: "pineapple" },
            name: String(one.title ?? ""),
            verb: Words.verb("for-you:source:pineapple"),
            glyph: Words.glyph("for-you:source:pineapple"),
            hue: one.what === "stop" ? "strong" : "action",
            stays: true,
            drawer: () => provider.drawerOf(one),
            verbs: () => verbs,
            targets: () => verbs,
            run: () => {
                if (typeof one.link === "string" && one.link !== "")
                    Niri.spawn(["xdg-open", one.link]);
            }
        });
    }

    // Pause All Agents, found by its name from the launcher's top as Reboot
    // is (WCJQSG, 2026-11-19 [1c8863e1]: for Joel's demo): Joel's own stop on
    // every agent, here and on each machine that stops with this one, asked
    // of the daemon on this socket, which takes it only from this shell's
    // own process (src/serve.rs's paused). Costly, it arms on its first
    // Return, saying what a second does, as Reboot's does, and the : line
    // asks once, as it asks Reboot. Found only by what's typed, it's in no
    // place a walk from outside goes, so a terminal's `launcher do` never
    // finds it, which matters since serve can tell only that this shell
    // asked. It stays while the daemon stops them, a few seconds, saying so
    // at once, then says in the foot what stopped, or why nothing did, in
    // the vocabulary's words for its code; the stop's own row in For You,
    // with Resume, follows as any stop's.
    readonly property var pauseAll: thing({
        key: "agents:pause-all",
        name: Words.name("agents:pause-all"),
        verb: Words.verb("agents:pause-all"),
        glyph: Words.glyph("agents:pause-all"),
        hue: "action",
        costly: true,
        keywords: (Words.of("agents:pause-all").words ?? []).join(" "),
        arms: Words.of("agents:pause-all").said?.arms ?? "",
        confirm: true,
        asked: () => Words.of("agents:pause-all").asked ?? "",
        stays: true,
        run: () => provider.pauseEveryAgent()
    })

    function pauseEveryAgent() {
        // Never sent is told from sent and lost: only the first is sure to
        // have stopped nothing.
        if (daemon.wire === null) {
            provider.refused({ key: "agents:pause-all" }, { code: "unreached" });
            return;
        }
        provider.launcher?.tellOnOpen([[Words.name("agents:pause-all"),
            Words.of("agents:pause-all").said?.pausing ?? "", Theme.strong]]);
        daemon.ask({ ask: "pause", scope: "all" }, answer => provider.paused(answer));
    }

    things: [pauseAll]

    // Says in the foot what the daemon answered Pause All Agents: what it
    // stopped, in its own line, or why it stopped nothing. No answer at all
    // (null) means the connection went after the ask was sent, so it may
    // have stopped them, and Joel is pointed at For You rather than told
    // it didn't. A code these words don't know, as a serve too old to take
    // a pause answers, says it couldn't be asked here.
    function paused(answer) {
        if (answer === null) {
            provider.refused({ key: "agents:pause-all" }, { code: "lost" });
            return;
        }
        if (answer.error) {
            const told = Words.of("for-you:source:pineapple").said ?? {};
            const known = told[`refused:${answer.error.code}`] !== undefined;
            provider.refused({ key: "agents:pause-all" },
                known ? answer.error : { code: "unoffered", message: answer.error.message });
            return;
        }
        const said = typeof answer.said === "string" && answer.said !== "" ? answer.said
            : Words.of("agents:pause-all").said?.pausing ?? "";
        provider.launcher?.tellOnOpen([[Words.name("agents:pause-all"),
            said, Theme.success]]);
    }

    // What For You is given (ForYou.js's given): each row a thing to do now,
    // weighed by how long it has waited and, for a stop, by the sessions it
    // parked, which wait on Joel as a session's want does.
    readonly property var forYou: records.map(one => ({
        row: provider.rowOf(one),
        thing: { kind: "todo", ready: true, takes: 1, since: one.since ?? null,
            blocks: Number(one.parked ?? 0) }
    }))
}
