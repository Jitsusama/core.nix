// What a program asking through SSH's askpass wants, read from the words it
// gives (Askpass.qml), kept apart from drawing so each prompt is tested as
// it reads (tests/askpass.qml). The kind is SSH_ASKPASS_PROMPT's: "confirm"
// asks for a yes, "none" says something that waits on something else, such
// as a touch, and anything else asks for a secret. The asker is the program
// that ran the askpass, as /proc names it.
//
// A prompt comes back as a title, in parts, each plain, a program or a
// thing; a detail, in the monospace face when it's a path or programs; and
// what's asked: whether a field shows, whether what's typed shows, and what
// the keys do.
.pragma library

// A program's name, without the path or the process number it's given as.
function named(process) {
    return process.trim().replace(/\s*\(\d+\)$/, "").replace(/^.*\//, "");
}

// A path with the home folder as ~, as Joel writes it.
function homely(path, home) {
    return home !== "" && path.startsWith(`${home}/`) ? `~${path.slice(home.length)}` : path;
}

function program(text) {
    return { text, as: "program" };
}

function thing(text) {
    return { text, as: "thing" };
}

function plain(text) {
    return { text, as: "plain" };
}

function read(kind, asker, prompt, home) {
    const text = prompt.trim();
    const who = named(asker) || "a program";
    const asked = keys => ({ field: keys !== null, shows: false, keys: keys ?? [] });
    const secret = asked([["\u21b5", "enter"], ["Esc", "refuse"]]);

    if (kind === "confirm") {
        const allow = {
            field: false, shows: false, keys: [["\u21b5", "allow"], ["Esc", "refuse"]]
        };
        // ssh-agent's and ssh-tpm-agent's, before a key confirmed on use signs:
        // "Allow use of key NAME?", its fingerprint, and ssh-tpm-agent says
        // which processes asked, as "ssh-keygen (1234) ← git ← zsh".
        const use = new RegExp("^Allow use of key (.+?)\\?(?:\\nKey fingerprint [^\\n]*)?"
            + "(?:\\nRequested by (.+))?$").exec(text);
        if (use) {
            const chain = (use[2] ?? "").split(" \u2190 ").map(named).filter(name => name !== "");
            const user = chain.length > 0 ? chain[0] : who;
            return Object.assign({
                title: [program(user), plain(" wants to use "), thing(homely(use[1], home))],
                detail: chain.length > 1 ? `for ${chain.slice(1).join(" \u2190 ")}` : "",
                mono: chain.length > 1
            }, allow);
        }
        // linux-id's, through pinentry-askpass, before a key the TPM holds
        // signs: a passkey made for a site or signed in with, or a plain
        // security key, which isn't told the site.
        const passkey = /^FIDO2 (Register|Authenticate): (.+)$/.exec(text);
        if (passkey)
            return Object.assign({
                title: passkey[1] === "Register"
                    ? [plain("Make a passkey for "), thing(passkey[2])]
                    : [plain("Sign in to "), thing(passkey[2]), plain(" with its passkey")],
                detail: "", mono: false
            }, allow);
        const key = /^FIDO Confirm (Register|Auth)$/.exec(text);
        if (key)
            return Object.assign({
                title: [plain(key[1] === "Register" ? "Add the TPM as a security key"
                    : "Sign in with the TPM's security key")],
                detail: "", mono: false
            }, allow);
        return Object.assign({ title: [program(who), plain(" asks")], detail: text, mono: false },
            allow);
    }

    if (kind === "none") {
        // ssh's, while a security key waits for a touch.
        if (/^Confirm user presence for key /.test(text))
            return Object.assign({
                title: [plain("Touch the security key for "), program(who)],
                detail: "", mono: false
            }, asked(null));
        return Object.assign({ title: [program(who), plain(" says")], detail: text, mono: false },
            asked(null));
    }

    // ssh's, for a security key's PIN: "Enter PIN for ED25519-SK key PATH: ".
    const pin = /^Enter PIN for (\S+) key (.+?):?$/.exec(text);
    if (pin)
        return Object.assign({
            title: [program(who), plain(" needs the security key's PIN")],
            detail: `${pin[1]} ${homely(pin[2], home)}`, mono: true
        }, secret);
    // ssh-tpm-agent's, for a key sealed with a PIN: "Enter passphrase for
    // (NAME): ", and ssh's for a key file: "Enter passphrase for key PATH: ".
    const passphrase = /^Enter passphrase for (?:\((.+)\)|key (.+?)):?$/.exec(text);
    if (passphrase)
        return Object.assign({
            title: [program(who), plain(" needs "), thing(homely(passphrase[1] ?? passphrase[2],
                home)), plain(passphrase[1] ? "'s PIN" : "'s passphrase")],
            detail: "", mono: false
        }, secret);
    // ssh's, for a host it hasn't met, which is answered in words.
    const words = /\(yes\/no/.test(text);
    return Object.assign({
        title: [program(who), plain(" asks")],
        detail: text.replace(/:$/, ""), mono: false
    }, secret, { shows: words });
}
