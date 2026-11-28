# How a node is run as a service: a systemd user unit on Linux and a
# launchd agent on a Mac, each given its words (home.nix's `node` and its
# further realms'). It changes only when how the service manager starts,
# stops and limits a node does, apart from what the node is told to do.
{
  config,
  lib,
  servesIn,
}:
{
  linuxNode = description: words: key: realm: {
    Unit = {
      Description = description;
      Wants = lib.optional (key != null) "${key}.socket";
      After = lib.optional (key != null) "${key}.socket";
      # The node reads what it serves only as it starts, so a switch that
      # changes it restarts the node, which the sessions outlive: on
      # 2026-11-06 a switch added the runs and the node served none.
      X-Restart-Triggers = [ (builtins.hashString "sha256" (builtins.toJSON (servesIn realm))) ];
    };
    Service = {
      ExecStart = lib.escapeShellArgs words;
      Environment = lib.optional (key != null) "SSH_AUTH_SOCK=%t/${key}.sock";
      # The node lets go of the sessions it carried when told to stop,
      # and redials a dropped link itself. It is started again however it
      # ends, unless systemd stopped it: a terminate signal from anything
      # else ends it cleanly too, and on 2026-11-14 one left the work
      # realm's node down for seven minutes.
      Restart = "always";
      RestartSec = "5s";
      # A stop lets the model calls it lends end first, for at most the
      # 60 s its --lends-drain-within gives, while its links carry the
      # answers back. systemd's own control-group kill would end each
      # link's ssh at the same moment, cutting every call it lends and
      # charging the rooms for each retry, so the terminate goes to the
      # node alone, which ends its links itself once the calls are in. The
      # stop timeout is said here because the drain is sized within it.
      KillMode = "mixed";
      TimeoutStopSec = "90s";
      # Each client, link and session it holds is a file, and at systemd's
      # 1024 a failed accept once ended the node (mycelium #372). A bare
      # number would set the hard limit too, so the hard limit is named
      # as systemd's own default, 524288, rather than lowered.
      LimitNOFILE = "65536:524288";
      # It wakes with every session it carries, so it runs among them, not
      # among Joel's programs, whose tasks each wakeup would preempt. What it
      # swaps out stays compressed in memory (checks.nix has the numbers):
      # waking to read it back from the disk held its calls up to 44 s.
      Slice = "agents.slice";
      MemoryZSwapWriteback = false;
    };
    Install.WantedBy = [ "default.target" ];
  };
  macNode = words: log: realm: {
    enable = true;
    config = {
      ProgramArguments = words;
      # The system's own ssh first, which reads the machine's SSH config
      # and the login session's agent. The account's profile follows it,
      # since a carry runs the harness's pi-carry by name, as the Linux
      # node finds it through the session's PATH.
      EnvironmentVariables = {
        PATH = "/usr/bin:/bin:${config.home.profileDirectory}/bin";
        # What the node starts is driven from another machine, where
        # nobody can touch this one's sensor, so its commits sign with
        # the agents' key, even in a held terminal.
        SIGN_AS = "agent";
        # The node reads what it serves only as it starts, and launchd
        # reloads an agent only when its own file changes, so what it
        # serves is written into it, as X-Restart-Triggers is on Linux.
        MYCELIUM_SERVES = builtins.hashString "sha256" (builtins.toJSON (servesIn realm));
      };
      RunAtLoad = true;
      # Started again whenever it stops, but not more than every 30 s.
      KeepAlive = true;
      ThrottleInterval = 30;
      # Each client, link and session it holds is a file, and launchd
      # gives an agent 256. 10240 is macOS's OPEN_MAX, and the kernel
      # quietly lowers a soft limit above its kern.maxfilesperproc to that.
      # No hard limit is named, so launchd's stays, unlimited unless
      # someone lowered it.
      SoftResourceLimits.NumberOfFiles = 10240;
      StandardErrorPath = "${config.home.homeDirectory}/Library/Logs/${log}.log";
    };
  };
}
