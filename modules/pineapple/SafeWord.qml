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
    // act's key, and Return on it the daemon's ask. What the daemon refused
    // is said in the journal, since the row it was on stays until it moves.
    function verbOf(act) {
        return thing({
            key: act.key,
            name: Words.name(act.key),
            verb: Words.verb(act.key),
            glyph: Words.glyph(act.key),
            hue: "action",
            run: () => daemon.ask(act.ask, answer => {
                if (answer?.error)
                    console.warn(`pineapple refused ${act.key}: ${answer.error.message ?? ""}`);
            })
        });
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

    // What For You is given (ForYou.js's given): each row a thing to do now,
    // weighed by how long it has waited.
    readonly property var forYou: records.map(one => ({
        row: provider.rowOf(one),
        thing: { kind: "todo", ready: true, takes: 1, since: one.since ?? null }
    }))
}
