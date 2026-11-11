# shellcheck shell=bash
# The stand-in hinge of lid-hinge.nix, made only once lid-hinge has looked
# for one, as the sensor hub's devices come at a real boot; then its pipe
# held open for the test's samples.
until journalctl -b -u lid-hinge -o cat | grep -qF 'following'; do
  sleep 0.2
done
hinge=/run/stand-in-iio/iio:device0
mkdir -p "$hinge/buffer" "$hinge/scan_elements"
echo 0.017453293 > "$hinge/in_angl_scale"
echo 1 > "$hinge/buffer/enable"
echo 1 > "$hinge/scan_elements/in_angl0_en"
echo 0 > "$hinge/scan_elements/in_angl0_index"
echo 'le:s16/32>>0' > "$hinge/scan_elements/in_angl0_type"
echo hinge > "$hinge/name"
exec sleep infinity > /run/stand-in-iio-nodes/iio:device0
