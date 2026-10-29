// Tuple's calls in the launcher: each row Tuple says can be done now
// (Tuple.qml's controls), such as muting Joel in the call he's in or
// joining the call whose link was just copied, found anywhere by name,
// and listed as Tuple's own acts inside Programs › Tuple (programActs,
// which Applications.qml asks every provider that has it). Inside, as
// VKCF3Z's design orders them (RSCH-20261028-2NK1E5, Components): opening
// Joel's room first, so Return twice does it on purpose, since Tuple has
// no window of its own and its row steps in; then the call that rings or
// is live, showing it, whether Joel's Muted, sharing and leaving; then
// joining the call whose link is copied; and copying his room's link
// last of all, after Tuple's Recent, Notices and Shortcuts, as every Copy
// is (Applications.qml). A ringing or live call is in Now as well.
import qs
import QtQuick

Provider {
    id: provider

    what: "call"

    // Tuple's desktop entry hides itself, so Programs lists it for these.
    hiddenPrograms: ["tuple"]

    things: Tuple.controls.map(control => thing(control))
    // So Programs › Tuple, held while a call starts or ends, is drawn again.
    onThingsChanged: changed()

    // Where each of Tuple's rows goes inside it, by its key.
    readonly property var places: ["tuple:open", "tuple:ringing", "tuple:show", "tuple:mute",
        "tuple:share", "tuple:leave", "tuple:join", "tuple:link"]

    function programActs(program) {
        if (program !== "tuple")
            return [];
        return things.slice().sort((a, b) => places.indexOf(a.key) - places.indexOf(b.key));
    }
}
