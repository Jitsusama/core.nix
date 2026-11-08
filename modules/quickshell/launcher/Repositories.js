// The git repositories under a folder, as the acts that list them read
// them (Repositories.qml): which there are and when each one's git last
// wrote, the one written last first, a repository's files the same way,
// each said from its repository's folder, and the page on GitHub of each
// that has a remote there. Kept apart from
// Quickshell's types, so the `the-launcher-acts-only-on-what-it-can`
// check runs it in Qt's own engine (tests/quick-acts.qml).
.pragma library

// A remote on GitHub, however git was told it: by SSH's short form or its
// URL, or by HTTPS, git's own protocol or a name and password, its owner
// and name after the host, .git or a slash after them or not.
const GITHUB = new RegExp("^(?:(?:ssh|git|https?)://(?:[^@/]+@)?github\\.com(?::\\d+)?/"
    + "|(?:[^@/]+@)?github\\.com:)([A-Za-z0-9_.-]+)/([A-Za-z0-9_.-]+?)(?:\\.git)?/?$");

// Where a line of grep's ends its config's path, then a section's heading,
// a remote's naming it, and a url as git writes them.
const CONFIG = "/.git/config:";
const HEADING = /^\[/;
const REMOTE = /^\[remote\s+"([^"]+)"\]$/;
const URL_LINE = /^url\s*=\s*(.+)$/;

// Milliseconds in a second, as stat's seconds are kept.
const SECOND = 1000;

// Two paths in order, A to Z by their characters, so two dated alike never
// swap between two reads.
function byPath(a, b) {
    if (a.path === b.path) {
        return 0;
    }
    return a.path < b.path ? -1 : 1;
}

// What stat says of each path, as the shell asks it ("%Y\t%n"): the
// seconds since 1970 it last changed, a tab, and its path. Each as
// { path, at }, at in milliseconds, the one changed last first and two
// alike by their paths, so the order never moves between two reads.
function dated(text) {
    return text.split("\n").map(line => {
        const tab = line.indexOf("\t");
        if (tab < 0) {
            return null;
        }
        return { path: line.slice(tab + 1), at: Number(line.slice(0, tab)) * SECOND };
    }).filter(one => one !== null && one.path !== "" && Number.isFinite(one.at))
        .sort((a, b) => b.at - a.at || byPath(a, b));
}

// The most of one repository's files its list holds, the ones changed
// last, since a row is built for each as it's stepped into and the
// largest repository Joel keeps holds over 23,000 files. The rest are
// left out of the list, as old as they are, for Neovim's own finder.
const MOST_FILES = 5000;

// A repository's files, dated, cut to the ones changed last.
function newest(files) {
    return files.slice(0, MOST_FILES);
}

// Each repository, by what stat says of its .git, a folder or, in a
// worktree, a file: { path, at }, the repository's own folder and when
// its git last wrote there, as git does whenever it changes its index,
// its branches or what it fetched, the one written last first.
function repositories(text) {
    return dated(text).filter(one => one.path.endsWith("/.git"))
        .map(one => ({ path: one.path.slice(0, -"/.git".length), at: one.at }));
}

// Each repository's remotes, by what grep says of its .git/config, each
// line its path, a colon, then a section's heading or a url: { [the
// repository's folder]: [{ name, url }] }, each in the file's order. A
// url is a remote's only under its heading, since a submodule's has one
// too.
function remotes(text) {
    const all = {};
    const named = {};
    for (const line of text.split("\n")) {
        const cut = line.indexOf(CONFIG);
        if (cut < 0) {
            continue;
        }
        const repository = line.slice(0, cut);
        const said = line.slice(cut + CONFIG.length).trim();
        if (HEADING.test(said)) {
            named[repository] = REMOTE.exec(said)?.[1];
            continue;
        }
        const url = URL_LINE.exec(said);
        if (url !== null && named[repository] !== undefined) {
            all[repository] = (all[repository] ?? [])
                .concat([{ name: named[repository], url: url[1].trim() }]);
        }
    }
    return all;
}

// A remote's page on GitHub, { address, name }, its HTTPS address and its
// owner and name ("Jitsusama/core.nix"), or null for a remote elsewhere.
function github(url) {
    const found = GITHUB.exec(url);
    if (found === null) {
        return null;
    }
    const name = `${found[1]}/${found[2]}`;
    return { address: `https://github.com/${name}`, name };
}

// A repository's page on GitHub, from its remotes: origin's, where it's
// there, since that's the one pushed to, then upstream's, as a fork
// names what it came from, then the first other remote there, or null
// for a repository with none on GitHub.
function onGitHub(listed) {
    const there = (listed ?? []).map(remote => ({ name: remote.name, page: github(remote.url) }))
        .filter(remote => remote.page !== null);
    const first = there.find(remote => remote.name === "origin")
        ?? there.find(remote => remote.name === "upstream") ?? there[0];
    return first === undefined ? null : first.page;
}

// The repositories listed that have a page on GitHub, in the order they're
// listed, each { path, at, page }, by each one's remotes as remotes gives
// them, by its folder.
function onGitHubAll(listed, byFolder) {
    return listed.map(one => ({ path: one.path, at: one.at, page: onGitHub(byFolder[one.path]) }))
        .filter(one => one.page !== null);
}

// The folder a file is in, said from its repository's own folder, as the
// path above it already names the repository: "docs" for one in its
// docs, "" for one at its top.
function within(path, repository) {
    const inside = path.startsWith(`${repository}/`) ? path.slice(repository.length + 1) : path;
    const cut = inside.lastIndexOf("/");
    return cut < 0 ? "" : inside.slice(0, cut);
}
