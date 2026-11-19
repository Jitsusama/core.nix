// Expect.run, as the QML tests that hand it their cases rely on it: each
// way a case can end, chosen by the word after `--`, since qml ends with the first.
// `expect-run-fails-each-case-that-throws` runs it once for each and reads
// what qml exits with and says.
import QtQuick
import "Expect.js" as Expect

QtObject {
    readonly property string ending: Qt.application.arguments.slice(-1)[0]

    Component.onCompleted: Expect.run(() => {
        if (ending === "fails")
            Expect.same("a case that doesn't hold", 1, 2);
        // What a case throws needn't be an Error, nor anything at all.
        if (ending === "null")
            throw null;
        if (ending === "word")
            throw "a bare word";
        // An Error with no words says what it is, and a value that can't
        // even be made words says what kind of thing it was.
        if (ending === "wordless")
            throw new RangeError("");
        if (ending === "unsayable")
            throw {
                toString() {
                    throw new Error("not this either");
                }
            };
    })
}
