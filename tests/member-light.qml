// What a collection's row costs in memory for its name made ready to match
// only once it's read, run in Qt's own JavaScript engine, the one the shell
// runs it in. The `the-launcher-makes-rows-lightly` check copies
// launcher/Member.js beside this file and runs `qml -platform offscreen
// member-light.qml` in a fresh process, with QML_XHR_ALLOW_FILE_READ set so
// it can read its own /proc. Every ready row the shell keeps is made so,
// about 6,000 of them on Joel's, so what each one's getter keeps alive is
// the shell's. Qt's engine keeps every argument of a function alive with
// any closure made in it (qv4compilerscanfunctions.cpp's
// argumentsCanEscape), so a getter made in Member.made kept the defaults
// and fields it was given with every row: 920 to 1,190 bytes over the
// row's own fields, where one made in a function holding only the row and
// its preparing takes 490 to 690 (2026-11-13).
import QtQuick
import QtQuick.Window
import "Member.js" as Member
import "Memory.js" as Memory

Window {
    visible: false

    // How many rows are kept, about as many as Joel's shell keeps ready.
    readonly property int rows: 20000
    // The most the getter may add to a row over its own fields, in bytes.
    readonly property int most: 800

    // A provider's defaults, as Provider.qml's member() gives them.
    function defaults() {
        return { what: "emoji", chord: [], side: "", count: "", context: "", note: "",
            below: null, verbs: null, plain: false, later: false, now: false,
            keywords: null, weight: 1 };
    }

    // A row's own fields, named as ReadyRows.qml's thing() names them.
    function fields(at) {
        const names = ["key", "thing", "name", "glyph", "hue", "side", "sideItems", "note",
            "graph", "reads", "state", "unfinished", "sure", "gone", "drawer", "drawerUnread",
            "verb", "run", "actsOnly", "stays", "now", "later", "wordsId", "verbs", "targets",
            "keysAlike", "keywords"];
        const made = {};
        for (const name of names)
            made[name] = name === "name" ? `row ${at}` : name === "sideItems" ? [] : at;
        return made;
    }

    // What a row made by `make` adds to the process, in bytes, its
    // garbage collected twice over so none made during a collection
    // under way is left.
    function weighed(make) {
        gc();
        gc();
        const before = Memory.anonymous();
        let kept = [];
        for (let at = 0; at < rows; at++)
            kept.push(make(at));
        gc();
        gc();
        const each = Math.round((Memory.anonymous() - before) / rows);
        kept = null;
        gc();
        gc();
        return each;
    }

    // A test that throws ends as a failure, rather than leaving qml running.
    Component.onCompleted: {
        try {
            run();
        } catch (error) {
            console.warn(`the test threw: ${error}`);
            Qt.exit(1);
        }
    }

    function run() {
        const plain = weighed(at => Object.assign({}, defaults(), fields(at)));
        const made = weighed(at => Member.made(defaults(), fields(at), name => name));
        let preparings = 0;
        const read = Member.made(defaults(), fields(7), name => {
            preparings += 1;
            return `${name} ready`;
        });
        if (preparings !== 0 || read.prepared !== "row 7 ready" || read.prepared === null
                || preparings !== 1) {
            console.warn(`FAIL a row's name read ready as ${read.prepared}, made`
                + ` ${preparings} times where it's made once, as it's first read`);
            Qt.exit(1);
            return;
        }
        const getter = made - plain;
        console.info(`${rows} rows kept, ${plain} bytes each plain, ${made} made`);
        if (getter > most) {
            console.warn(`FAIL a made row took ${getter} bytes over its fields, more than ${most}`);
            Qt.exit(1);
            return;
        }
        Qt.exit(0);
    }
}
