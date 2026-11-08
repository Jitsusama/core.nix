pragma Singleton
// What the clipboard holds now, for everything that acts on a copy:
// Tuple's joining a call whose link was copied (Tuple.qml), and the
// programs' acts in the launcher that take one, GIMP's picture, Neovim's
// path, Chrome's link and the reader's markdown. One wl-paste watches it
// for the whole shell, so a copy starts one small command rather than one
// for each that reads it, and Copied.js says what the copy is.
//
// Its types are what the program offering it lists, and its text is read
// only when one of them is text, so a picture isn't read as words. A copy
// a password manager marks secret, or the clipboard cleared, has no types
// and no text, so nothing offers to act on what was copied before it.
//
// A path is acted on only while its file is there, which its folder is
// asked, through a folder model that hears the folder change, so a file
// removed after it was copied takes its act with it, and nothing reads
// the file itself, however large.
import Quickshell
import Quickshell.Io
import QtQuick
import Qt.labs.folderlistmodel
import "Copied.js" as Copying

Singleton {
    id: root

    readonly property string home: Quickshell.env("HOME") ?? ""

    // The types the copy is offered as.
    property var types: []
    // Its text, its lines as they were, or "" for a copy with none.
    property string text: ""

    // The type to ask for its picture, or "" while it holds none.
    readonly property string picture: Copying.picture(types)
    // The one web address it is, or "".
    readonly property string link: Copying.link(text)
    // The file's path it names, whether or not the file is there.
    readonly property var named: Copying.place(text, home)
    // The file's path it names, with the line and column to open it at,
    // while the file is there, else null.
    readonly property var place: named !== null && found !== null
        && found.status === FolderListModel.Ready && found.count > 0 ? named : null

    // The folder asked whether the file named is there, made afresh for
    // each copy, so what it said of the last one is never read as this
    // one's while it looks.
    property var found: null

    Instantiator {
        model: root.named === null ? [] : [root.named]
        onObjectAdded: (index, object) => root.found = object
        onObjectRemoved: (index, object) => {
            if (root.found === object)
                root.found = null;
        }

        FolderListModel {
            required property var modelData

            folder: Copying.folderUrl(modelData.folder)
            nameFilters: [Copying.nameFilter(modelData.name)]
            showDirs: false
            showDotAndDotDot: false
            showHidden: true
            caseSensitive: true
        }
    }

    // Each copy as one line: its types, then a tab and its text, each line
    // break as a unit separator, or an empty line for a copy with nothing
    // to read. wl-paste skips a copy that has no type asked for, so it asks
    // for none and gives the command whatever there is, and the command
    // looks at the copy's types to know whether that's text.
    Process {
        running: true
        command: ["wl-paste", "--no-newline", "--watch", "sh", "-c",
            "if [ \"$CLIPBOARD_STATE\" != data ]; then cat > /dev/null; echo; exit; fi;"
            + " types=$(wl-paste --list-types | tr '\\n' ' '); printf '%s\\t' \"$types\";"
            + " if printf '%s' \"$types\" | tr ' ' '\\n'"
            + " | grep -q -x -E 'text/.*|TEXT|STRING|UTF8_STRING';"
            + " then tr '\\n' '\\037'; else cat > /dev/null; fi; echo"]
        stdout: SplitParser {
            onRead: line => {
                const copy = Copying.heard(line);
                root.types = copy.types;
                root.text = copy.text;
            }
        }
    }
}
