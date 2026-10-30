# Chrome's Open Tabs

Chrome's open tabs, live, for its Recent in the launcher, through an
extension of ours, which Chrome installs by itself from the file
`../nixos.nix` names. Chrome's own session file says every tab too, and
is what the launcher reads (`../recent-sites`), since a managed Chrome may
refuse an extension its policy doesn't list; only an extension can go to
the very tab or close one, which is what this is for where it's allowed.

- `extension/`: the extension itself, as Chrome unpacks it. It asks for the
  tabs, to say which are open and go to or close one, and for native
  messaging, to answer the launcher through a program of ours.
- `host/`: that program, chrome-open-tabs, which Chrome starts for the
  extension and the launcher asks on a socket of its own.
- `key.pem`: the key it's signed with, which fixes its id,
  eefbjjlcekpocpoiobhilnmpabjhmjhm. It signs this extension alone and guards
  nothing, so it lives here in the open.
