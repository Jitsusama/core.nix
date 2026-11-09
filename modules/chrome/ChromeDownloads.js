// Chrome's downloads, as chrome-recent-sites prints them, for its Recent
// and its Open the Latest Download (ChromeProgram.qml). Plain JavaScript,
// held to it by tests/quick-acts.qml in Qt's own engine.
.pragma library

const MILLISECONDS = 1000;
// A page's address, up to its host, and what follows it.
const SCHEME = /^[a-z][a-z0-9+.-]*:\/\//i;
const PAST_HOST = /[/?#].*$/;

// The downloads a line each says: its path, its size in bytes, when it
// finished in seconds since 1970 or nothing, and the page it came from,
// a tab between each, newest first as printed. Each is { path, size, at,
// from }, `at` in milliseconds as Recent dates things, or null where
// Chrome kept no time; a line without a path and a size is left out.
function listed(lines) {
    const found = [];
    for (const line of lines) {
        const [path, size, when, from] = line.split("\t");
        const bytes = Number(size);
        if (!path || size === undefined || size === "" || !Number.isFinite(bytes)) {
            continue;
        }
        const seconds = when === undefined || when === "" ? Number.NaN : Number(when);
        found.push({
            path,
            size: bytes,
            at: Number.isFinite(seconds) ? seconds * MILLISECONDS : null,
            from: from ?? ""
        });
    }
    return found;
}

// The newest of them, which Open the Latest Download opens, or null while
// Chrome has none still where it saved it.
function latest(downloads) {
    return downloads.length > 0 ? downloads[0] : null;
}

// The host of the page it came from, as its drawer says it, or "".
function cameFrom(address) {
    if (!SCHEME.test(address ?? "")) {
        return "";
    }
    return address.replace(SCHEME, "").replace(PAST_HOST, "");
}

// What opens it, as the desktop opens a file.
function opening(download) {
    return ["xdg-open", download.path];
}
