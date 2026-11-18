// Has the sign-in (SignIn.qml) tell of a network wanting one once, and take
// its notice back once it stops wanting one, through Quickshell itself, with
// Wifi's cards, the notices, niri and the report stood in. The notice is a
// notify-send that niri starts, so it arrives a moment after it's told: a
// network signed in within that moment had its notice arrive after it was
// taken back, and stay (2026-11-15). One arriving for a network that no
// longer wants a sign-in is taken back as it comes. It exits 0 once every
// case holds, and otherwise 1, saying which didn't.
import qs
import Quickshell
import QtQuick
import "Expect.js" as Expect

ShellRoot {
    Component.onCompleted: {
        try {
            toldOnce();
            keptWhileWanted();
            takenBackLate();
            // On the next turn, once the shell's loaded to hear it.
            Qt.callLater(Qt.exit, 0);
        } catch (error) {
            Expect.fail(error.message);
        }
    }

    // Wi-Fi's one card, on Lantern, reaching as `reach` says.
    function lantern(reach) {
        return [{ name: "wlan0", network: "Lantern", reach: reach }];
    }

    // Lantern's notice as the notices hold it once it's arrived, open and
    // unread, saying whether it was dismissed.
    function arrived(id) {
        const notification = { appName: "Wi-Fi", summary: "Lantern wants a sign-in" };
        notification.dismissed = false;
        notification.dismiss = () => {
            notification.dismissed = true;
        };
        const notice = { id: id, app: "Wi-Fi", summary: notification.summary, read: false };
        Notices.live = { [id]: notification };
        Notices.history = [notice];
        Notices.came(notification, notice);
        return notification;
    }

    function withdrawn() {
        return Report.said.filter(said => said === "withdrew sign-in Lantern").length;
    }

    // A network wanting a sign-in is told of once, however often it's
    // followed or checked again.
    function toldOnce() {
        Expect.same("nothing wanting a sign-in, nothing is told", SignIn.told, []);
        Wifi.cards = lantern("signIn");
        SignIn.follow();
        Expect.same("a network wanting a sign-in is told of once", Niri.spawned.length, 1);
        Wifi.cards = lantern("checking");
        Expect.same("checked again, it's told of no more", Niri.spawned.length, 1);
    }

    // Its notice arriving while it still wants a sign-in stays.
    function keptWhileWanted() {
        const notification = arrived(7);
        Expect.expect("a notice arriving while its network wants a sign-in stays",
            !notification.dismissed && Notices.marked.length === 0,
            JSON.stringify(Notices.marked));
    }

    // Signed in, its notice is taken back, and one arriving only after that
    // is taken back as it comes.
    function takenBackLate() {
        Wifi.cards = lantern("online");
        Expect.same("online, its sign-in is taken back", withdrawn(), 1);
        Notices.marked = [];
        const late = arrived(8);
        Expect.expect("a notice arriving after its network was signed in is taken back",
            late.dismissed && JSON.stringify(Notices.marked) === JSON.stringify([[8, true,
                "withdrawn"]]),
            JSON.stringify({ dismissed: late.dismissed, marked: Notices.marked }));
    }
}
