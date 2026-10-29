# Chrome's Open Tabs

Chrome's open tabs, for its Recent in the launcher, which lists what's open
first. Chrome says which tabs are open only to an extension, so this is one
of ours, which Chrome installs by itself from the file `../nixos.nix` names.

- `extension/`: the extension itself, as Chrome unpacks it. It asks for the
  tabs, to say which are open and go to or close one, and for native
  messaging, to answer the launcher through a program of ours.
- `key.pem`: the key it's signed with, which fixes its id,
  eefbjjlcekpocpoiobhilnmpabjhmjhm. It signs this extension alone and guards
  nothing, so it lives here in the open.
