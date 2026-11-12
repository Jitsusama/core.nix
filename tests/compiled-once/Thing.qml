// A type of the shell's own folder, for tests/compiled-once.sh, which
// rewrites this file to see that Qt compiles what changed.
import QtQuick
import "kept.js" as Kept

QtObject {
    readonly property string said: Kept.word()
}
