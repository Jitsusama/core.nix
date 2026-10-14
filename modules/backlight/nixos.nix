# The light sensor, read by iio-sensor-proxy for whoever claims it over
# D-Bus, which on a machine with one is the account's backlight daemon
# (home.nix). The proxy reads a sensor that has a buffer as the kernel fills
# it, so nothing asks the sensor every so often; udev starts it only on a
# machine with a sensor.
{
  hardware.sensor.iio.enable = true;
}
