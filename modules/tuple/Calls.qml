// Tuple's calls in the launcher, found anywhere by name: each row Tuple
// says can be done now (Tuple.qml's controls), such as muting Joel in the
// call he's in or joining the call whose link was just copied. Tuple's own
// place (People.qml) leads with the same rows. A ringing call and a copied
// link are also first at home, since opening the launcher then is when
// they're wanted.
import qs
import QtQuick

Provider {
    what: "call"

    things: Tuple.controls.map(control => thing(control))

    function home() {
        return things.filter(row => row.key === "tuple:ringing" || row.key === "tuple:join");
    }
}
