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

// The process number a program's name can end with, in brackets.
const PROCESS_NUMBER = /\s*\(\d+\)$/;
// The folders before a program's name, when it's given as a path.
const PROCESS_FOLDERS = /^.*\//;
// ssh-agent's and ssh-tpm-agent's, before a key confirmed on use signs:
// "Allow use of key NAME?", its fingerprint, and ssh-tpm-agent says which
// processes asked, as "ssh-keygen (1234) ← git ← zsh".
const KEY_USE = new RegExp("^Allow use of key (.+?)\\?(?:\\nKey fingerprint [^\\n]*)?"
    + "(?:\\nRequested by (.+))?$");
// linux-id's, through pinentry-askpass, before a key the TPM holds signs: a
// passkey made for a site or signed in with.
const PASSKEY = /^FIDO2 (Register|Authenticate): (.+)$/;
// linux-id's for a plain security key, which isn't told the site.
const SECURITY_KEY = /^FIDO Confirm (Register|Auth)$/;
// ssh's, while a security key waits for a touch.
const PRESENCE = /^Confirm user presence for key /;
// ssh's, for a security key's PIN: "Enter PIN for ED25519-SK key PATH: ".
const PIN = /^Enter PIN for (\S+) key (.+?):?$/;
// ssh-tpm-agent's, for a key sealed with a PIN: "Enter passphrase for
// (NAME): ", and ssh's for a key file: "Enter passphrase for key PATH: ".
const PASSPHRASE = /^Enter passphrase for (?:\((.+)\)|key (.+?)):?$/;
// ssh's, for a host it hasn't met, which is answered in words.
const YES_OR_NO = /\(yes\/no/;
// The colon a question can end with, which the detail leaves off.
const LAST_COLON = /:$/;

// A program's name, without the path or the process number it's given as.
function named(process) {
    return process.trim().replace(PROCESS_NUMBER, "").replace(PROCESS_FOLDERS, "");
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

// A prompt as Askpass.qml draws it: the title and detail it says, and what
// it asks, as allowed(), told() or secret() give it.
function shown(title, detail, mono, asking) {
    return { title, detail, mono, field: asking.field, shows: asking.shows, keys: asking.keys };
}

// What a confirmation asks: no field, only a key to allow and one to refuse.
function allowed() {
    return { field: false, shows: false, keys: [["\u21b5", "allow"], ["Esc", "refuse"]] };
}

// What a prompt that only says something asks: nothing, with no keys.
function told() {
    return { field: false, shows: false, keys: [] };
}

// What a question asks: a field, whose words show when they aren't a secret.
function secret(shows) {
    return { field: true, shows, keys: [["\u21b5", "enter"], ["Esc", "refuse"]] };
}

// A key confirmed on use, by the first of the processes that asked, or by
// the asker when none are named, for the rest of them.
function keyUsed(use, who, home) {
    const chain = (use[2] ?? "").split(" \u2190 ").map(named).filter(name => name !== "");
    const user = chain.length > 0 ? chain[0] : who;
    return shown([program(user), plain(" wants to use "), thing(homely(use[1], home))],
        chain.length > 1 ? `for ${chain.slice(1).join(" \u2190 ")}` : "", chain.length > 1,
        allowed());
}

// A program asking for a yes: to use a key, a passkey or the TPM's security
// key, or whatever else it says.
function confirmation(text, who, home) {
    const use = KEY_USE.exec(text);
    if (use) {
        return keyUsed(use, who, home);
    }
    const passkey = PASSKEY.exec(text);
    if (passkey) {
        return shown(passkey[1] === "Register"
            ? [plain("Make a passkey for "), thing(passkey[2])]
            : [plain("Sign in to "), thing(passkey[2]), plain(" with its passkey")],
            "", false, allowed());
    }
    const key = SECURITY_KEY.exec(text);
    if (key) {
        return shown([plain(key[1] === "Register" ? "Add the TPM as a security key"
            : "Sign in with the TPM's security key")], "", false, allowed());
    }
    return shown([program(who), plain(" asks")], text, false, allowed());
}

// A program saying something that waits on something else, such as a touch.
function telling(text, who) {
    if (PRESENCE.test(text)) {
        return shown([plain("Touch the security key for "), program(who)], "", false, told());
    }
    return shown([program(who), plain(" says")], text, false, told());
}

// A program asking for a security key's PIN, a key's passphrase or PIN, or
// a question answered in words.
function question(text, who, home) {
    const pin = PIN.exec(text);
    if (pin) {
        return shown([program(who), plain(" needs the security key's PIN")],
            `${pin[1]} ${homely(pin[2], home)}`, true, secret(false));
    }
    const passphrase = PASSPHRASE.exec(text);
    if (passphrase) {
        return shown([program(who), plain(" needs "), thing(homely(passphrase[1] ?? passphrase[2],
            home)), plain(passphrase[1] ? "'s PIN" : "'s passphrase")], "", false, secret(false));
    }
    return shown([program(who), plain(" asks")], text.replace(LAST_COLON, ""), false,
        secret(YES_OR_NO.test(text)));
}

function read(kind, asker, prompt, home) {
    const text = prompt.trim();
    const who = named(asker) || "a program";
    if (kind === "confirm") {
        return confirmation(text, who, home);
    }
    if (kind === "none") {
        return telling(text, who);
    }
    return question(text, who, home);
}
