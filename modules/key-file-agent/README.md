# A Key in a File, Served

Not a module: [`units.nix`][units] is a function the modules that keep an
SSH key in a file on a machine with no TPM call, giving the systemd user
units that serve that key alone on `<name>.sock` in the runtime folder, as
ssh-tpm-agent serves a key in the TPM. [yubikey-signing][signing] serves
the agents' key with it, and [file-link-keys][links] each realm's link
key.

[units]: units.nix
[signing]: ../yubikey-signing/home.nix
[links]: ../file-link-keys/home.nix
