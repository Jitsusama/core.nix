// A stand-in People source for the desktop's VM tests that knows a crowd,
// as Tuple knows every contact of a company: as many people as the test
// writes in the runtime folder's crowd file, "<count> <round>", each
// round saying them all anew, as Slack's answers each make everyone
// People knows again. With no file, or a count of 0, it knows nobody, so
// no other test sees it.
import qs
import Quickshell
import Quickshell.Io
import QtQuick

PersonSource {
    id: source

    called: "Crowd"

    // What the test said last, so a file read again unchanged says nothing.
    property string said: ""

    function heard(now) {
        if (now === said)
            return;
        said = now;
        const count = Number(now.split(" ")[0]) || 0;
        const crowd = [];
        for (let at = 0; at < count; at += 1)
            crowd.push({ emails: [`crowd${at}@example.com`], name: `Crowd Member ${at}` });
        people = crowd;
        // Said, so a test knows each round was heard.
        Report.say(`crowd ${now === "" ? "none" : now}`);
    }

    FileView {
        id: told

        path: `${Quickshell.env("XDG_RUNTIME_DIR")}/crowd`
        watchChanges: true
        printErrors: false
        onFileChanged: reload()
        // A file read empty is one caught being written, so only a file
        // that isn't there says it knows nobody.
        onLoaded: {
            const now = told.text().trim();
            if (now !== "")
                source.heard(now);
        }
        onLoadFailed: source.heard("")
    }
}
