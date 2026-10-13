// What the askpass prompt says for each program's words, run in Qt's own
// JavaScript engine, the one the shell runs it in. The
// `the-askpass-says-who-asks-for-what` check copies Askpass.js beside this
// file and runs `qml -platform offscreen askpass.qml`. Each case gives what a
// program sends and what the prompt has to say.
import QtQuick
import "Askpass.js" as Askpass

QtObject {
    readonly property string home: "/home/joel"

    function said(read) {
        const title = read.title.map(part => part.as === "plain" ? part.text
            : part.as === "program" ? `[${part.text}]` : `*${part.text}*`).join("");
        const keys = read.keys.map(key => key.join(" ")).join(", ");
        return `${title} | ${read.detail}${read.mono ? " (mono)" : ""} | `
            + `${read.field ? read.shows ? "words" : "secret" : "no field"} | ${keys}`;
    }

    Component.onCompleted: {
        const failures = [];
        let asked = 0;
        const expect = (what, kind, asker, prompt, wanted) => {
            asked++;
            const got = said(Askpass.read(kind, asker, prompt, home));
            if (got !== wanted)
                failures.push(`${what} said\n  ${got}\nnot\n  ${wanted}`);
        };

        // ssh-tpm-agent, before Joel's signing key signs a commit.
        expect("a signature", "confirm", "ssh-tpm-agent",
            "Allow use of key joel-signing?\nKey fingerprint SHA256:abc.\n"
                + "Requested by ssh-keygen (1234) \u2190 tpm-sign \u2190 git \u2190 zsh",
            "[ssh-keygen] wants to use *joel-signing* | for tpm-sign \u2190 git \u2190 zsh (mono)"
                + " | no field | \u21b5 allow, Esc refuse");
        expect("a signature nobody's named for", "confirm", "ssh-tpm-agent",
            "Allow use of key joel-signing?\nKey fingerprint SHA256:abc.",
            "[ssh-tpm-agent] wants to use *joel-signing* |  | no field | \u21b5 allow, Esc refuse");
        expect("ssh-agent's, with a key's path", "confirm", "/usr/bin/ssh-agent",
            "Allow use of key /home/joel/.ssh/id_ed25519?\nKey fingerprint SHA256:abc.",
            "[ssh-agent] wants to use *~/.ssh/id_ed25519* |  | no field"
                + " | \u21b5 allow, Esc refuse");
        expect("a yes it can't read", "confirm", "thing", "Really?",
            "[thing] asks | Really? | no field | \u21b5 allow, Esc refuse");

        // The keys' PINs.
        expect("a TPM key's PIN", "", "ssh-tpm-agent", "Enter passphrase for (joel-signing): ",
            "[ssh-tpm-agent] needs *joel-signing*'s PIN |  | secret | \u21b5 enter, Esc refuse");
        expect("a security key's PIN", "", "ssh",
            "Enter PIN for ED25519-SK key /home/joel/.ssh/id_ed25519_sk: ",
            "[ssh] needs the security key's PIN | ED25519-SK ~/.ssh/id_ed25519_sk (mono)"
                + " | secret | \u21b5 enter, Esc refuse");
        expect("a key file's passphrase", "", "ssh",
            "Enter passphrase for key /home/joel/.ssh/id_rsa: ",
            "[ssh] needs *~/.ssh/id_rsa*'s passphrase |  | secret | \u21b5 enter, Esc refuse");

        // A touch, which nothing typed answers.
        expect("a touch", "none", "ssh",
            "Confirm user presence for key ED25519-SK SHA256:abc",
            "Touch the security key for [ssh] |  | no field | ");
        expect("something else to wait on", "none", "", "Wait for it",
            "[a program] says | Wait for it | no field | ");

        // A host ssh hasn't met is answered in words, which show.
        expect("a new host", "", "ssh",
            "The authenticity of host 'x' can't be established.\n"
                + "Are you sure you want to continue connecting (yes/no/[fingerprint])? ",
            "[ssh] asks | The authenticity of host 'x' can't be established.\n"
                + "Are you sure you want to continue connecting (yes/no/[fingerprint])?"
                + " | words | \u21b5 enter, Esc refuse");
        expect("anything else", "", "", "Password: ",
            "[a program] asks | Password | secret | \u21b5 enter, Esc refuse");

        // Said as warnings, since Qt keeps ordinary logging quiet.
        for (const failure of failures)
            console.warn(failure);
        console.warn(failures.length > 0 ? `${failures.length} went wrong of ${asked}`
            : `all ${asked} held`);
        Qt.exit(failures.length > 0 ? 1 : 0);
    }
}
