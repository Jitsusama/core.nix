// The launcher's emoji are found by what they're called and what they mean,
// best first, in Qt's own JavaScript engine, as it runs them: each query
// below and the emoji it should put first, and what it should find nothing
// for. Run by the the-launcher-finds-emoji check, beside the built list.
import QtQuick
import "Fuzzy.js" as Fuzzy
import "launcher/Emoji.js" as Emoji

Item {
    // What each query should put first.
    readonly property var firsts: ({
        "fire": "fire",
        "thumbs": "thumbs up",
        "shrug": "person shrugging",
        "clap": "clapping hands",
        "red heart": "red heart",
        "canada": "flag: Canada",
        "tada": "party popper",
        "party": "party popper",
        "cat": "cat",
        "rocket": "rocket",
        "eyes": "eyes",
        "skull": "skull",
        "think": "thinking face",
        "grin": "grinning face",
        "joy": "face with tears of joy",
        "100": "hundred points",
        "warn": "warning",
        "check": "check mark"
    })
    // Letters scattered through a name find nothing, as everywhere else.
    readonly property var nothing: ["zzzz", "qxj", "kitty litter box", "thup"]

    function quoted(value) {
        return JSON.stringify(value);
    }

    Component.onCompleted: {
        const request = new XMLHttpRequest();
        request.open("GET", Qt.resolvedUrl("emoji.jsonl"), false);
        request.send();
        const all = request.responseText.trim().split("\n")
            .map((line, i) => Emoji.prepare(JSON.parse(line), i));
        const failures = [];
        if (all.length < 1800)
            failures.push(`only ${all.length} emoji were listed`);
        for (const query in firsts) {
            const found = Emoji.search(all, Fuzzy.words(query)).map(e => e.name);
            if (found[0] !== firsts[query])
                failures.push(`"${query}" put first ${quoted(found[0])}, not "${firsts[query]}"; `
                    + `then ${quoted(found.slice(1, 4))}`);
        }
        // Found by where words start, a match is as tidy as can be, the
        // only way one is found.
        for (const [query, grade] of [["tada", 0], ["party pop", 0]]) {
            const words = Fuzzy.words(query);
            const got = Emoji.grade(Emoji.search(all, words), words);
            if (got !== grade)
                failures.push(`"${query}" was graded ${got}, not ${grade}`);
        }
        for (const query of nothing) {
            const found = Emoji.search(all, Fuzzy.words(query)).map(e => e.name);
            if (found.length > 0)
                failures.push(`"${query}" found ${quoted(found.slice(0, 3))}, but nothing should`);
        }

        // How long a search of every emoji takes, against the least any
        // search could: one look for its first word in each emoji's
        // keywords. A busy machine slows both alike, so the ratio holds
        // where milliseconds wouldn't. Every query is typed a letter at a
        // time, each searching every emoji, though the launcher narrows the
        // last results as a query grows. On a laptop at rest a search takes
        // about a millisecond, three times the least, against the 8.3 ms of
        // a frame at 120 Hz; matching every keyword loosely as well, as the
        // main search matches names, takes it past the budget. The fastest
        // of five batches counts. How fast the engine runs the search
        // differs from one process to the next by as much as half, which no
        // batch within one can even out, so a search too slow exits 2 and
        // the check runs it again in a fresh process (checks.nix); a
        // search that finds the wrong emoji exits 1 and is never run again.
        const most = 5;
        const typed = Object.keys(firsts)
            .reduce((all, q) => all.concat(Array.from(q, (_, i) => q.slice(0, i + 1))), []);
        const timed = work => {
            const started = Date.now();
            for (const query of typed)
                work(Fuzzy.words(query));
            return (Date.now() - started) / typed.length;
        };
        let each = Infinity;
        let floor = Infinity;
        for (let batch = 0; batch < 5; batch++) {
            floor = Math.min(floor, timed(words => {
                let seen = 0;
                for (const e of all)
                    if (e.meant.indexOf(words[0]) >= 0)
                        seen++;
                return seen;
            }));
            each = Math.min(each, timed(words => Emoji.search(all, words)));
        }
        const times = each / Math.max(floor, 0.001);
        console.warn(`${typed.length} searches of ${all.length} emoji: ${each.toFixed(3)} ms each, `
            + `${times.toFixed(1)} times the least a search could take`);
        const slow = times > most;
        if (slow)
            console.warn(`a search of every emoji took ${times.toFixed(1)} times the least a `
                + `search could, more than ${most}`);

        // Said as warnings, since Qt keeps ordinary logging quiet.
        for (const failure of failures)
            console.warn(failure);
        const asked = Object.keys(firsts).length + nothing.length + 5;
        console.warn(failures.length > 0 ? `${failures.length} of ${asked} went wrong`
            : slow ? `1 of ${asked} too slow in this process`
            : `all ${asked} held`);
        Qt.exit(failures.length > 0 ? 1 : slow ? 2 : 0);
    }
}
