# sshd asks a quiet caller every 15 s whether it's still there, and lets it
# go after four go unanswered. A caller that went to sleep, or lost its
# network, otherwise holds its session for the two hours TCP takes to
# notice, and what it started runs on beside it. Its own ssh asks the same
# from its side. Whether sshd runs at all is the machine's to say.
#
# Only keys sign in. NixOS's sshd takes an account's password too, and a
# laptop opens sshd's port on whatever network it's on, where a password
# can be guessed and a key, in a TPM or on a YubiKey, can't. Root never
# signs in over SSH: builders and links sign in as accounts of their own,
# and a person signs in as themselves before becoming root.
{
  services.openssh.settings = {
    ClientAliveInterval = 15;
    ClientAliveCountMax = 4;
    PasswordAuthentication = false;
    KbdInteractiveAuthentication = false;
    PermitRootLogin = "no";
  };
}
