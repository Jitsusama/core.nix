// How fast a drawer's search runs, in Qt's own JavaScript engine, the
// one the shell runs it in. The `the-drawer-search-keeps-pace` check
// copies DrawerPage.js and Search.js beside this file and runs it in a
// fresh process at a time, until one is fast enough
// (tests/lib/qml-in-time.sh): a process's engine runs the same code by
// as much as half faster or slower than the next, while a slower search
// slows every one.
//
// A search over a drawer of hyphenated names, as Emoji's every part is,
// reads each text by its code rather than a letter at a time, which took
// a median 56 ms a key against 10 (2026-11-20). It's timed as the drawer
// runs it, DrawerPage.matchesIn over laid lines, and held two ways: to a
// multiple of the same lines with no joiners, which need no map, so the
// map stays cheap; and that in turn to a multiple of the plainest search
// there is, toLowerCase and includes over the same words, so a search
// slower whether joined or not is refused too. Each is the fastest of
// five batches of ten, batches of each timed in turn so all see the same
// machine, since a slower one is the builder's other work or the
// collector. Every search must find every place, so one quick from
// finding nothing passes none. The ratios are said every run, to set the
// bounds below from. Timing the finder alone, by code took 2.8 times as
// long as with no joiners on a builder, and a letter at a time 12
// (2026-11-22); timed through DrawerPage's walk over lines, which is the
// same with joiners or none, 3.0 and 6.4.
import QtQuick
import "DrawerPage.js" as Page

QtObject {
    // How many times as long a search over joined words may take as one
    // over the same with none, and that as the plainest search: words
    // with none took 8.6 times the plainest on a builder and 5.6 to 8.1
    // on a laptop, and 14.2 to 14.6 there with their fast path gone, which
    // this only just refuses, while a search a letter at a time took 60
    // (2026-11-23).
    readonly property real joined: 6
    readonly property real plain: 14

    // A case that throws would leave qml waiting on nothing, and the check
    // with it, so a throw fails it as a wrong answer does.
    Component.onCompleted: {
        try {
            judged();
        } catch (error) {
            console.warn(`the test threw: ${error.message}`);
            Qt.exit(1);
        }
    }

    function judged() {
        // Each line holds its names eight times, so the search's own
        // reading outweighs DrawerPage's walk over a line, which is the
        // same with joiners or none and would hide how much slower one is.
        const phrase = (joiner, at) => `Face-${joiner}with-${joiner}tears-${joiner}of-`
            + `${joiner}joy ${at} \u00b7 t-${joiner}shirt \u00b7 wi-${joiner}fi \u00b7 pirate-`
            + `${joiner}flag`;
        const linesWith = joiner => Array.from({ length: 100 }, (_, at) => {
            const text = Array.from({ length: 8 }, () => phrase(joiner, at)).join(" \u00b7 ");
            return { text, length: text.length,
                parts: [{ words: {}, length: text.length, on: null }] };
        });
        const withJoiners = linesWith("\u2060");
        const withNone = linesWith("");
        const counts = new Set();
        // The plainest search runs many more rounds a batch, so its batch
        // is long beside the clock's millisecond too: each is said a round.
        const timed = (search, rounds) => {
            const began = Date.now();
            for (let round = 0; round < rounds; round += 1) {
                counts.add(search());
            }
            return (Date.now() - began) / rounds;
        };
        const rounds = [10, 10, 200];
        const paged = lines => () => Page.matchesIn(lines, "shirt").length;
        const plainest = () => withNone.reduce((count, line) => {
            const lowered = line.text.toLowerCase();
            let found = count;
            let at = lowered.indexOf("shirt");
            while (at >= 0) {
                found += 1;
                at = lowered.indexOf("shirt", at + 5);
            }
            return found;
        }, 0);
        const fastest = [Infinity, Infinity, Infinity];
        for (let pass = 0; pass < 5; pass += 1) {
            [paged(withJoiners), paged(withNone), plainest].forEach((search, at) => {
                fastest[at] = Math.min(fastest[at], timed(search, rounds[at]));
            });
        }
        // A batch inside the clock's millisecond would make any ratio, so
        // none is taken as quicker than one.
        const [byCode, bare, floor] = fastest.map((ms, at) => Math.max(ms, 1 / rounds[at]));
        const failures = [];
        const slow = [];
        if (counts.size !== 1 || !counts.has(800)) {
            failures.push(`a search found ${[...counts].join(", ")} places, not 800`);
        }
        const overBare = byCode / bare;
        const overFloor = bare / floor;
        console.warn(`joined words took ${byCode.toFixed(2)} ms, ${overBare.toFixed(1)} times`
            + ` the ${bare.toFixed(2)} ms with none, ${overFloor.toFixed(1)} times the`
            + ` plainest ${floor.toFixed(3)} ms`);
        if (overBare > joined) {
            slow.push(`joined words took ${overBare.toFixed(1)} times as long, `
                + `more than ${joined}`);
        }
        if (overFloor > plain) {
            slow.push(`words with none took ${overFloor.toFixed(1)} times the plainest search, `
                + `more than ${plain}`);
        }
        for (const failure of failures.concat(slow)) {
            console.warn(failure);
        }
        Qt.exit(failures.length > 0 ? 1 : slow.length > 0 ? 2 : 0);
    }
}
