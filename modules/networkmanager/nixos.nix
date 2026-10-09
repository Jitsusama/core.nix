# NetworkManager joins the networks a laptop meets, Wi-Fi and wired alike,
# and remembers them. Joel picks a network with nmtui in a terminal; the
# signed-in account may change networks without a password, as polkit's
# defaults for NetworkManager allow.
{
  networking.networkmanager.enable = true;
  # The kernel's Wi-Fi stack reads each country's channel and power limits
  # from here; without it, it falls back to the most restrictive set.
  hardware.wirelessRegulatoryDatabase = true;
}
