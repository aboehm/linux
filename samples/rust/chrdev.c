// SPDX-License-Identifier: GPL-2.0-only
/*
 * 	Character device sample device
 *	Copyright (C) Alexander Böhm <alexander.boehm@malbolge.net> (2024).
 *
 */

#include <linux/module.h>
#include <linux/kernel.h>
#include <linux/sched/signal.h>
#include <linux/interrupt.h>
#include <linux/fs.h>
#include <linux/miscdevice.h>
#include <linux/string.h>
#include <linux/errno.h>
#include <linux/init.h>

#include <linux/uaccess.h>

static const char RETURN_MESSAGE[] = "Hello from chrdev\n";
static const int RETURN_MESSAGE_LEN = 18;

static int chrdev_open(
    struct inode *i,
    struct file *f
) {
	printk (KERN_INFO "chrdev: Open character device");
    return 0;
}

static ssize_t chrdev_read (
        struct file *filp,
        char __user *buffer,
		size_t count,
        loff_t *ppos
) {
	printk (KERN_INFO "chrdev: Read from character device");
    return copy_to_user(buffer, &RETURN_MESSAGE, RETURN_MESSAGE_LEN)
        ? -EFAULT
        : RETURN_MESSAGE_LEN;
}

static const struct file_operations chrdev_fops = {
	.owner		= THIS_MODULE,
    .open       = chrdev_open,
	.read		= chrdev_read,
	.llseek		= noop_llseek,
};

static struct miscdevice chrdev_device = {
	MISC_DYNAMIC_MINOR,
	"chrdev",
	NULL, // &chrdev_fops,
};

static int __init chrdev_init(void)
{
    long ptr = (long) chrdev_fops.open;
	printk (KERN_INFO "*chrdev_open = %x", ptr);
    long ptr_this_module = (long) THIS_MODULE;
	printk (KERN_INFO "*this module = %x", ptr_this_module);
	printk (KERN_INFO "Native character device sample driver init");
	if (misc_register (&chrdev_device)) {
		printk (KERN_WARNING "chrdev: Couldn't register device");
		return -EBUSY;
	}
	return 0;
}

static void __exit chrdev_exit (void) 
{
	printk (KERN_INFO "Natvice character device sample driver exit");
	misc_deregister (&chrdev_device);
}


MODULE_AUTHOR("Alexander Böhm");
MODULE_LICENSE("GPL");

module_init(chrdev_init);
module_exit(chrdev_exit);
