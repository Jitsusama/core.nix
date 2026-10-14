// SPDX-License-Identifier: GPL-2.0
/*
 * An ambient light sensor for a virtual machine, which has none, shaped like
 * a laptop's sensor hub: an IIO device named "als" whose illuminance is read
 * once from sysfs and then heard through its buffer as it changes. Writing
 * in_illuminance_raw is the light changing: the reading is kept, and pushed
 * to the buffer while it's on, which is what the desktop test needs.
 */
#include <linux/iio/buffer.h>
#include <linux/iio/iio.h>
#include <linux/iio/kfifo_buf.h>
#include <linux/module.h>
#include <linux/mutex.h>
#include <linux/platform_device.h>

struct stand_in {
	struct mutex lock;
	s32 raw;
};

static const struct iio_chan_spec stand_in_channels[] = {
	{
		.type = IIO_LIGHT,
		.info_mask_separate = BIT(IIO_CHAN_INFO_RAW) |
				      BIT(IIO_CHAN_INFO_SCALE),
		.scan_index = 0,
		.scan_type = {
			.sign = 's',
			.realbits = 32,
			.storagebits = 32,
			.endianness = IIO_LE,
		},
	},
	IIO_CHAN_SOFT_TIMESTAMP(1),
};

static int stand_in_read_raw(struct iio_dev *indio_dev,
			     struct iio_chan_spec const *channel, int *val,
			     int *val2, long mask)
{
	struct stand_in *light = iio_priv(indio_dev);

	switch (mask) {
	case IIO_CHAN_INFO_RAW:
		mutex_lock(&light->lock);
		*val = light->raw;
		mutex_unlock(&light->lock);
		return IIO_VAL_INT;
	case IIO_CHAN_INFO_SCALE:
		/* A thousandth of a lux a step, as the sensor hub's. */
		*val = 0;
		*val2 = 1000;
		return IIO_VAL_INT_PLUS_MICRO;
	default:
		return -EINVAL;
	}
}

static int stand_in_write_raw(struct iio_dev *indio_dev,
			      struct iio_chan_spec const *channel, int val,
			      int val2, long mask)
{
	struct stand_in *light = iio_priv(indio_dev);
	struct {
		s32 raw;
		aligned_s64 timestamp;
	} scan = { .raw = val };

	if (mask != IIO_CHAN_INFO_RAW)
		return -EINVAL;

	mutex_lock(&light->lock);
	light->raw = val;
	if (iio_buffer_enabled(indio_dev))
		iio_push_to_buffers_with_ts(indio_dev, &scan, sizeof(scan),
					    iio_get_time_ns(indio_dev));
	mutex_unlock(&light->lock);
	return 0;
}

static const struct iio_info stand_in_info = {
	.read_raw = stand_in_read_raw,
	.write_raw = stand_in_write_raw,
};

static int stand_in_probe(struct platform_device *device)
{
	struct iio_dev *indio_dev;
	struct stand_in *light;
	int ret;

	indio_dev = devm_iio_device_alloc(&device->dev, sizeof(*light));
	if (!indio_dev)
		return -ENOMEM;
	light = iio_priv(indio_dev);
	mutex_init(&light->lock);
	light->raw = 0;

	indio_dev->name = "als";
	indio_dev->info = &stand_in_info;
	indio_dev->channels = stand_in_channels;
	indio_dev->num_channels = ARRAY_SIZE(stand_in_channels);
	indio_dev->modes = INDIO_DIRECT_MODE;

	ret = devm_iio_kfifo_buffer_setup(&device->dev, indio_dev, NULL);
	if (ret)
		return ret;
	return devm_iio_device_register(&device->dev, indio_dev);
}

static struct platform_driver stand_in_driver = {
	.driver = { .name = "stand-in-light" },
	.probe = stand_in_probe,
};

static struct platform_device *stand_in_device;

static int __init stand_in_init(void)
{
	int ret = platform_driver_register(&stand_in_driver);

	if (ret)
		return ret;
	stand_in_device = platform_device_register_simple("stand-in-light",
							  PLATFORM_DEVID_NONE,
							  NULL, 0);
	if (IS_ERR(stand_in_device)) {
		platform_driver_unregister(&stand_in_driver);
		return PTR_ERR(stand_in_device);
	}
	return 0;
}

static void __exit stand_in_exit(void)
{
	platform_device_unregister(stand_in_device);
	platform_driver_unregister(&stand_in_driver);
}

module_init(stand_in_init);
module_exit(stand_in_exit);
MODULE_DESCRIPTION("An ambient light sensor that senses nothing, for tests");
MODULE_LICENSE("GPL");
