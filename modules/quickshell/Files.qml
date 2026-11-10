pragma Singleton
// A file and a folder, each one thing wherever the launcher shows it, as a
// piece of mail is (Mail.qml): `file` and `folder` give the whole row, which
// a program's Recent (Applications.qml) and the file chooser (open-with's
// FileChooser.qml) take as it is, made with the launcher's `thing` of the
// surface showing it. Its name is the last part of its path, its folder
// beside it, said from home as ~; it's keyed by its kind and its whole
// path, so one file is one thing wherever it shows. One kept elsewhere, as
// in Google's Drive, has no path here, so `elsewhere` keys and names it as
// its place says, drawn the same.
//
// The surface says what's done to it: its own acts come first, the first
// being what Return does, and Copy the Path always last. One given what it
// holds steps in on Return instead, its acts first inside, then what it
// holds, then Copy the Path: a folder is gone into rather than opened, and
// a file saved over asks first.
import qs
import Quickshell
import QtQuick
import "FileFacts.js" as FileFacts

Singleton {
    id: root

    readonly property string home: Quickshell.env("HOME")

    // A path as Joel reads it, from home as ~.
    function shown(path) {
        return path === home ? "~" : path.startsWith(`${home}/`) ? `~${path.slice(home.length)}` : path;
    }

    // Its name, the last part of its path: "~/src/core.nix" is core.nix,
    // and / and ~ are themselves; one in the Nix store goes by its own
    // name past its hash, but where it's `listed` in its folder, as the
    // file chooser lists it, whose rows keep the hash (FileFacts.named).
    function named(path, listed) {
        return FileFacts.named(shown(path), listed === true);
    }

    // The folder it's in, as said beside it: "~/src/core.nix" is in ~/src,
    // "/etc" in /.
    function within(path) {
        const said = shown(path);
        const cut = said.lastIndexOf("/");
        return cut < 0 || said === "/" ? "" : cut === 0 ? "/" : said.slice(0, cut);
    }

    // The folder said beside a file's name, but nothing for the home: a
    // lone "~" says nothing, and the drawer carries the path (VKCF3Z,
    // 2026-11-09).
    function beside(path) {
        const folder = within(path);
        return folder === "~" ? "" : folder;
    }

    // A file's row: Return does the first of `acts`, and Ctrl+] goes into
    // them all, then Copy the Path; given `inside`, Return steps in.
    function file(path, thing, acts, inside) {
        return made("file", path, thing, acts, inside ?? null, {});
    }

    // A file a program has open now, as a program's Recent leads with it:
    // the same row, with `now` (open, or closed) said before its folder, so
    // a side cut short loses the folder and never the state, which sits in
    // the same spot on every row that has one (VKCF3Z, 2026-10-30), and one
    // `gone` dimmed where it was, as the launcher keeps a row whose thing
    // has gone. Where the program holding it says how far through it is
    // (`through`), that's said in open's place, since it says open too.
    function held(path, thing, acts, now, gone) {
        const far = gone === true ? "" : throughOf(path);
        return made("file", path, thing, acts, null, {
            side: [far !== "" ? far : now, beside(path)].filter(said => said !== "")
                .join(" \u00b7 "),
            gone: gone === true
        });
    }

    // How far through each file it has open a program says it is, by the
    // program and then the file's path, as zathura says "p. 41 of 212"
    // (ZathuraProgram.qml), each program's said afresh with each reading
    // of what it has open, so one closed since says nothing.
    property var through: ({})

    function saying(program, sides) {
        through = Object.assign({}, through, { [program]: sides });
    }

    function throughOf(path) {
        return Object.values(through).map(sides => sides[path] ?? "")
            .find(side => side !== "") ?? "";
    }

    // A folder's row, the same, but given `inside`, what it holds, Return
    // steps in to its acts and then those.
    function folder(path, thing, acts, inside) {
        return made("folder", path, thing, acts, inside ?? null, {});
    }

    // A file or a folder kept elsewhere, with no path here until it's
    // fetched, as one in Google's Drive: the same row, keyed and named by
    // `fields` (key, name, side, note, drawer), with no Copy the Path,
    // since it has none.
    function elsewhere(kind, fields, thing, acts, inside) {
        const first = acts[0] ?? null;
        const row = {
            key: fields.key,
            what: kind,
            component: kind,
            name: fields.name,
            glyph: Words.glyph(`path:${kind}`),
            hue: "program",
            side: fields.side ?? "",
            note: fields.note ?? "",
            verb: first?.verb ?? Words.verb(`path:${kind}`),
            run: first?.run ?? (() => {}),
            verbs: () => acts,
            // What d and y take, without what's inside (Launcher.qml's
            // actionsOf), which a folder lists only as it's stepped into.
            operands: () => acts
        };
        if (fields.drawer)
            row.drawer = fields.drawer;
        if (inside) {
            const held = () => acts.concat(inside());
            Object.assign(row, { verb: "more", stays: true, now: false, verbs: held, targets: held });
        }
        return thing(row);
    }

    function made(kind, path, thing, acts, inside, also) {
        const all = () => acts.concat([copying(kind, path, thing)]);
        const first = acts[0] ?? null;
        const row = {
            key: `${kind}:${path}`,
            what: kind,
            component: kind,
            name: named(path),
            glyph: Words.glyph(`path:${kind}`),
            hue: "program",
            side: within(path),
            verb: first?.verb ?? Words.verb(`path:${kind}`),
            run: first?.run ?? (() => {}),
            verbs: all,
            // What d and y take, without what's inside, and its Copy the
            // Path as plain fields, since y over a folder of hundreds asks
            // every row (Launcher.qml's actionsOf).
            operands: () => acts.concat([copying(kind, path, fields => fields)])
        };
        Object.assign(row, also);
        if (inside !== null) {
            const held = () => acts.concat(inside(), [copying(kind, path, thing)]);
            Object.assign(row, { verb: "more", stays: true, now: false, verbs: held, targets: held });
        }
        return thing(row);
    }

    // Copy the Path, the last of every file's and folder's acts.
    function copying(kind, path, thing) {
        const key = `verb:path:${kind}:copy`;
        return thing({
            key: `${key}:${path}`,
            name: Words.name(key),
            verb: Words.verb(key),
            glyph: Words.glyph(key),
            hue: "action",
            copy: path,
            copies: path,
            run: () => Quickshell.clipboardText = path
        });
    }
}
