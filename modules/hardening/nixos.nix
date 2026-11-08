# The kernel's doors that nothing on a machine walks through, shut, so code
# that gets in as an account finds fewer ways further in. Each costs nothing:
# no program core.nix runs needs what's shut, and none is slower for it.
#
# What's left open on purpose: kptr_restrict stays at 1, since 2 hides the
# kernel's addresses from root's perf too, and the kernel's AutoFDO profile
# is recorded with them (docs/kernel.md). ptrace stays at Yama's 1, which a
# debugger attaching to its own child needs. io_uring stays on until it's
# known what on a machine uses it. And kernel lockdown isn't built, since
# mainline turns hibernation off under it.
_: {
  boot.kernel.sysctl = {
    # Nothing here replaces the running kernel without a reboot, and a root
    # that could would run an unsigned kernel past Secure Boot, which signs
    # only what the firmware starts. Once set, it stays set until a reboot.
    "kernel.kexec_load_disabled" = 1;

    # An account opening a terminal with an unusual line discipline had the
    # kernel load that discipline's module for it, and those rarely used
    # modules are where the kernel's tty bugs were found. Root still loads
    # one by name.
    "dev.tty.ldisc_autoload" = 0;

    # Opening a file or FIFO someone else owns in a shared sticky folder,
    # such as /tmp, as though creating it, is how one account tricks
    # another's program into writing where it didn't mean to. 2 refuses it
    # in group-writable sticky folders as well as world-writable ones.
    "fs.protected_regular" = 2;
    "fs.protected_fifos" = 2;

    # A laptop isn't a router, and on a network it doesn't trust an ICMP
    # redirect is someone telling it to send its traffic their way. The
    # kernel already ignores them on links it has; these cover the links it
    # makes later (a dock's cable, a VPN) and IPv6, and stop it sending them.
    "net.ipv4.conf.default.accept_redirects" = 0;
    "net.ipv4.conf.all.secure_redirects" = 0;
    "net.ipv4.conf.default.secure_redirects" = 0;
    "net.ipv4.conf.all.send_redirects" = 0;
    "net.ipv4.conf.default.send_redirects" = 0;
    "net.ipv6.conf.all.accept_redirects" = 0;
    "net.ipv6.conf.default.accept_redirects" = 0;
  };
}
