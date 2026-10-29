// The Google account signed out, which wants Joel (Google.qml's account):
// true while Google refuses the sign-in the program reads, until he signs
// in again, when its meetings and mail come back. It's said in Now and the
// margin only, never as a notice, since it comes true while he's away as
// often as not and waits for him. Return signs in again, in pi, whose
// Google sign-in (/google-auth) writes the file the program reads; the
// program then hears it within a minute.
import qs
import QtQuick

Piece {
    active: Google.account === "signedOut"
    kind: "wants"
    wants: true
    hue: "program"
    title: "Google"
    value: "signed out"
    hint: "Google signed out, so no meetings or mail until you sign in again"
    context: "sign in again in pi"
    actions: [{
        glyph: Theme.glyph.signIn,
        name: "Sign In Again",
        verb: "sign in",
        run: () => Sessions.tell("/google-auth", Sessions.home)
    }]
}
