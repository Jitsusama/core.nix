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

// The path a line recent-files printed names, a program, a path and when
// with a tab between each: what lies between the first tab and the last,
// whole though it holds a tab, or the rest of a line with one tab, or ""
// from a line with none.
function openedPath(line) {
    const first = line.indexOf("\t");
    const last = line.lastIndexOf("\t");
    if (first < 0) {
        return "";
    }
    return first === last ? line.slice(first + 1) : line.slice(first + 1, last);
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

// A `recent` record of the programs daemon as the lines its source's
// command printed, each { line, at }, `at` in milliseconds or undefined
// where the source didn't say: a file's or a folder's path, a site's
// address and title with a tab between, or, from the list programs
// share, the program and the path.
function recentLines(record) {
    const at = used => (used === null || used === undefined ? undefined : used * SECOND);
    if (record.sites) {
        return record.sites.map(site => ({
            line: `${site.address}\t${site.title}`,
            at: at(site.used_at_s)
        }));
    }
    const shared = record.app === null || record.app === undefined;
    return (record.paths ?? []).map(one => ({
        line: shared ? `${one.program}\t${one.path}` : one.path,
        at: at(one.used_at_s)
    }));
}

// An `opened` record of the programs daemon as the lines its source's
// command printed (recents.nix's `opened`): a site's tab as its id, its
// window, whether it's in front, when it was last looked at, its address
// and its title; a file as its process and its path, or niri's window,
// the path and what Go To is given.
function openedLines(record) {
    const said = value => (value === null || value === undefined ? "" : String(value));
    if (record.sites) {
        return record.sites.map(tab => [tab.id, tab.window, tab.front ? "true" : "false",
            said(tab.looked_at_ms), tab.address, tab.title].map(said).join("\t"));
    }
    return (record.files ?? []).map(file => (file.window === null || file.window === undefined
        ? `${said(file.pid)}\t${file.path}`
        : `@${file.window}\t${file.path}\t${said(file.go)}`));
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
