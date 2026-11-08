// Neovim's own acts in Programs › Neovim (programActs, which
// Applications.qml asks every provider that has it), each found from the
// top by its name too (foundActs). Open a File by Name steps into the git
// repositories under ~/src, the one whose git last wrote first, each
// saying how long ago, and on into a repository's files, the one changed
// last first, each found by typing as any step is (Repositories.qml).
// Open the Copied Path comes after it, since it comes and goes: it's
// there only while the clipboard holds an existing file's path, said
// beside it, and opens it at the line, and the column after that, the
// copy gives (Copied.qml), as a compiler's or ripgrep's "main.rs:12:5"
// says them.
//
// Either opens Neovim in a new window of the kitty used last, through
// kitty's remote control, as kitty's Recent opens a folder, in the
// repository or the file's folder, or in a new kitty there while none
// runs (NeovimOpening.js).
pragma ComponentBehavior: Bound

import qs
import QtQuick
import "../Copied.js" as Copying
import "Ago.js" as Ago
import "NeovimOpening.js" as Opening
import "Repositories.js" as Repos

Provider {
    id: provider

    // The desktop entry Neovim's program row is made from.
    readonly property string program: "nvim"

    // The existing file's path the clipboard holds, or null.
    readonly property var place: Copied.place
    // So Programs › Neovim, held while it's copied, draws its act.
    onPlaceChanged: changed()

    // Found from the top by their names, Open the Copied Path only while
    // there's one.
    foundActs: [{ program: program, key: "verb:nvim:find" }]
        .concat(place !== null ? [{ program: program, key: "verb:nvim:copied" }] : [])

    readonly property Repositories repositories: Repositories {
        folder: provider.settings.folder ?? ""
        fd: provider.settings.fd ?? "fd"
        stat: provider.settings.stat ?? "stat"
        showing: provider.launcher?.open ?? false
        onChanged: provider.changed()
    }

    function programActs(asked) {
        if (asked !== program)
            return [];
        return [finding()].concat(place !== null ? [copied()] : []);
    }

    // Open a File by Name, saying how many repositories it holds once
    // they're read, and, stepped into while there are none, that it's
    // looking, or that there are none.
    function finding() {
        const key = "verb:nvim:find";
        return Repos.saysEmpty(thing({
            key: key,
            name: Words.name(key),
            verb: Words.verb(key),
            glyph: Words.glyph(key),
            hue: "action",
            side: provider.repositories.read ? String(provider.repositories.listed.length) : "",
            ask: Words.of(key).ask ?? "",
            nested: true,
            targets: () => {
                provider.repositories.ask();
                const now = Date.now();
                return provider.repositories.listed.map(one => provider.repository(one, now));
            }
        }), {
            none: Words.of(key).empty,
            looking: Words.of(key, "looking").empty,
            folder: Files.shown(provider.repositories.folder)
        }, () => provider.repositories.read);
    }

    // A repository as the folder it is, saying when its git last wrote,
    // which Return steps into, its files listed there.
    function repository(one, now) {
        const row = Files.folder(one.path, thing, [], () => provider.filesOf(one.path));
        return Object.assign(row, { side: Ago.ago(one.at, now), nested: true });
    }

    // A repository's files, each as the file it is, Return opening it.
    function filesOf(folder) {
        provider.repositories.askFiles(folder);
        return (provider.repositories.files[folder] ?? []).map(file => Files.file(file.path,
            thing, [provider.opening(file.path, folder, 0, 0)]));
    }

    // Open the Copied Path, saying the path and its line as copied.
    function copied() {
        const key = "verb:nvim:copied";
        const where = place;
        return thing({
            key: key,
            name: Words.name(key),
            verb: Words.verb(key),
            glyph: Words.glyph(key),
            hue: "action",
            side: Copying.placeSaid(where, Files.home),
            // What's copied when it runs, which a row found from the top,
            // made as the act came, may not have changed with.
            run: () => {
                const now = Copied.place;
                if (now !== null)
                    provider.open(now.path, now.folder, now.line, now.column);
            }
        });
    }

    // A file's Open, Recent's own word for opening it in the program.
    function opening(path, folder, line, column) {
        const key = "verb:path:file:open";
        return thing({
            key: `${key}:${path}`,
            name: Words.name(key),
            verb: Words.verb(key),
            glyph: Words.glyph(key),
            hue: "action",
            run: () => provider.open(path, folder, line, column)
        });
    }

    function open(path, folder, line, column) {
        Niri.spawn(Opening.inKitty(Opening.kitty(Object.values(Niri.windows)), folder,
            Opening.neovim(path, line, column)));
    }
}
