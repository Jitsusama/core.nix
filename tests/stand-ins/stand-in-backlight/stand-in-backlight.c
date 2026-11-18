// SPDX-License-Identifier: GPL-2.0
/*
 * A backlight for a virtual machine, which has none: a brightness from 0 to
 * 100 that drives nothing. Writing it makes the kernel tell udev, as it does
 * for a laptop's panel, which is what the desktop test needs to see.
 *
 * And a keyboard light beside it, as a laptop's keyboard has: two steps
 * above off, named as the kernel names one, so the launcher finds it and
 * logind lets the account set it, as it does a laptop's.
 */
#include <linux/backlight.h>
#include <linux/err.h>
#include <linux/leds.h>
#include <linux/module.h>

static struct backlight_device *backlight;

static int stand_in_update_status(struct backlight_device *device)
{
	return 0;
}

static const struct backlight_ops stand_in_ops = {
	.update_status = stand_in_update_status,
};

/* The light drives nothing either; the kernel keeps the level it's set to. */
static int stand_in_light(struct led_classdev *light,
			  enum led_brightness level)
{
	return 0;
}

static struct led_classdev keyboard = {
	.name = "stand-in::kbd_backlight",
	.brightness = 1,
	.max_brightness = 2,
	.brightness_set_blocking = stand_in_light,
};

static int __init stand_in_init(void)
{
	const struct backlight_properties properties = {
		.type = BACKLIGHT_RAW,
		.brightness = 50,
		.max_brightness = 100,
	};
	int error;

	backlight = backlight_device_register("stand-in", NULL, NULL,
					      &stand_in_ops, &properties);
	if (IS_ERR(backlight))
		return PTR_ERR(backlight);
	error = led_classdev_register(NULL, &keyboard);
	if (error)
		backlight_device_unregister(backlight);
	return error;
}

static void __exit stand_in_exit(void)
{
	led_classdev_unregister(&keyboard);
	backlight_device_unregister(backlight);
}

module_init(stand_in_init);
module_exit(stand_in_exit);
MODULE_DESCRIPTION("A backlight and a keyboard light that drive nothing, for tests");
MODULE_LICENSE("GPL");
