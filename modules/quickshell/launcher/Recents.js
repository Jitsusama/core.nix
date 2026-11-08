// How Recents.qml reads the lines its sources print, apart from the
// processes that print them, so tests/recent.qml holds each in Qt's own
// engine: a dated line's time, and the list many programs share, each
// program's paths and when it last used them.
.pragma library

const SECOND = 1000;
const DIGITS = /^\d+$/;

// A line a source printed, apart from when it says it was last used:
// { line, at }, `at` in milliseconds since 1970, or undefined where the
// source isn't dated, or the line left its time empty or not in digits.
function dated(line, isDated) {
    const tab = isDated ? line.lastIndexOf("\t") : -1;
    if (tab < 0) {
        return { line, at: undefined };
    }
    const seconds = line.slice(tab + 1);
    return {
        line: line.slice(0, tab),
        at: DIGITS.test(seconds) ? Number(seconds) * SECOND : undefined
    };
}

// A shared list's line as { app, path }: the program before the first tab,
// as `appOf` names its desktop entry ("" for none), and the path after it,
// whole though it holds a tab.
function sharedLine(line, appOf) {
    const tab = line.indexOf("\t");
    return { app: tab > 0 ? appOf(line.slice(0, tab)) : "", path: line.slice(tab + 1) };
}

// The shared list's lines, each program's paths newest first, each once,
// at most `most` a program.
function sharedFrom(lines, appOf, most) {
    const each = {};
    for (const { app, path } of lines.map(line => sharedLine(line, appOf))) {
        const paths = each[app] ?? [];
        if (app !== "" && paths.length < most && !paths.includes(path)) {
            each[app] = paths.concat([path]);
        }
    }
    return each;
}

// When each program on the shared list last used each of its paths, in
// milliseconds, by the line's time in `times`, the newest where it's
// listed twice, as `sharedFrom` keeps the newest.
function sharedAtFrom(lines, appOf, times) {
    const each = {};
    for (const line of lines) {
        const { app, path } = sharedLine(line, appOf);
        const at = times[line];
        const kept = each[app] ?? {};
        if (app !== "" && at !== undefined && !(path in kept)) {
            each[app] = Object.assign(kept, { [path]: at });
        }
    }
    return each;
}
