# Keeps the clock set on any network. A network can name its own time server,
# and timesyncd then asks only that one, turning to the pool of public servers
# only when nothing names any server at all; a hotel's that never answered
# left the clock unset all night. Naming the pool as the machine's own servers
# has it ask the pool first and the network's server after. A network that
# blocks the pool still sets the clock from its own server, a few minutes
# later, once timesyncd has given up on each of the pool's addresses.
{ config, ... }:
{
  services.timesyncd.servers = config.networking.timeServers;
}
