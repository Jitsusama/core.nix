// What the launcher's matcher must find, and how fast, run in Qt's own
// JavaScript engine, the one the shell runs it in. The
// `the-launcher-finds-what-is-meant` check copies Fuzzy.js beside this file
// and runs `qml -platform offscreen fuzzy.qml`, over a fixed list shaped like
// a real desktop's: its actions, programs and session commands. Each query
// names the row that has to come first, leaves few rows, or finds nothing.
import QtQuick
import "Fuzzy.js" as Fuzzy

QtObject {
    readonly property var names: [
        "Open a terminal",
        "Open a terminal below this window",
        "Open the next window below this one",
        "Show these shortcuts",
        "Close the window",
        "Focus the column to the left, or the screen past it",
        "Focus the column to the right, or the screen past it",
        "Focus the window above, or the workspace past it",
        "Focus the window below, or the workspace past it",
        "Focus the first column",
        "Focus the last column",
        "Find a window",
        "Focus the floating windows, or the tiled ones",
        "Move the column left, or onto the screen past it",
        "Move the column right, or onto the screen past it",
        "Move the column to the workspace above",
        "Move the column to the workspace below",
        "Move the column to the start",
        "Move the column to the end",
        "Move the window into the column on the left, or out of its own",
        "Move the window into the column on the right, or out of its own",
        "Move the window up, or onto the workspace past it",
        "Move the window down, or onto the workspace past it",
        "Focus the screen to the left",
        "Focus the screen to the right",
        "Focus the screen above",
        "Focus the screen below",
        "Move the column to the screen to the left",
        "Move the column to the screen to the right",
        "Move the column to the screen above",
        "Move the column to the screen below",
        "Move the window to the screen to the left",
        "Move the window to the screen to the right",
        "Move the window to the screen above",
        "Move the window to the screen below",
        "Go to the workspace above",
        "Go to the workspace below",
        "Move the workspace up",
        "Move the workspace down",
        "Move the window to the workspace above",
        "Move the window to the workspace below",
        "Move the workspace to the screen before",
        "Move the workspace to the screen after",
        "Show every workspace",
        "Cycle the column's width",
        "Cycle the window's height",
        "Widen the column",
        "Narrow the column",
        "Make the window taller",
        "Make the window shorter",
        "Give the window back its share of the column's height",
        "Make the column as wide as the screen, or put it back",
        "Widen the column into the space beside it",
        "Stretch the window to the screen's edges, or put it back",
        "Fill the screen with the window, or put it back",
        "Float the window, or tile it again",
        "Tell the window it fills the screen, but keep it in its tile",
        "Show the column's windows as tabs, or stacked",
        "Centre the window",
        "Centre every column in view",
        "Give every key to the window, or take them back",
        "Turn the volume up",
        "Turn the volume down",
        "Mute the speakers, or unmute them",
        "Mute the microphone, or unmute it",
        "Play or pause the music",
        "Play the next track",
        "Play the previous track",
        "Brighten the screen",
        "Dim the screen",
        "Take a screenshot of a region",
        "Take a screenshot of the screen",
        "Take a screenshot of the window",
        "kitty",
        "Google Chrome",
        "Slack",
        "Neovim",
        "1Password",
        "btop",
        "Files",
        "Settings",
        "Lock the screen",
        "Suspend the machine",
        "Log out",
        "Reboot the machine",
        "Power off the machine",
        "Join a Wi-Fi network"
    ].concat([1, 2, 3, 4, 5, 6, 7, 8, 9].reduce((all, n) => all.concat([
        `Go to workspace ${n}`,
        `Move the column to workspace ${n}`,
        `Move the window to workspace ${n}`
    ]), []))

    readonly property var firsts: ({
        "vol up": "Turn the volume up",
        "vol": "Turn the volume up",
        "vol down": "Turn the volume down",
        "loc": "Lock the screen",
        "lock": "Lock the screen",
        "workspace up": "Move the workspace up",
        "below": "Focus the screen below",
        "tab": "Show the column's windows as tabs, or stacked",
        "float": "Float the window, or tile it again",
        "close": "Close the window",
        "term": "Open a terminal",
        "open term": "Open a terminal",
        "term below": "Open a terminal below this window",
        "kit": "kitty",
        // A word written with a hyphen, typed without it, is the same word.
        "wifi": "Join a Wi-Fi network",
        "join wifi": "Join a Wi-Fi network",
        "wi-fi": "Join a Wi-Fi network",
        "chr": "Google Chrome",
        "chrome": "Google Chrome",
        "sl": "Slack",
        "neo": "Neovim",
        "1p": "1Password",
        "pass": "1Password",
        "workspace 3": "Go to workspace 3",
        "screenshot": "Take a screenshot of a region",
        "screenshot win": "Take a screenshot of the window",
        "dim": "Dim the screen",
        "bright": "Brighten the screen",
        "mute mic": "Mute the microphone, or unmute it",
        "next": "Play the next track",
        "prev": "Play the previous track",
        "pause": "Play or pause the music",
        "music": "Play or pause the music",
        "sus": "Suspend the machine",
        "reb": "Reboot the machine",
        "off": "Power off the machine",
        "log": "Log out",
        "cent": "Centre the window",
        // The word typed whole beats the word it begins.
        "wide": "Make the column as wide as the screen, or put it back",
        "widen": "Widen the column",
        "every key": "Give every key to the window, or take them back"
    })
    // Letters that start no word find nothing, initials and letters
    // inside a word among them.
    readonly property var nothing: ["zz", "qx", "xylophone", "kc", "zq", "gc", "vu", "lck", "shot", "rome", "nv"]
    readonly property var atMost: ({
        "close": 1,
        "tab": 1,
        "wide": 3,
        "float": 2,
        "lock": 1,
        "kit": 1,
        "sl": 1
    })

    function rank(things, query) {
        const words = Fuzzy.words(query);
        const need = Fuzzy.letters(words);
        const found = [];
        for (const item of things) {
            if ((item.mask & need) !== need)
                continue;
            const m = Fuzzy.match(item, words, false);
            if (m.score >= 0)
                found.push({ item: item, score: m.score, grade: m.grade });
        }
        return Fuzzy.keep(found).map(f => f.item.text);
    }

    function quoted(value) {
        return JSON.stringify(value);
    }

    Component.onCompleted: {
        const things = names.map(Fuzzy.prepare);
        const failures = [];
        for (const query in firsts) {
            const found = rank(things, query);
            if (found[0] !== firsts[query])
                failures.push(`"${query}" put first ${quoted(found[0])}, not "${firsts[query]}"; `
                    + `then ${quoted(found.slice(1, 4))}`);
        }
        for (const query of nothing) {
            const found = rank(things, query);
            if (found.length > 0)
                failures.push(`"${query}" found ${quoted(found.slice(0, 3))}, but nothing should`);
        }
        for (const query in atMost) {
            const found = rank(things, query);
            if (found.length > atMost[query])
                failures.push(`"${query}" left ${found.length} rows, more than ${atMost[query]}: `
                    + quoted(found.slice(0, 5)));
        }

        // Passing over a thing whose words don't start as a typed word
        // does loses nothing: every query and every prefix of one, each a
        // letter of the alphabet alone too, finds the same things with the
        // word starts tested first as it does with them not (a match that
        // wants its positions tests neither mask), and a text's starts say
        // the same as its prepared self's.
        const alone = Array.from("abcdefghijklmnopqrstuvwxyz0123456789");
        const prefixes = Object.keys(firsts).concat(nothing, Object.keys(atMost))
            .reduce((all, q) => all.concat(Array.from(q, (_, i) => q.slice(0, i + 1))), alone);
        let unstarted = 0;
        for (const query of prefixes) {
            const words = Fuzzy.words(query);
            for (const item of things) {
                const quick = Fuzzy.match(item, words, false).score >= 0;
                if (quick !== Fuzzy.match(item, words, true).score >= 0)
                    failures.push(`"${query}" ${quick ? "found" : "missed"} "${item.text}"`
                        + " only when tested by its word starts");
                if (!Fuzzy.begins(item.starts, words))
                    unstarted++;
            }
        }
        for (const item of things)
            if (Fuzzy.starts(item.text) !== item.starts)
                failures.push(`"${item.text}" starts its words differently said alone`);
        if (unstarted === 0)
            failures.push("no thing was passed over by its word starts");
        // A text kept ready is made once while it reads the same, and
        // matches as one made afresh does.
        const once = Fuzzy.kept("Open a terminal");
        if (Fuzzy.kept("Open a terminal") !== once || Fuzzy.kept("Open a window") === once)
            failures.push("a text kept ready was made again, or kept for another");
        if (Fuzzy.match(once, Fuzzy.words("open term"), false).score
            !== Fuzzy.match(Fuzzy.prepare("Open a terminal"), Fuzzy.words("open term"), false).score)
            failures.push("a text kept ready matched otherwise than one made afresh");

        // How long a query takes, against the least any search could: one
        // look for its first word in each thing. A machine busy with other
        // work slows both alike, so the ratio holds where a time in
        // milliseconds wouldn't, and it still catches the matcher growing
        // slower in kind. On a laptop at rest a query over this list, the
        // size of a desktop's main search, takes about 0.1 ms, three looks'
        // worth, against the 8.3 ms of a frame at 120 Hz, since matching
        // by words is a look for each word and a sum. Every query is
        // typed a letter at a time, as a person types it, each searching
        // the whole list, though the launcher narrows the last results as a
        // query grows; over a list three times the size too. The fastest of
        // five batches counts. How fast the engine runs the matcher differs
        // from one process to the next by as much as half, which no batch
        // within one can even out, so a matcher too slow exits 2 and the
        // check runs it again in a fresh process (checks.nix); a ranking
        // that's wrong exits 1 and is never run again. At rest it's 3.6 to
        // 4.3 times (2026-10-29, one core alone or shared with a busy loop),
        // and a machine loaded past its cores raises it to 5, since the
        // matcher's allocations share caches and memory the floor's plain
        // look hardly uses; a matcher grown slower in kind costs a multiple,
        // which 6 still catches.
        const most = 6;
        const many = things.concat(
            names.map(n => Fuzzy.prepare(n + " again")),
            names.map(n => Fuzzy.prepare("another " + n)));
        const typed = Object.keys(firsts)
            .reduce((all, q) => all.concat(Array.from(q, (_, i) => q.slice(0, i + 1))), []);
        const rounds = 4;
        const timed = work => {
            const started = Date.now();
            for (let r = 0; r < rounds; r++)
                for (const query of typed)
                    work(query);
            return (Date.now() - started) / (rounds * typed.length);
        };
        const slow = [];
        for (const list of [things, many]) {
            let each = Infinity;
            let floor = Infinity;
            for (let batch = 0; batch < 5; batch++) {
                floor = Math.min(floor, timed(query => {
                    const first = Fuzzy.words(query)[0];
                    let seen = 0;
                    for (const item of list)
                        if (item.folded.indexOf(first) >= 0)
                            seen++;
                    return seen;
                }));
                each = Math.min(each, timed(query => rank(list, query)));
            }
            const times = each / Math.max(floor, 0.001);
            console.warn(`${typed.length} queries over ${list.length} things: `
                + `${each.toFixed(3)} ms each, ${times.toFixed(1)} times the least a search could take`);
            if (times > most)
                slow.push(`a query over ${list.length} things took ${times.toFixed(1)} times `
                    + `the least a search could, more than ${most}`);
        }

        // Said as warnings, since Qt keeps ordinary logging quiet.
        for (const failure of failures.concat(slow))
            console.warn(failure);
        const asked = Object.keys(firsts).length + nothing.length + Object.keys(atMost).length + 5;
        console.warn(failures.length > 0 ? `${failures.length} of ${asked} went wrong`
            : slow.length > 0 ? `${slow.length} of ${asked} too slow in this process`
            : `all ${asked} held`);
        Qt.exit(failures.length > 0 ? 1 : slow.length > 0 ? 2 : 0);
    }
}
