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
        "Go back to the window before, or hold Super to choose another",
        "Focus the floating windows, or the tiled ones",
        "Move the column left, or onto the screen past it",
        "Move the column right, or onto the screen past it",
        "Move the window up, or onto the workspace past it",
        "Move the window down, or onto the workspace past it",
        "Move the column to the start",
        "Move the column to the end",
        "Move the window into the column on the left, or out of its own",
        "Move the window into the column on the right, or out of its own",
        "Move the window up its column",
        "Move the window down its column",
        "Swap the window with the one to the left",
        "Swap the window with the one to the right",
        "Take the window to the right into this column",
        "Put the column's bottom window into a column of its own",
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
        "Move the column to the workspace above",
        "Move the column to the workspace below",
        "Move the window to the workspace above",
        "Move the window to the workspace below",
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
        "Fill the screen with the window, or put it back",
        "Float the window, or tile it again",
        "Tell the window it fills the screen, but keep it in its tile",
        "Stretch the window to the screen's edges, or put it back",
        "Show the column's windows as tabs, or stacked",
        "Centre the column",
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
        "Lock",
        "Suspend",
        "Log Out",
        "Reboot",
        "Power Off"
    ].concat([1, 2, 3, 4, 5, 6, 7, 8, 9].reduce((all, n) => all.concat([
        `Go to workspace ${n}`,
        `Move the column to workspace ${n}`,
        `Move the window to workspace ${n}`
    ]), []))

    readonly property var firsts: ({
        "vu": "Turn the volume up",
        "vol": "Turn the volume up",
        "vold": "Turn the volume down",
        "lo": "Lock",
        "lock": "Lock",
        "lck": "Lock",
        "sw": "Swap the window with the one to the left",
        "below": "Focus the screen below",
        "tab": "Show the column's windows as tabs, or stacked",
        "float": "Float the window, or tile it again",
        "close": "Close the window",
        "term": "Open a terminal",
        "open term": "Open a terminal",
        "term below": "Open a terminal below this window",
        "kit": "kitty",
        "chr": "Google Chrome",
        "gc": "Google Chrome",
        "chrome": "Google Chrome",
        "sl": "Slack",
        "nv": "Neovim",
        "1p": "1Password",
        "pass": "1Password",
        "ws 3": "Go to workspace 3",
        "screenshot": "Take a screenshot of a region",
        "shot win": "Take a screenshot of the window",
        "dim": "Dim the screen",
        "bright": "Brighten the screen",
        "mute mic": "Mute the microphone, or unmute it",
        "next": "Play the next track",
        "prev": "Play the previous track",
        "pause": "Play or pause the music",
        "music": "Play or pause the music",
        "sus": "Suspend",
        "reb": "Reboot",
        "off": "Power Off",
        "log": "Log Out",
        "cent": "Centre the column",
        "wide": "Widen the column",
        "every key": "Give every key to the window, or take them back"
    })
    readonly property var nothing: ["zz", "qx", "xylophone", "kc", "zq"]
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

        // How long a query takes, against the least any search could: one
        // look for its first word in each thing. A machine busy with other
        // work slows both alike, so the ratio holds where a time in
        // milliseconds wouldn't, and it still catches the matcher growing
        // slower in kind. On a laptop at rest a query over this list, the
        // size of a desktop's main search, takes 0.12 ms, four and a half
        // looks' worth, against the 8.3 ms of a frame at 120 Hz; doing the
        // alignment twice over takes it to almost seven. Every query is
        // typed a letter at a time, as a person types it, each searching
        // the whole list, though the launcher narrows the last results as a
        // query grows; over a list three times the size too. The fastest of
        // five batches counts. How fast the engine runs the matcher differs
        // from one process to the next by as much as half, which no batch
        // within one can even out, so a matcher too slow exits 2 and the
        // check runs it again in a fresh process (checks.nix); a ranking
        // that's wrong exits 1 and is never run again.
        const most = 6.5;
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
        const asked = Object.keys(firsts).length + nothing.length + Object.keys(atMost).length + 2;
        console.warn(failures.length > 0 ? `${failures.length} of ${asked} went wrong`
            : slow.length > 0 ? `${slow.length} of ${asked} too slow in this process`
            : `all ${asked} held`);
        Qt.exit(failures.length > 0 ? 1 : slow.length > 0 ? 2 : 0);
    }
}
