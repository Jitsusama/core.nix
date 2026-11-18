// SPDX-License-Identifier: GPL-2.0
/*
 * A monitor for a virtual machine, which has none that takes DDC/CI: an I2C
 * bus named as an MST hub's are, with a monitor at DDC/CI's address that
 * answers for its brightness and takes a new one, as a desk's monitor does.
 * Its brightness reads at /sys/module/stand_in_monitor/parameters/brightness,
 * out of 100, and starts at 40.
 */
#include <linux/i2c.h>
#include <linux/module.h>
#include <linux/mutex.h>
#include <linux/string.h>

#define DDC_CI 0x37
#define BRIGHTNESS 0x10
#define BRIGHTEST 100

static ushort brightness = 40;
module_param(brightness, ushort, 0444);
MODULE_PARM_DESC(brightness, "The monitor's brightness, out of 100");

static DEFINE_MUTEX(answering);
/* What the monitor will say when next read, made as it's asked. */
static u8 answer[11];
static bool answered;

static u8 summed(u8 from, const u8 *bytes, int length)
{
	while (length--)
		from ^= *bytes++;
	return from;
}

/* A host's message: 0x51, 0x80 and its length, what it says, a checksum. */
static int stand_in_heard(const u8 *message, int length)
{
	int said;

	if (length < 3 || message[0] != 0x51 || !(message[1] & 0x80))
		return -EIO;
	said = message[1] & 0x7f;
	if (length != said + 3 || summed(0x6e, message, length - 1) != message[length - 1])
		return -EIO;
	message += 2;
	if (said == 2 && message[0] == 0x01) {
		bool known = message[1] == BRIGHTNESS;

		answer[0] = 0x6e;
		answer[1] = 0x88;
		answer[2] = 0x02;
		answer[3] = known ? 0 : 1;
		answer[4] = message[1];
		answer[5] = 0;
		answer[6] = 0;
		answer[7] = known ? BRIGHTEST : 0;
		answer[8] = known ? brightness >> 8 : 0;
		answer[9] = known ? brightness & 0xff : 0;
		answer[10] = summed(0x50, answer, 10);
		answered = true;
		return 0;
	}
	if (said == 4 && message[0] == 0x03 && message[1] == BRIGHTNESS) {
		brightness = min_t(ushort, (message[2] << 8) | message[3], BRIGHTEST);
		return 0;
	}
	return -EIO;
}

static int stand_in_xfer(struct i2c_adapter *bus, struct i2c_msg *messages, int count)
{
	int i, error = 0;

	mutex_lock(&answering);
	for (i = 0; i < count && !error; i++) {
		struct i2c_msg *message = &messages[i];

		if (message->addr != DDC_CI) {
			error = -ENXIO;
		} else if (message->flags & I2C_M_RD) {
			/* Read before it was asked, as a monitor would, it says nothing. */
			if (!answered || message->len > sizeof(answer)) {
				error = -EIO;
			} else {
				memcpy(message->buf, answer, message->len);
				answered = false;
			}
		} else {
			error = stand_in_heard(message->buf, message->len);
		}
	}
	mutex_unlock(&answering);
	return error ? error : count;
}

static u32 stand_in_functionality(struct i2c_adapter *bus)
{
	return I2C_FUNC_I2C;
}

static const struct i2c_algorithm stand_in_algorithm = {
	.xfer = stand_in_xfer,
	.functionality = stand_in_functionality,
};

static struct i2c_adapter bus = {
	.owner = THIS_MODULE,
	.algo = &stand_in_algorithm,
	.name = "DPMST",
};

static int __init stand_in_init(void)
{
	return i2c_add_adapter(&bus);
}

static void __exit stand_in_exit(void)
{
	i2c_del_adapter(&bus);
}

module_init(stand_in_init);
module_exit(stand_in_exit);
MODULE_DESCRIPTION("A monitor that answers DDC/CI for its brightness, for tests");
MODULE_LICENSE("GPL");
