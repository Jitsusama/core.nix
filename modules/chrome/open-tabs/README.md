# Chrome's Open Tabs

Chrome's open tabs, live, for its Recent in the launcher, through an
extension of ours, which Chrome installs by itself from the file
`../nixos.nix` names. The launcher asks it through `../recent-sites`,
which lists its tabs where it answers, and Go To and Close It hand it the
ids it gave; only an extension can go to the very tab or close one.
Chrome's own session file says every tab too, and is what's read when
this lists nothing, since a managed Chrome may refuse an extension its
policy doesn't list.

- `extension/`: the extension itself, as Chrome unpacks it. It asks for the
  tabs, to say which are open and go to or close one, and for native
  messaging, to answer the launcher through a program of ours.
- `host/`: that program, chrome-open-tabs, which Chrome starts for the
  extension and the launcher asks on a socket of its own.
- `key.pem`: the key it's signed with, which fixes its id,
  eefbjjlcekpocpoiobhilnmpabjhmjhm. It signs this extension alone and guards
  nothing, so it lives here in the open.
