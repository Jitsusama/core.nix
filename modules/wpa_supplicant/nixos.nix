# wpa_supplicant does the radio's half of Wi-Fi: scanning, joining with WPA2,
# WPA3 or an enterprise login, and moving between access points. It holds no
# networks of its own. wifid adds each Wi-Fi card over D-Bus, as it appears,
# with the settings below, and hands it the network to join.
{
  networking.wireless = {
    enable = true;
    dbusControlled = true;
    # wifid adds every card itself, so wpa_supplicant starts with none and no
    # machine names its interface. NixOS restarts wpa_supplicant whenever a
    # card comes or goes, a USB one included, and wifid adds them all again.
    autoDetectInterfaces = false;
    # Makes the directory wpa_cli answers from, so it works at a terminal.
    userControlled = true;
  };

  # What every card wifid adds starts from. wpa_cli reaches a card through
  # its control socket, for root and the wpa_supplicant group, since it can
  # change any network. Nothing is saved back: wifid owns the networks.
  environment.etc."wpa_supplicant/interface.conf".text = ''
    ctrl_interface=DIR=/run/wpa_supplicant/control GROUP=wpa_supplicant
    update_config=0
    # A card scans and asks for network details from a random address, and
    # joins each network from the one wifid gives it.
    preassoc_mac_addr=1
    gas_rand_mac_addr=1
    # No Wi-Fi Direct: it would add a second interface beside every card.
    p2p_disabled=1
    # Looks for a better access point of the same network when the signal
    # drops below -70 dBm, every 30 seconds, and hourly otherwise.
    bgscan="simple:30:-70:3600"
    pmf=1
  '';
}
