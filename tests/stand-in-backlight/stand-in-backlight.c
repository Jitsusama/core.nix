// SPDX-License-Identifier: GPL-2.0
/*
 * A backlight for a virtual machine, which has none: a brightness from 0 to
 * 100 that drives nothing. Writing it makes the kernel tell udev, as it does
 * for a laptop's panel, which is what the desktop test needs to see.
 */
#include <linux/backlight.h>
#include <linux/err.h>
#include <linux/module.h>

static struct backlight_device *backlight;

static int stand_in_update_status(struct backlight_device *device)
{
	return 0;
}

static const struct backlight_ops stand_in_ops = {
	.update_status = stand_in_update_status,
};

static int __init stand_in_init(void)
{
	const struct backlight_properties properties = {
		.type = BACKLIGHT_RAW,
		.brightness = 50,
		.max_brightness = 100,
	};

	backlight = backlight_device_register("stand-in", NULL, NULL,
					      &stand_in_ops, &properties);
	return PTR_ERR_OR_ZERO(backlight);
}

static void __exit stand_in_exit(void)
{
	backlight_device_unregister(backlight);
}

module_init(stand_in_init);
module_exit(stand_in_exit);
MODULE_DESCRIPTION("A backlight that drives nothing, for tests");
MODULE_LICENSE("GPL");
