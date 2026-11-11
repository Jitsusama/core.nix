# The Keys VM Tests

Where a machine keeps its secrets and keys, on machines booted in QEMU:
the keyring that holds what programs store, the passkeys and security keys
the TPM makes through linux-id, and the keys commits are signed with. Each
test's opening comment says what stands in for a browser, a prompt or Joel.

| File                               | Holds                                                              |
| ---------------------------------- | ------------------------------------------------------------------ |
| `gnome-keyring.nix`, `-vm-test.py` | a secret kept in the keyring across logins, asking for nothing     |
| `linux-id.nix`, `-vm-test.py`      | passkeys and security keys made in the TPM, and signed in with     |
| `linux-id-askpass.sh`              | that test's askpass, answering as Joel does and noting each ask    |
| `passkey.py`                       | that test's browser, through Yubico's FIDO library                 |
| `signing.nix`, `-vm-test.py`       | Joel's and the agents' signing keys in the TPM, each commit signed |
| `signing-askpass.sh`               | that test's askpass, giving the PIN and each confirmation          |
