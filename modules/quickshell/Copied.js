// What the clipboard holds, as the acts that take a copy read it
// (Copied.qml): the line the clipboard's history says of it, whether it's
// a picture and of what type, one web address alone, or a file's path
// with the line and column to open it at, and whether that file is
// markdown. Kept apart from Quickshell's types, so the
// `the-launcher-acts-only-on-what-it-can` check runs it in Qt's own
// engine (tests/quick-acts.qml).
.pragma library

// How the clipboard's history opens the line saying what the clipboard
// holds now, told apart from its copies' lines by that alone, so the
// copies it sends as the shell connects aren't parsed here.
const NOW = "{\"now\":";

// A picture's type, as the clipboard's history says it too
// (launcher/Copies.qml), in the fewest words, and the extension a file of
// it is saved with.
const PICTURES = {
    "image/png": { said: "PNG", extension: "png" },
    "image/jpeg": { said: "JPEG", extension: "jpg" },
    "image/webp": { said: "WebP", extension: "webp" },
    "image/gif": { said: "GIF", extension: "gif" }
};

// One web address and nothing else: http or https, a host by its name,
// an IPv4 address or a bracketed IPv6 one, a port, then anything without
// a space.
const LINK = new RegExp("^https?://(\\[[0-9a-f:.]+\\]|[a-z0-9]([a-z0-9-]*[a-z0-9])?"
    + "(\\.[a-z0-9]([a-z0-9-]*[a-z0-9])?)*)(:\\d{1,5})?([/?#]\\S*)?$", "i");

// A line and a column after a path, as compilers, grep and ripgrep say
// them ("main.rs:12", "main.rs:12:5", "main.rs:12:"), the path lazily so
// the last ones are taken.
const AT = /^(.+?):(\d+)(?::(\d+))?:?$/;

// The characters a folder's name filter reads as wildcards.
const WILDCARDS = /[*?[\]]/g;

// What a picture's type starts with, and what's left of an unknown one
// to be said or saved by.
const IMAGE = /^image\//;
const NOT_A_WORD = /[^a-z0-9]/gi;

// A web address's scheme, and a slash it ends on.
const SCHEME = /^https?:\/\//i;
const LAST_SLASH = /\/$/;

// What a path may not hold: a second line or a tab.
const BREAKS = /[\n\r\t]/;

// A file URL's start, as a file manager copies one, its host empty or
// this machine's.
const FILE_URL = /^file:\/\//i;
const FILE_HOST = /^file:\/\/(localhost)?/i;

// A markdown file's extensions.
const MARKDOWN = /\.(md|markdown)$/i;

// The extensions of the documents zathura's plugins read: MuPDF's PDF,
// XPS, EPUB, FictionBook and Mobipocket, DjVu, and PostScript. The pictures
// MuPDF reads too are the picture viewer's.
const DOCUMENT = /\.(pdf|xps|oxps|epub|fb2|mobi|djvu|djv|ps|eps)$/i;

// What the clipboard's history says the clipboard holds now,
// {"now":{"types":[...],"text":"..."}}, as { types, text }: the types it's
// offered as, and its text, "" for one with none to read, as one cleared
// or a password manager's. Null for any other line it sends, a copy kept
// or let go, which the launcher's Copies reads (clipboard/src/main.rs).
function heard(line) {
    if (!line.startsWith(NOW)) {
        return null;
    }
    let said;
    try {
        said = JSON.parse(line).now;
    } catch (_error) {
        return null;
    }
    if (said === null || typeof said !== "object") {
        return null;
    }
    return {
        types: Array.isArray(said.types)
            ? said.types.filter(type => typeof type === "string") : [],
        text: typeof said.text === "string" ? said.text : ""
    };
}

// The type to ask for the picture the clipboard holds, PNG wherever it's
// offered as PNG, since every picture program reads it, else the first
// picture type offered, or "" while it holds no picture.
function picture(types) {
    if (types.includes("image/png")) {
        return "image/png";
    }
    return types.find(type => type.startsWith("image/")) ?? "";
}

// A picture's type in the fewest words: its name where it's known, else
// what follows image/ in capitals, as a file's kind is said.
function pictureSaid(type) {
    return PICTURES[type]?.said ?? type.replace(IMAGE, "").toUpperCase();
}

// The extension a picture of that type is saved with.
function extension(type) {
    return PICTURES[type]?.extension ?? type.replace(IMAGE, "").replace(NOT_A_WORD, "");
}

// The one web address the copy is, spaces around it aside, or "" for
// anything else: a word, an address among other words, two addresses, or
// a link of another kind, as a mail's.
function link(text) {
    const one = text.trim();
    return LINK.test(one) ? one : "";
}

// An address as the launcher says it beside an act: without its scheme
// or a slash it ends on, since the act says it's a web address.
function linkSaid(address) {
    return address.replace(SCHEME, "").replace(LAST_SLASH, "");
}

// The file's path the copy names, from the root or from home as ~, or as
// a file URL as a file manager copies one, and the line and column after
// it where it gives them: { path, folder, name, line, column }, line and
// column 0 where it gives none, or null for anything else, as a
// relative path, which names no folder the shell is in, a folder's path
// ending in a slash, or more than one line. Whether the file is there is
// asked of the folder (Copied.qml).
function place(text, home) {
    const one = text.trim();
    if (one === "" || BREAKS.test(one)) {
        return null;
    }
    let path = one;
    if (FILE_URL.test(path)) {
        try {
            path = decodeURIComponent(path.replace(FILE_HOST, ""));
        } catch (_error) {
            return null;
        }
    }
    let line = 0;
    let column = 0;
    const at = AT.exec(path);
    if (at !== null) {
        path = at[1];
        line = Number(at[2]);
        column = at[3] === undefined ? 0 : Number(at[3]);
    }
    if (path === "~" || path.startsWith("~/")) {
        path = `${home}${path.slice(1)}`;
    }
    if (!path.startsWith("/") || path.endsWith("/")) {
        return null;
    }
    const cut = path.lastIndexOf("/");
    const name = path.slice(cut + 1);
    if (name === "." || name === "..") {
        return null;
    }
    return {
        path,
        folder: cut === 0 ? "/" : path.slice(0, cut),
        name,
        line,
        column: line === 0 ? 0 : column
    };
}

// A place as the launcher says it beside an act: from home as ~, then its
// line and column as they were copied.
function placeSaid(where, home) {
    const shown = where.path === home ? "~"
        : where.path.startsWith(`${home}/`) ? `~${where.path.slice(home.length)}` : where.path;
    const line = where.line > 0 ? `:${where.line}` : "";
    const column = where.column > 0 ? `:${where.column}` : "";
    return `${shown}${line}${column}`;
}

// The folder a place is in as a URL a folder model reads, each part of
// its path escaped, so a # or a space in a name stays part of it.
function folderUrl(folder) {
    return `file://${folder.split("/").map(part => encodeURIComponent(part)).join("/")}`;
}

// A file's name as a folder's name filter that finds only it, each
// wildcard in it made a class of itself.
function nameFilter(name) {
    return name.replace(WILDCARDS, wildcard => `[${wildcard}]`);
}

// Whether a file is markdown, by the extensions the reader's own files
// take: .md, and .markdown, which a few projects still use.
function markdown(path) {
    return MARKDOWN.test(path);
}

// Whether a file is a document zathura reads, by its extension.
function document(path) {
    return DOCUMENT.test(path);
}
