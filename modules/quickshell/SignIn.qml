// Signing in to a network that wants it: the one door the notice, the
// launcher's row and the network piece all open, and only when Joel asks,
// never by itself. One notice goes out for each network found wanting a
// sign-in, and another only once it has stopped wanting one and wants it
// again, as when a day's pass runs out. The door is wifid's sign-in window,
// which reaches the page through the card whatever a VPN is doing, and
// closes once the network lets the machine through.
pragma Singleton
import Quickshell
import Quickshell.Io
import QtQuick

Singleton {
    id: root

    readonly property var card: Wifi.signingIn
    readonly property string network: card ? card.network : ""
    readonly property string page: card ? card.signIn ?? "" : ""

    function open(): void {
        if (page !== "")
            Niri.spawn(["wifid-sign-in"]);
    }

    // The networks told of, which want a sign-in still.
    property var told: []

    function follow(): void {
        // One being checked again still wants it until the check says not,
        // or its notice would come and go with every check.
        const still = card => card.reach === "signIn"
            || (card.reach === "checking" && told.includes(card.network));
        const wanting = Wifi.cards.filter(still).map(card => card.network);
        for (const network of told.filter(network => !wanting.includes(network)))
            withdraw(network);
        told = told.filter(network => wanting.includes(network));
        for (const network of wanting.filter(network => !told.includes(network))) {
            told = told.concat([network]);
            tell(network);
        }
    }

    // A network that no longer wants a sign-in takes its notice back: off
    // the screen if it's still there, and read in the history, so it never
    // waits there as missed once there's nothing to do about it.
    function withdraw(network: string): void {
        const summary = `${network} wants a sign-in`;
        const ours = notice => notice.appName === "Wi-Fi" && notice.summary === summary;
        for (const id in Notices.live)
            if (ours(Notices.live[id]))
                Notices.live[id].dismiss();
        for (const notice of Notices.history)
            if (notice.app === "Wi-Fi" && notice.summary === summary && !notice.read)
                Notices.mark(notice.id, true);
        Report.say(`withdrew sign-in ${network}`);
    }

    // The notice's action comes back through the shell, so the door stays
    // this one.
    function tell(network: string): void {
        Niri.spawn(["sh", "-c", [
            'asked=$(notify-send --app-name=Wi-Fi --action=sign-in="Sign in" \\',
            '    "$1 wants a sign-in" "Nothing reaches past it until you sign in.")',
            '[ "$asked" = sign-in ] && exec qs ipc call wifi signIn'
        ].join("\n"), "sign-in", network]);
    }

    Connections {
        target: Wifi

        function onCardsChanged() {
            root.follow();
        }
    }

    IpcHandler {
        target: "wifi"

        function signIn(): void {
            root.open();
        }
    }
}
