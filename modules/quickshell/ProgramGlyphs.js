// Each program's own mark, said once for everywhere a program is shown: the
// launcher's rows (launcher/Provider.qml) and the notices it sends
// (Notices.qml), so a notice wears the mark its program's row does and never
// a bell where the program has one. By its desktop entry's id or a window's
// app id: its logo where the icon font has one, as a hex code point, and
// otherwise the theme's mark for what it is, as `mark:` and the mark's name,
// so no two programs read alike. Logos are named by the programs rather than
// by a meaning, so they're kept here and not among the theme's marks. A
// window Chrome opens for a site wears Chrome's. Every program an account
// lists has its own (`every-program-wears-its-own-mark`).
.pragma library

const table = {
    kitty: "f011b",
    "google-chrome": "f268",
    chromium: "f268",
    slack: "f198",
    nvim: "f36f",
    "com.onepassword.onepassword": "f0881",
    gimp: "f338",
    "org.inkscape.inkscape": "f33b",
    spotify: "f1bc",
    swayimg: "f02e9",
    "org.pwmt.zathura": "f0219",
    mpv: "f36e",
    reader: "f0354",
    btop: "mark:processor",
    pi: "mark:agent",
    tuple: "f1218",
    "google-mail": "f02ab",
    "google-calendar": "mark:calendar",
    "google-drive": "f02b6",
    "google-docs": "f0dc8",
    "google-sheets": "f09f7",
    "google-slides": "f0428",
    "google-meet": "f0567",
};

const CHROME_SITE = /^chrome-.+-[^-]+$/u;
const HEX = 16;
const MARK = "mark:";

// A program's code in the table, with a machine's own over it (`own`, the
// launcher's `settings.glyphs`), or "" for one it doesn't know.
function code(id, own) {
    const key = (id ?? "").toLowerCase();
    const all = Object.assign({}, table, own ?? {});
    return all[key] ?? (CHROME_SITE.test(key) ? all["google-chrome"] : "");
}

// A program's mark as a glyph, the theme's marks given for one named by its
// meaning, or "" for a program the table doesn't know.
function glyph(id, own, marks) {
    const found = code(id, own);
    if (found.startsWith(MARK)) {
        return marks?.[found.slice(MARK.length)] ?? "";
    }
    return found === "" ? "" : String.fromCodePoint(Number.parseInt(found, HEX));
}
