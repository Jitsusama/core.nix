// SPDX-License-Identifier: GPL-2.0
/*
 * A lid for a virtual machine, which has none: a switch that reports the
 * lid shut or open as a laptop's does, so logind says so. Writing 1 to
 * /sys/module/stand_in_lid/parameters/shut shuts it, and 0 opens it.
 */
#include <linux/input.h>
#include <linux/module.h>

static struct input_dev *lid;
static bool shut;

static void stand_in_report(void)
{
	input_report_switch(lid, SW_LID, shut);
	input_sync(lid);
}

static int stand_in_set_shut(const char *value, const struct kernel_param *param)
{
	int error = param_set_bool(value, param);

	if (error)
		return error;
	/* Set before the switch exists, it's reported once it does. */
	if (lid)
		stand_in_report();
	return 0;
}

static const struct kernel_param_ops stand_in_shut_ops = {
	.set = stand_in_set_shut,
	.get = param_get_bool,
};
module_param_cb(shut, &stand_in_shut_ops, &shut, 0644);
MODULE_PARM_DESC(shut, "Whether the lid is shut");

static int __init stand_in_init(void)
{
	int error;

	lid = input_allocate_device();
	if (!lid)
		return -ENOMEM;
	lid->name = "Stand-in Lid Switch";
	lid->phys = "stand-in/lid";
	lid->id.bustype = BUS_VIRTUAL;
	input_set_capability(lid, EV_SW, SW_LID);
	error = input_register_device(lid);
	if (error) {
		input_free_device(lid);
		lid = NULL;
		return error;
	}
	stand_in_report();
	return 0;
}

static void __exit stand_in_exit(void)
{
	input_unregister_device(lid);
}

module_init(stand_in_init);
module_exit(stand_in_exit);
MODULE_DESCRIPTION("A lid switch that opens and shuts by its parameter, for tests");
MODULE_LICENSE("GPL");
