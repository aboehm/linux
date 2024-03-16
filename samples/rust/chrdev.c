// SPDX-License-Identifier: GPL-2.0-only
/*
 * 	Character device sample device
 *	Copyright (C) Alexander Böhm <alexander.boehm@malbolge.net> (2024).
 *
 */

#include <linux/module.h>
#include <linux/kernel.h>
#include <linux/fs.h>
#include <linux/miscdevice.h>
#include <linux/string.h>
#include <linux/errno.h>
#include <linux/init.h>

typedef struct  {
    // Current cursor position
    char* head;
    // Limit of the cursor positon
    char* end;
} message_buffer_t;

// Returned datae of the device
const char READ_DATA[] = "Hello CLT 2024\n";

static int chrdev_fops_open(
    struct inode *i,
    struct file *f
) {
	printk (KERN_INFO "chrdev: Open character device\n");
    // Initialize the context
    message_buffer_t* buf = (message_buffer_t*) kmalloc(sizeof(message_buffer_t), GFP_KERNEL);
    buf->head = (char*) READ_DATA;
    buf->end = buf->head + sizeof(READ_DATA);
    // Place the context in the file context
    f->private_data = buf;
    return 0;
}

static ssize_t chrdev_fops_read (
        struct file *f,
        char __user *buffer,
		size_t count,
        loff_t *ppos
) {
	printk (KERN_INFO "chrdev: Read from character device\n");
    // Get the file context
    message_buffer_t* buf = (message_buffer_t*) f->private_data;
    // Determine the available bytes to give back
    size_t len = count;
    if (buf->head + len > buf->end) {
        len = buf->end - buf->head;
    }
    // Copy into user space
    int res = copy_to_user(buffer, buf->head,  len);
    if (res != 0) {
        // Increament the head of the file descriptor
        return -EFAULT;
    } else {
        buf->head += len;
        return len;
    }
}

static int chrdev_fops_release(
    struct inode *i,
    struct file *f
) {
    if (f->private_data != NULL) {
        // Free the allocated file context
        kfree(f->private_data);
        f->private_data = NULL;
    }
    return 0;
}

static const struct file_operations chrdev_fops = {
	.owner		= THIS_MODULE,
    .open       = chrdev_fops_open,
	.read		= chrdev_fops_read,
    .release    = chrdev_fops_release,
	.llseek		= noop_llseek,
};

static struct miscdevice chrdev_device = {
	MISC_DYNAMIC_MINOR,
	"chrdev",
	&chrdev_fops,
};

static int __init chrdev_init(void)
{
	printk (KERN_INFO "chrdev: Native character device sample driver init\n");
	if (misc_register (&chrdev_device)) {
		printk (KERN_WARNING "Couldn't register device\n");
		return -EBUSY;
	}
	return 0;
}

static void __exit chrdev_exit (void) 
{
	printk (KERN_INFO "Native character device sample driver exit\n");
	misc_deregister (&chrdev_device);
}

MODULE_AUTHOR("Alexander Böhm");
MODULE_LICENSE("GPL");

module_init(chrdev_init);
module_exit(chrdev_exit);
