// What Programs › Neovim makes of what each Neovim holds
// (NeovimProgram.qml): `open-now neovim`'s lines read, every file open
// once, what each one's row says, what only Neovim says in its drawer, the
// command going to it, and what the program's drawer says. Kept apart
// from Quickshell's types, so the `the-launcher-acts-only-on-what-it-can`
// check runs it in Qt's own engine (tests/quick-acts.qml).
.pragma library

// Between the parts of a side, as every side joins them.
const BETWEEN = " \u00b7 ";

// A line of `open-now neovim`: `@` and niri's window, what Go To gives
// open-now to bring the Neovim forward inside kitty, where its server
// listens, and what it holds as JSON.
const FIELDS = 4;

// Neovim says when in seconds, and the launcher in milliseconds.
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
// modified, tabs, tabCount, at, neovim, windows }: `tabs` the numbers of
// the tabs showing it, none while it's hidden, `tabCount` how many tabs its
// Neovim has, `at` when that Neovim last had it in a window, in seconds, 0
// for never, and `windows` how many Neovims hold it, modified where any of
// them has it modified, since that's what waits on Joel.
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
                at: file.at ?? 0,
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

// What a file's drawer says that only its Neovim knows, after what's said
// of the file itself (NeovimProgram.qml's plate): when it was last in a
// window, said by `when(milliseconds)`, only where Neovim has said, and
// then its buffer's number, as :ls lists it and :b takes it, last since
// it's an identifier (VKCF3Z, b9a2e7ae). Each { id, value }, by the
// launcher's words. Whether it's modified and where it shows are its
// row's side, so neither is said again.
function plateFacts(one, when) {
    return [
        ["nvim:used", one.at > 0 ? when(one.at * MILLISECONDS) : ""],
        ["nvim:buffer", Number.isInteger(one.buffer) ? String(one.buffer) : ""]
    ].filter(fact => fact[1] !== "").map(fact => ({ id: fact[0], value: fact[1] }));
}

// The command bringing a file forward: its Neovim's kitty window inside
// kitty, then the buffer inside Neovim, through open-now.
function going(openNow, one) {
    return [openNow, "neovim", "go", one.neovim.go, one.neovim.server, String(one.buffer)];
}

// What the program's drawer says Neovim holds (Applications.qml asks every
// provider's programFacts): how many files in how many windows, how many
// of them are modified, each only while there are any.
function held(neovims, files, said) {
    const windows = neovims.length;
    const inWindows = (windows === 1 ? said.inWindow : said.inWindows) ?? "";
    const modified = files.filter(one => one.modified).length;
    return [
        ["nvim:files", files.length === 0 ? ""
            : inWindows.replace("{count}", String(files.length)).replace("{windows}",
                String(windows))],
        ["nvim:modified", modified > 0 ? String(modified) : ""]
    ].filter(fact => fact[1] !== "").map(fact => ({ id: fact[0], value: fact[1] }));
}
