// A stand-in launcher source for the desktop's VM tests, ending each search
// at the top as Search Slack does (Provider.qml's last), as the runtime
// folder's search-ends file says: "asking" ends it with Look Further for
// what's typed, holding only a dimmed stand-in while it's asked, "found"
// with what was found, and "none", or no file, with nothing, so no other
// test sees it.
import qs
import Quickshell
import Quickshell.Io
import QtQuick

Provider {
    id: provider

    what: "search"

    // What the test said: "none", "asking" or "found".
    property string said: "none"

    function last(typed) {
        if (said === "none")
            return null;
        const words = typed.trim();
        return thing({
            key: "verb:search-ends",
            name: `Look Further for ${words}`,
            verb: "more",
            glyph: Theme.glyph.launcher,
            hue: "action",
            nested: true,
            verbs: () => provider.said === "found"
                ? [thing({ key: `search-ends:${words}`, name: `Found ${words}`, verb: "copy" })]
                : [thing({ key: "search-ends:asking", name: "Still Looking", gone: true })]
        });
    }

    function heard(now) {
        if (now === said)
            return;
        said = now;
        // Said, so a test knows it was heard before it looks for a row.
        Report.say(`search ends ${now}`);
        changed();
    }

    FileView {
        id: told

        path: `${Quickshell.env("XDG_RUNTIME_DIR")}/search-ends`
        watchChanges: true
        printErrors: false
        onFileChanged: reload()
        // A file read empty is one caught being written.
        onLoaded: {
            const now = told.text().trim();
            if (now !== "")
                provider.heard(now);
        }
        onLoadFailed: provider.heard("none")
    }
}
