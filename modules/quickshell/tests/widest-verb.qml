// Which verb the foot keeps room for after Return, run in Qt's own
// JavaScript engine, the one the shell runs it in. The
// `the-foot-weighs-each-verb-once` check copies
// modules/quickshell/launcher/WidestVerb.js and tests/lib/Expect.js beside
// this file and runs `qml -platform offscreen widest-verb.qml`. Each
// distinct verb, a state's included, is measured once; the widest is kept,
// the earlier where two are as wide; "connect" is the least; and a verb
// that isn't a string is left out.
import QtQuick
import "Expect.js" as Expect
import "WidestVerb.js" as WidestVerb

QtObject {
    Component.onCompleted: Expect.run(cases)

    // A measure that counts what it's asked, each verb as wide as it's
    // long, so a case can say how many times each verb was measured.
    function counting() {
        const asked = {};
        return {
            asked,
            measure: verb => {
                asked[verb] = (asked[verb] ?? 0) + 1;
                return verb.length;
            }
        };
    }

    // The first opening measured every thing's verb twice, once for it and
    // once for the widest so far, where most things share a handful
    // (2026-11-27).
    function measuresEachDistinctVerbOnce() {
        const tally = counting();
        const things = [{ verb: "open" }, { verb: "open" }, { verb: "brighten" },
            { verb: "open", states: { on: { verb: "brighten" }, off: { verb: "open" } } },
            { verb: "connect" }];
        WidestVerb.among(things, tally.measure);
        Expect.same("each verb measured once", tally.asked,
            { connect: 1, open: 1, brighten: 1 });
    }

    function keepsTheWidest() {
        const things = [{ verb: "open" }, { verb: "brighten" }, { verb: "run" }];
        Expect.same("the widest kept", WidestVerb.among(things, counting().measure),
            "brighten");
    }

    // Two as wide keep the one named first, as the room did before.
    function keepsTheEarlierOfTwoAsWide() {
        const things = [{ verb: "lighten" }, { verb: "brighten" }, { verb: "darkened" },
            { verb: "dimming" }];
        Expect.same("the first of the widest", WidestVerb.among(things, counting().measure),
            "brighten");
    }

    // A network's source says "connect", so the room never holds less.
    function connectIsTheLeast() {
        const things = [{ verb: "open" }, { verb: "run" }];
        Expect.same("narrower verbs keep connect", WidestVerb.among(things,
            counting().measure), "connect");
        Expect.same("no things keep connect", WidestVerb.among([], counting().measure),
            "connect");
        Expect.same("as wide as connect keeps connect", WidestVerb.among(
            [{ verb: "connects" }], verb => (verb === "connects" ? 7 : verb.length)),
            "connect");
    }

    // A state's verb is drawn after Return as the thing's own is.
    function countsStatesVerbs() {
        const things = [{ verb: "open", states: { playing: { verb: "pause playing" } } }];
        Expect.same("a state's verb kept", WidestVerb.among(things, counting().measure),
            "pause playing");
    }

    // A verb that isn't a string is never measured or kept.
    function skipsAVerbThatIsNoString() {
        const tally = counting();
        const things = [{ verb: 12345678910 }, { name: "no verb" },
            { verb: "open", states: { on: { verb: null }, off: {} } }];
        Expect.same("connect kept", WidestVerb.among(things, tally.measure), "connect");
        Expect.same("only strings measured", tally.asked, { connect: 1, open: 1 });
    }

    function cases() {
        measuresEachDistinctVerbOnce();
        keepsTheWidest();
        keepsTheEarlierOfTwoAsWide();
        connectIsTheLeast();
        countsStatesVerbs();
        skipsAVerbThatIsNoString();
    }
}
