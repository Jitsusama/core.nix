// Tuple's calls in the launcher, found anywhere by name: each row Tuple
// says can be done now (Tuple.qml's controls), such as muting Joel in the
// call he's in or joining the call whose link was just copied. They live
// in Tuple's own place (People.qml, the people provider) as well, which
// leads with them, so a search that finds the place shows them only
// there. A ringing call and a copied link are also first at home, since
// opening the launcher then is when they're wanted.
import qs
import QtQuick

Provider {
    what: "call"

    things: Tuple.controls.map(control => thing(Object.assign({ inside: "people" }, control)))

    function home() {
        return things.filter(row => row.key === "tuple:ringing" || row.key === "tuple:join");
    }
}
