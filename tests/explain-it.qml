// Explain It, the act that says in plain words what the session behind a
// row works on, run in Qt's own JavaScript engine, the one the shell runs it
// in. The `explain-it-asks-only-what-a-cell-is` check copies
// modules/quickshell/launcher/Explaining.js and tests/lib/Expect.js beside
// this file and runs it offscreen.
import QtQuick
import "Expect.js" as Expect
import "launcher/Explaining.js" as Explaining

QtObject {
    // A case that doesn't hold throws, and ends the test as a failure.
    Component.onCompleted: {
        try {
            cases();
            Qt.exit(0);
        } catch (error) {
            Expect.fail(error.message ?? String(error));
        }
    }

    // A session as `mycelium cells <ADDRESS> -o json` says one, cut short.
    readonly property var cell: ({
        kind: "cell",
        name: "HARBOR",
        type: "session",
        title: "Tidy the Lighthouse's Rows",
        id: "01a1e64b-1835-7563-a889-c1011fd09c8a",
        window: "π - Tidy The Lighthouse's Rows - lighthouse",
        standing: "moving",
        doing: "bash",
        repo: "lighthouse",
        usage: { cost_today: 2.5, turns_today: 41 },
        machine: "kestrel",
        since: "2026-11-16T21:37:23.509Z",
        quest: "QEST-20261116-HARBOR",
        plan: {
            at: "2026-11-17T12:53:39.328Z",
            chose: "Rows now fold under their day.",
            next: "Say a row's time beside it.",
            plan: "PLAN-20261116-ROWS01"
        },
        cwd: "/home/j/src/lighthouse",
        trees: [{ key: "snapshot-lighthouse-abc", commit: "abc" }],
        networks: ["orchestration.wcjqsg", "repo.lighthouse"]
    })

    readonly property string asks: "Say what this is.\n\n{facts}\n\nThat's all."

    function cases() {
        // What it asks the node.
        Expect.same("a session's address is asked of the realm's node as JSON",
            Explaining.command("harbour", "HARBOR"),
            ["mycelium", "--realm", "harbour", "cells", "HARBOR", "-o", "json"]);
        Expect.same("an address with digits is asked too, as the notices daemon reads one",
            Explaining.command("harbour", "1C8A2F0E"),
            ["mycelium", "--realm", "harbour", "cells", "1C8A2F0E", "-o", "json"]);
        Expect.same("an address in small letters isn't one, so nothing is asked",
            Explaining.command("harbour", "harbor"), null);
        Expect.same("an address past twelve isn't one",
            Explaining.command("harbour", "ABCDEFGHIJKLM"), null);
        Expect.same("an address opening with a dash would read as an option",
            Explaining.command("harbour", "-all"), null);
        Expect.same("no address asks nothing", Explaining.command("harbour", ""), null);
        Expect.same("no realm asks nothing", Explaining.command("", "HARBOR"), null);
        Expect.same("a realm that isn't a name asks nothing",
            Explaining.command("har bour; rm", "HARBOR"), null);

        // What the node's answer is.
        Expect.same("a session's answer is the session",
            Explaining.cellOf(JSON.stringify(cell))?.name, "HARBOR");
        Expect.same("a machine is no session to explain",
            Explaining.cellOf(JSON.stringify({ kind: "cell", name: "kestrel", type: "machine" })),
            null);
        Expect.same("a network is no cell", Explaining.cellOf(JSON.stringify({ kind: "network",
            name: "repo.lighthouse" })), null);
        Expect.same("an answer that isn't JSON is none", Explaining.cellOf("mycelium: no cell"),
            null);
        Expect.same("nothing heard is none", Explaining.cellOf(""), null);

        // What the quick model is told: what the session is, never its ids.
        const facts = Explaining.facts(cell);
        Expect.same("the facts are the work as Joel knows it, a line each", facts, [
            "Its title: Tidy the Lighthouse's Rows",
            "Its repository: lighthouse",
            "Its machine: kestrel",
            "How it stands: moving",
            "Doing now: bash",
            "Running since: 2026-11-16T21:37:23.509Z",
            "Latest in its plan: Rows now fold under their day.",
            "Next in its plan: Say a row's time beside it."
        ]);
        const told = Explaining.message(cell, asks);
        Expect.same("the question holds the facts where its words say",
            told, `Say what this is.\n\n${facts.join("\n")}\n\nThat's all.`);
        for (const hidden of ["QEST-", "PLAN-", "01a1e64b", "snapshot", "/home/j", "2.5",
            "orchestration", "HARBOR"]) {
            Expect.same(`the question names no ${hidden}`, told.includes(hidden), false);
        }
        Expect.same("a session that says little is told as little",
            Explaining.facts({ kind: "cell", name: "HARBOR", type: "session",
                title: "Tidy it", plan: {} }), ["Its title: Tidy it"]);
        Expect.same("words with $ in them stay as written",
            Explaining.message(Object.assign({}, cell, { title: "Cost $& and $1" }), "{facts}")
                .split("\n")[0], "Its title: Cost $& and $1");
        Expect.same("words that hold no place for facts take them at the end",
            Explaining.message({ kind: "cell", name: "H", type: "session", title: "T" }, "Say."),
            "Say.\n\nIts title: T");
    }
}
