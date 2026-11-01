# A Mac's sshd asks a quiet caller every 15 s whether it's still there, and
# lets it go after four go unanswered. A caller that went to sleep, or lost
# its network, otherwise holds its session for the two hours TCP takes to
# notice, and what it started runs on beside it. Its own ssh asks the same
# from its side. Whether sshd runs at all stays macOS's Remote Login.
{
  services.openssh.extraConfig = ''
    ClientAliveInterval 15
    ClientAliveCountMax 4
  '';
}
