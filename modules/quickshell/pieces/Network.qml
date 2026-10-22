// The network, as wifid tells it: wired, the Wi-Fi network's name, or
// offline. It's true, in the warning colour, while the machine is offline,
// while its network wants a sign-in, and while the network reaches no
// further than itself, and says which; signing in is one of its actions.
// wifid sends each change as it happens, so nothing is asked.
import qs
import QtQuick

Piece {
    readonly property var card: Wifi.joined
    readonly property string reach: card ? card.reach ?? "" : ""

    present: Wifi.described
    active: !Wifi.wired && (card === null || reach === "signIn" || reach === "limited")
    role: active ? "warning" : "text"
    wants: !Wifi.wired && reach === "signIn"
    sign: Wifi.wired ? "wired" : card === null ? "offline" : settings.sign ?? ""
    // Only what's wrong: joined is the glyph alone, and its name is in
    // Wi-Fi.
    glance: Wifi.wired || card === null ? ""
        : reach === "signIn" ? "sign in" : reach === "limited" ? "limited" : ""
    value: {
        if (Wifi.wired)
            return "wired";
        if (card === null)
            return "offline";
        if (reach === "signIn")
            return "sign in";
        if (reach === "limited")
            return `${card.network}, limited`;
        return card.network;
    }
    title: "Wi-Fi"
    hint: {
        if (reach === "signIn")
            return `${card.network} wants a sign-in`;
        if (reach === "limited")
            return `${card.network} reaches no further than itself`;
        return value;
    }
    context: reach === "signIn" ? card.signIn : ""
    actions: (reach === "signIn" ? [{
        glyph: String.fromCodePoint(0xf05a9),
        name: `Sign in to ${card.network}`,
        run: () => SignIn.open()
    }] : []).concat(card === null ? [] : [{
        glyph: String.fromCodePoint(0xf0450),
        name: "Check it again",
        run: () => Wifi.check()
    }])
}
