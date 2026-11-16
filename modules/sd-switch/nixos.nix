# sd-switch, which home-manager switches an account's units with, patched to
# switch however many change at once.
#
# It asks systemd to be told as each job ends but read none of it until it
# had sent every job. zbus keeps 64 unread messages of a kind and then stops
# reading the bus, so in a switch of more than 64 jobs the next job's reply
# never came: on 16 November two switches of about 75 hung for
# home-manager's 5 minutes and the machine went back to what it ran. It now
# reads them on a thread of its own from the start, and its wait for the last
# job ends at its timeout saying how many were left
# (sd-switch-reads-as-it-sends.patch). home-manager's useGlobalPkgs gives
# each account's activation the system's patched one.
{
  nixpkgs.overlays = [
    (_: prev: {
      sd-switch = prev.sd-switch.overrideAttrs (old: {
        patches = (old.patches or [ ]) ++ [ ./sd-switch-reads-as-it-sends.patch ];
      });
    })
  ];
}
