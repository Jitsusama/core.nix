// Whether a Map and a Set still mean what the language says they do, in
// the Qt the shell is built with, whose tables find string and number
// keys by hash (modules/quickshell/qtdeclarative-maps-find-keys-by-hash.patch).
// The `maps-keep-their-meaning` check runs this as the shell: keys equal
// by value find each other (a string made two ways, 1 and 1.0, +0 and -0,
// NaN as stock Qt finds it), others never do, objects are kept by who they are, the order is
// the order they came in through deletes and clears, a walk sees what it
// adds and skips what it deletes, and a big table finds every key it
// holds and none it doesn't. It exits 1, saying what was wrong, for any.
import Quickshell
import QtQuick

ShellRoot {
    Component.onCompleted: {
        const wrong = [];
        const expect = (what, got, wanted) => {
            if (got !== wanted)
                wrong.push(`${what}: ${String(got)}, not ${String(wanted)}`);
        };

        const map = new Map();
        const one = {};
        const other = {};
        map.set("row:" + "1", "made");
        map.set(1, "one");
        map.set(0, "zero");
        map.set(NaN, "nan");
        map.set(one, "object");
        map.set(true, "true");
        map.set(null, "null");
        map.set(undefined, "undefined");
        expect("a string made another way", map.get(["row", "1"].join(":")), "made");
        expect("1.0 as 1", map.get(1.0), "one");
        expect("a computed 1", map.get(Math.sqrt(1)), "one");
        expect("-0 as +0", map.get(-0), "zero");
        // NaN as Qt 6.12.0 holds it, by its bits: 0 / 0 isn't found.
        expect("NaN as NaN", map.get(NaN), "nan");
        expect("the same object", map.get(one), "object");
        expect("another object", map.has(other), false);
        expect("the string 1 as the number", map.has("1"), false);
        expect("true", map.get(true), "true");
        expect("null", map.get(null), "null");
        expect("undefined", map.get(undefined), "undefined");
        expect("the size", map.size, 8);
        map.set(1, "again");
        expect("set again in place", [...map.keys()].indexOf(1), 1);
        expect("set again's value", map.get(1), "again");

        // Deletes move every later key, so each must still be found after.
        const order = new Map();
        for (let at = 0; at < 50; at += 1)
            order.set(`key:${at}`, at);
        for (let at = 0; at < 50; at += 3)
            order.delete(`key:${at}`);
        order.set("key:0", "back");
        const kept = [...order.keys()];
        const wanted = Array.from({ length: 50 }, (_, at) => at)
            .filter(at => at % 3 !== 0).map(at => `key:${at}`).concat(["key:0"]);
        expect("the order through deletes", kept.join(","), wanted.join(","));
        for (const key of kept) {
            if (!order.has(key))
                wrong.push(`lost ${key} after deletes`);
        }
        expect("a deleted key", order.has("key:3"), false);
        expect("deleting what isn't there", order.delete("key:3"), false);

        // A walk sees what's added as it goes and skips what's deleted.
        const walked = new Set(["a", "b", "c"]);
        const seen = [];
        walked.forEach(key => {
            seen.push(key);
            if (key === "a") {
                walked.delete("b");
                walked.add("d");
            }
        });
        expect("a walk through changes", seen.join(","), "a,c,d");

        walked.clear();
        expect("cleared", walked.size, 0);
        expect("cleared has", walked.has("a"), false);
        walked.add("a");
        expect("filled again", [...walked].join(","), "a");

        // A big table finds every key it holds and none it doesn't.
        const big = new Set();
        for (let at = 0; at < 20000; at += 1)
            big.add(`emoji:${at}`);
        for (let at = 0; at < 20000; at += 1)
            big.add(at);
        let missing = 0;
        for (let at = 0; at < 20000; at += 1) {
            if (!big.has(`emoji:${at}`) || !big.has(at))
                missing += 1;
        }
        expect("big keys found", missing, 0);
        expect("big keys not held", big.has("emoji:20000") || big.has(20000), false);
        expect("big size", big.size, 40000);

        const weak = new WeakMap();
        weak.set(one, 1);
        expect("a weak key", weak.get(one), 1);
        expect("another weak key", weak.has(other), false);

        for (const one of wrong)
            console.warn(`a Map or a Set meant something else: ${one}`);
        // Said once the shell has loaded, as before it nothing hears it.
        Qt.callLater(() => Qt.exit(wrong.length === 0 ? 0 : 1));
    }
}
