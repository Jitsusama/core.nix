// How the shell's client of a daemon mends a record it holds with a change
// saying only what differs, run in Qt's own JavaScript engine, the one the
// shell runs it in. The `a-change-mends-only-what-moved` check copies
// modules/quickshell/Mending.js and tests/lib/Expect.js beside this file
// and runs it offscreen. A field said replaces the held one, and one said
// as null goes; a field said deep, an object both held and said, is
// mended one level in, what didn't move inside it kept as the very object
// it was; one not said deep, or not an object both times, comes whole; and
// what was held is never changed. A kind's fields said deep are those its
// watch asked that the daemon's hello offers, none from a daemon offering
// nothing.
import QtQuick
import "Expect.js" as Expect
import "Mending.js" as Mending

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

    function held() {
        return {
            key: "session:kestrel",
            step: "read",
            standing: "running",
            presence: { at: 1, cwd: "~/src/skiff", owner: "kestrel", landings: [{ what: "Tide" }] }
        };
    }

    function cases() {
        const was = held();
        const change = {
            key: "session:kestrel",
            step: null,
            standing: 3,
            presence: { at: 2, owner: null, busy: true }
        };

        const deep = Mending.mended(was, change, ["presence", "standing"]);
        Expect.same("a field said replaces the held one, one said as null goes, and inside a "
            + "field said deep only what moved is laid over",
            deep,
            {
                key: "session:kestrel",
                standing: 3,
                presence: { at: 2, cwd: "~/src/skiff", landings: [{ what: "Tide" }], busy: true }
            });
        Expect.expect("what didn't move inside a field said deep is the very object it was",
            deep.presence.landings === was.presence.landings,
            "its landings were copied, so comparing them costs a walk");

        Expect.same("a field not said deep comes whole",
            Mending.mended(was, change, []).presence, { at: 2, owner: null, busy: true });
        Expect.same("a field said deep that wasn't held as an object comes whole",
            Mending.mended({ key: "session:kestrel" }, { presence: { at: 2 } }, ["presence"]),
            { key: "session:kestrel", presence: { at: 2 } });
        Expect.same("a field said deep as a list comes whole",
            Mending.mended({ presence: { at: 1 } }, { presence: [1, 2] }, ["presence"]),
            { presence: [1, 2] });
        Expect.same("what was held is never changed", was, held());

        const offered = { session: { live: ["presence"], deep: ["presence"] }, machine: {} };
        Expect.same("a kind's fields said deep are those asked that its hello offers",
            Mending.deepByKind(offered, ["presence", "step"]), { session: ["presence"] });
        Expect.same("a daemon offering nothing says nothing deep",
            Mending.deepByKind(undefined, ["presence"]), {});
        Expect.same("a watch asking nothing deep is said nothing deep",
            Mending.deepByKind(offered, undefined), {});
        // Parsed, since a literal's `__proto__` sets what it inherits rather than a key.
        const odd = Mending.deepByKind(JSON.parse(
            '{"__proto__": {"deep": ["presence"]}, "constructor": {"deep": ["step"]}}'),
            ["presence", "step"]);
        Expect.same("a kind the hello names as what every object inherits is a kind of its own",
            [Mending.deepOf(odd, { key: "__proto__:kestrel" }),
                Mending.deepOf(odd, { key: "machine:tern", kind: "constructor" }),
                Mending.deepOf(odd, { key: "toString" })],
            [["presence"], ["step"], []]);

        const by = { session: ["presence"] };
        Expect.same("a change's kind is the one it says",
            Mending.deepOf(by, { key: "machine:tern", kind: "session" }), ["presence"]);
        Expect.same("or else its key's before the first colon",
            Mending.deepOf(by, { key: "session:kestrel:two" }), ["presence"]);
        Expect.same("a kind offering nothing deep has nothing deep",
            Mending.deepOf(by, { key: "machine:tern" }), []);
        Expect.same("a key naming what a plain object inherits has nothing deep",
            ["constructor", "toString", "__proto__", "hasOwnProperty:kestrel"]
                .map(key => Mending.deepOf(by, { key })),
            [[], [], [], []]);
        Expect.same("and a change to it is mended whole",
            Mending.mended({ key: "constructor", step: 1 },
                { key: "constructor", step: 2 }, Mending.deepOf(by, { key: "constructor" })),
            { key: "constructor", step: 2 });
    }
}
