.pragma library

// The shape of a mesh address, as the launcher reads one wherever a session
// is named by it: Start New Work's cell it was offered from (Sprouting.js),
// and the session Explain It asks the node about (Explaining.js). One to
// twelve capital letters and digits, in ASCII, as the notices daemon's
// `address_shaped` and mycelium-link's `addressed` read one too.
// modules/mycelium/site/tests/addresses.txt holds what is one and what
// isn't, and tests/address.qml judges this copy by it, so none of the copies
// drifts from the others unseen.
const SHAPE = /^[A-Z0-9]{1,12}$/u;

// Whether `text` is a mesh address: only a string, and only of that shape.
function shaped(text) {
    return typeof text === "string" && SHAPE.test(text);
}
