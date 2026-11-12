// A collection's row as launcher/Member.js makes it, run in Qt's own
// JavaScript engine, the one the shell runs it in. The
// `a-row-made-again-keeps-every-field` check copies Member.js into a
// launcher folder beside this file, and tests/lib/Expect.js beside it, and
// runs `qml -platform offscreen member.qml`.
//
// For You makes a session's row again from Agents' row for it, which
// carries the getter its name is matched through. Copied, that getter
// became a plain field, and making the row turned it back into a getter
// in place: Qt 6.12 then kept the getter's slot over the next field of
// every row grown from the same shape, so a field read as nothing, or a
// write went past the row's end, and the shell crashed three times at
// 23:05 on 2026-11-11, each just after For You opened.
import QtQuick
import "launcher/Member.js" as Member
import "Expect.js" as Expect

QtObject {
    // The provider's defaults, as Provider.qml's member() gives them.
    function defaults() {
        return {
            what: "session",
            side: "",
            count: "",
            note: "",
            verbs: null,
            later: false
        };
    }

    function made(fields) {
        return Member.made(defaults(), fields, name => `ready ${name}`);
    }

    // Rows made again from rows made, each with more fields than the one
    // before, so their shapes grow from one another as For You's do.
    function madeAgain() {
        const extras = ["forYou", "returns", "verb", "stays", "targets", "run", "drawer"];
        const wrong = [];
        for (let n = 0; n < 60; n++) {
            const first = made({ name: `session ${n}`, key: `agents:${n}`, title: "Asked" });
            const given = { key: `on:${n}` };
            for (const extra of extras.slice(0, n % (extras.length + 1)))
                given[extra] = `${extra} ${n}`;
            const again = made(Object.assign({}, first, given));
            for (const name of Object.keys(given)) {
                if (again[name] !== given[name])
                    wrong.push(`${n} ${name}: ${again[name]}`);
            }
            if (again.title !== "Asked" || again.name !== `session ${n}`)
                wrong.push(`${n} title ${again.title}, name ${again.name}`);
        }
        Expect.same("a row made again from a row made keeps every field it's given",
            wrong.slice(0, 6), []);
    }

    // The name is made ready from the row's own name, once, when it's read.
    function preparing() {
        let asked = 0;
        const row = Member.made(defaults(), { name: "Ada" }, name => {
            asked += 1;
            return `ready ${name}`;
        });
        Expect.same("a row's name isn't made ready until it's read", asked, 0);
        Expect.same("a row's name is made ready from its own name", row.prepared, "ready Ada");
        Expect.same("a row's name is made ready once", [row.prepared, asked], ["ready Ada", 1]);
        const again = made(Object.assign({}, row, { name: "Grace" }));
        Expect.same("a row made again makes its own name ready", again.prepared,
            "ready Grace");
    }

    // The fields given win over the defaults, and a default stays where
    // none is given.
    function defaulting() {
        const row = made({ name: "Ada", side: "busy" });
        Expect.same("a field given wins over its default", [row.side, row.later],
            ["busy", false]);
    }

    Component.onCompleted: {
        try {
            madeAgain();
            preparing();
            defaulting();
            Qt.exit(0);
        } catch (error) {
            Expect.fail(`${error}`);
        }
    }
}
