// A stand-in People source for the desktop's VM tests, as a machine's own
// repository may bring one (Vault's): it keeps a page for Grace, as the
// test says in the runtime folder's pages file. With no file it knows
// nothing, so no other test sees it; "looking" has it still finding her
// page, and "found" has it found, so a test can step into her while her
// page is being found and watch Open in Pages light where it stood.
import qs
import Quickshell
import Quickshell.Io
import QtQuick

PersonSource {
    id: source

    called: "Pages"

    // What the test said: "", "looking" or "found".
    property string said: ""

    function page(person) {
        return said === "found" && grace(person) ? "https://pages.example.com/grace" : "";
    }

    function looking(person) {
        return said === "looking" && grace(person);
    }

    function grace(person) {
        return person.emails.includes("grace@example.com");
    }

    // Each change is a remake of everyone People lists, so only a change
    // in what the test said asks for one.
    function heard(now) {
        if (now === said)
            return;
        said = now;
        place?.changed();
    }

    FileView {
        id: told

        path: `${Quickshell.env("XDG_RUNTIME_DIR")}/pages`
        watchChanges: true
        printErrors: false
        onFileChanged: reload()
        onLoaded: source.heard(told.text().trim())
        onLoadFailed: source.heard("")
    }
}
