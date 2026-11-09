// What Programs › Neovim makes of what each Neovim holds and of the
// sessions kept on leaving (NeovimProgram.qml): `open-now neovim`'s lines
// read, every file open once, what each one's row says, the command going
// to it, what the program's drawer says, and the sessions read from what
// find lists of their folder. Kept apart from Quickshell's types, so the
// `the-launcher-acts-only-on-what-it-can` check runs it in Qt's own engine
// (tests/quick-acts.qml).
.pragma library
.import "Ago.js" as Ago

// Between the parts of a side, as every side joins them.
const BETWEEN = " \u00b7 ";

// A line of `open-now neovim`: `@` and niri's window, what Go To gives
// open-now to bring the Neovim forward inside kitty, where its server
// listens, and what it holds as JSON.
const FIELDS = 4;

// A session's file ends so, and starts with the root folder's / written
// as its name writes it (sessions.lua's name).
const SESSION = ".vim";
const ROOT = "=2F";

const MILLISECONDS = 1000;

// What `open-now neovim` printed, a Neovim a line, each { window, go,
// server, held }, `held` its process, folder, tabs and files as
// open-with's neovim-held.lua says them; a line not so is passed over, so
// the rest are still read.
function heard(text) {
    const neovims = [];
    for (const line of text.split("\n")) {
        const parts = line.split("\t");
        if (parts.length < FIELDS || !parts[0].startsWith("@")) {
            continue;
        }
        let holds = null;
        try {
            holds = JSON.parse(parts.slice(FIELDS - 1).join("\t"));
        } catch (_malformed) {
            continue;
        }
        if (holds === null || !Array.isArray(holds.files)) {
            continue;
        }
        neovims.push({
            window: Number(parts[0].slice(1)),
            go: parts[1],
            server: parts[2],
            held: holds
        });
    }
    return neovims;
}

// Every file the Neovims hold, each once, from the Neovim looked at last,
// by `focused(window)`, its niri window's focus time, and in it the file
// used last first, so Return lands where Joel was. Each { path, buffer,
// modified, tabs, tabCount, neovim, windows }: `tabs` the numbers of the
// tabs showing it, none while it's hidden, `tabCount` how many tabs its
// Neovim has, and `windows` how many Neovims hold it, modified where any
// of them has it modified, since that's what waits on Joel.
function open(neovims, focused) {
    const ordered = neovims.slice().sort((a, b) => focused(b.window) - focused(a.window));
    const byPath = new Map();
    const files = [];
    for (const neovim of ordered) {
        const own = neovim.held.files.slice().sort((a, b) => (b.at ?? 0) - (a.at ?? 0));
        for (const file of own) {
            const seen = byPath.get(file.path);
            if (seen) {
                seen.windows += 1;
                seen.modified = seen.modified || file.modified === true;
                continue;
            }
            const one = {
                path: file.path,
                buffer: file.buffer,
                modified: file.modified === true,
                tabs: file.tabs ?? [],
                tabCount: neovim.held.tabs ?? 1,
                neovim,
                windows: 1
            };
            byPath.set(file.path, one);
            files.push(one);
        }
    }
    return files;
}

// Where a file is in its Neovim: the tabs showing it, by their number,
// while it has more than one tab, shown while it has one, and hidden
// where no window shows it.
function where(one, said) {
    if (one.tabs.length === 0) {
        return said.hidden ?? "";
    }
    if (one.tabCount <= 1) {
        return said.shown ?? "";
    }
    if (one.tabs.length === 1) {
        return (said.tab ?? "").replace("{index}", String(one.tabs[0]));
    }
    return (said.tabs ?? "").replace("{indices}", one.tabs.join(", "));
}

// What a file's row says before its folder (Files.held): modified first,
// as what waits on Joel, then where it is, then how many windows hold it
// where more than one does, so a side cut short keeps its state.
function side(one, said) {
    return [
        one.modified ? said.modified ?? "" : "",
        where(one, said),
        one.windows > 1 ? (said.windows ?? "").replace("{count}", String(one.windows)) : ""
    ].filter(part => part !== "").join(BETWEEN);
}

// The command bringing a file forward: its Neovim's kitty window inside
// kitty, then the buffer inside Neovim, through open-now.
function going(openNow, one) {
    return [openNow, "neovim", "go", one.neovim.go, one.neovim.server, String(one.buffer)];
}

// What the program's drawer says Neovim holds (Applications.qml asks every
// provider's programFacts): how many files in how many windows, how many
// of them are modified, and how many sessions are kept, each only while
// there are any.
function held(neovims, files, kept, said) {
    const windows = neovims.length;
    const inWindows = (windows === 1 ? said.inWindow : said.inWindows) ?? "";
    const modified = files.filter(one => one.modified).length;
    return [
        ["nvim:files", files.length === 0 ? ""
            : inWindows.replace("{count}", String(files.length)).replace("{windows}",
                String(windows))],
        ["nvim:modified", modified > 0 ? String(modified) : ""],
        ["nvim:sessions", kept.length > 0 ? String(kept.length) : ""]
    ].filter(fact => fact[1] !== "").map(fact => ({ id: fact[0], value: fact[1] }));
}

// A session's folder, read back from its file's name (sessions.lua's
// name), where = and two hex digits are a byte, read as UTF-8 as a URL's
// %-escapes are; any % of its own is escaped first, so it stays itself.
// A name that doesn't read back is "".
function folderOf(name) {
    const bare = name.endsWith(SESSION) ? name.slice(0, name.length - SESSION.length) : name;
    try {
        return decodeURIComponent(bare.split("%").join("%25").split("=").join("%"));
    } catch (_malformed) {
        return "";
    }
}

// What find printed of the sessions' folder, a file a line: when it was
// written, in seconds since 1970, a tab, and its name. Each session
// { folder, path, at }, `at` in milliseconds, newest first; a file that
// isn't one, such as one Neovim left half written, is passed over.
function sessions(text, place) {
    return text.split("\n").map(line => line.split("\t"))
        .filter(parts => parts.length === 2 && parts[1].startsWith(ROOT)
            && parts[1].endsWith(SESSION))
        .map(([when, name]) => ({
            folder: folderOf(name),
            path: `${place}/${name}`,
            at: Number(when) * MILLISECONDS
        }))
        .filter(one => Number.isFinite(one.at) && one.folder !== "")
        .sort((a, b) => b.at - a.at);
}

// What a session's row says beside the folder's name: when it was kept,
// then the folder holding it, as `within` gives it.
function sessionSide(one, now, within) {
    return [Ago.ago(one.at, now), within].filter(part => part !== "").join(BETWEEN);
}
