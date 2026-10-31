pragma Singleton
// What became of each save the launcher's picker staged for Google Drive,
// said as a notice once the google program has sent it or given up
// (Saves.js says which and how). The notices are sent from the shell, as
// Wi-Fi's sign-in is (SignIn.qml), so the act Joel picks comes back here
// through the drive ipc target: Open in Drive, Sign In through pi, Try
// Again, or Save a Copy into a local folder he picks in the launcher. A
// failure's notice is taken back once its file is up or kept elsewhere.
// The staged copy stays until one or the other, so nothing saved is lost.
import qs
import Quickshell
import Quickshell.Io
import QtQuick
import "Saves.js" as Said

Singleton {
    id: root

    // The launcher, whose picker Save a Copy asks (Documents.qml binds it).
    property var launcher: null

    readonly property var uploads: Google.drive.uploads
    onUploadsChanged: tellNew()

    // When the shell started: what was asked before is a restart's, and
    // was told then.
    property real since: 0
    Component.onCompleted: since = Date.now()

    // The saves told lately, by Saves.js's key, so each is told once.
    property var told: []
    readonly property int keep: 50

    function tellNew() {
        for (const upload of Said.untold(uploads, since, told)) {
            told = told.concat([Said.key(upload)]).slice(-keep);
            if ((upload.failed ?? null) === null)
                withdraw(upload.path);
            tell(upload);
        }
    }

    // Sends its notice and runs the act notify-send answers with.
    function tell(upload) {
        const notice = Said.notice(upload, Google.drive.listings);
        const flags = notice.acts.map(act => `--action=${act.key}=${act.name}`)
            .concat(notice.answers ? ["--hint=string:x-answers:save"] : []);
        Niri.spawn(["sh", "-c", saying, "drive-save", notice.summary, notice.body,
            upload.link ?? "", upload.path].concat(flags));
    }

    readonly property string saying: [
        'summary=$1 body=$2 link=$3 path=$4',
        'shift 4',
        'asked=$(notify-send --app-name="Google Drive" "$@" -- "$summary" "$body")',
        'case $asked in',
        '    open) exec google-opens "$link" ;;',
        '    sign-in) exec qs ipc call drive signIn ;;',
        '    again) exec qs ipc call drive again "$path" ;;',
        '    copy) exec qs ipc call drive copy "$path" ;;',
        'esac'
    ].join("\n")

    // A failure's notice, its file up or kept elsewhere now: off the
    // screen if it's there, and read in the history, so it never waits as
    // missed once there's nothing to do about it.
    function withdraw(path) {
        for (const id in Notices.live) {
            const live = Notices.live[id];
            if (Said.failedOf({ app: live.appName, summary: live.summary, body: live.body }, path))
                live.dismiss();
        }
        for (const notice of Notices.history) {
            if (!notice.read && Said.failedOf(notice, path))
                Notices.mark(notice.id, true);
        }
    }

    // Save a Copy: where, asked in the launcher's picker at local folders
    // only, starting where Joel last saved one, else Documents. Picking
    // nothing keeps the staged copy, and the failure's notice, as they are.
    function copy(path) {
        const chooser = Google.chooserOf(root.launcher);
        if (chooser === null)
            return;
        const home = Quickshell.env("HOME") ?? "";
        const last = typeof chooser.savedLastIn === "function" ? chooser.savedLastIn() : "";
        chooser.chooseToSave({
            name: path.slice(path.lastIndexOf("/") + 1),
            folder: last || `${home}/Documents`,
            title: Words.name("verb:attachment:save"),
            local: true
        }, paths => {
            if (paths.length > 0)
                keepAt(path, paths[0]);
        });
    }

    // Copies the staged file where Joel picked, and only once it's there
    // lets the staged copy and its mark go, saying where it went.
    function keepAt(staged, to) {
        const home = Quickshell.env("HOME") ?? "";
        const shown = home !== "" && to.startsWith(`${home}/`) ? `~${to.slice(home.length)}` : to;
        Niri.spawn(["sh", "-c", kept, "drive-copy", staged, to, shown]);
    }

    readonly property string kept: [
        'staged=$1 to=$2 shown=$3',
        'name=${staged##*/}',
        'if ! cp -- "$staged" "$to"; then',
        '    exec notify-send --app-name="Google Drive" -- "Not Saved" \\',
        '        "$name couldn\'t be copied to ${shown%/*}"',
        'fi',
        'rm -f -- "$staged" "${staged%/*}/.$name.drive"',
        'qs ipc call drive kept "$staged"',
        'asked=$(notify-send --app-name="Google Drive" --hint=string:x-answers:save \\',
        '    --action=open="Open" -- "Saved a Copy" "$shown")',
        '[ "$asked" = open ] && exec xdg-open "$to"'
    ].join("\n")

    IpcHandler {
        target: "drive"

        function signIn(): void {
            Sessions.tell("/google-auth", Sessions.home);
        }

        function again(path: string): void {
            Google.upload(path);
        }

        function copy(path: string): void {
            root.copy(path);
        }

        function kept(path: string): void {
            root.withdraw(path);
        }
    }
}
