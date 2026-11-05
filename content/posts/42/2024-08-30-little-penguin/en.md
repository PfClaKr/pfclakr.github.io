+++
title = "little-penguin-1: ten small steps into the Linux kernel"
description = "Building and booting your own kernel, writing modules, loading one on USB keyboard hot-plug, a misc character device, debugfs with a mutex, and walking the mount tree from procfs."
tags = ["42", "linux", "kernel", "c"]
+++

**little-penguin-1** is a series of ten short assignments, inspired by the old Eudyptula Challenge,
that walk you from "I've never compiled a kernel" to writing small drivers. Each one is small. Together
they cover the whole loop of kernel work: build, boot, write a module, load it, and debug it
([source](https://github.com/PfClaKr/Ecole42Paris_LittlePenguin)).

| # | Assignment |
|---|---|
| 00 | build Linus's latest tree with your own `CONFIG_LOCALVERSION` and boot it |
| 01 | a "Hello world" module |
| 02 | patch the top-level `Makefile`'s `EXTRAVERSION`, rebuild, submit the diff as a patch |
| 03 | fix a file so `checkpatch.pl --strict` is silent |
| 04 | load a module automatically when a USB keyboard is plugged in |
| 05 | a misc character device, `/dev/fortytwo` |
| 06 | build and boot `linux-next` |
| 07 | a debugfs directory with three files |
| 08 | clean up a deliberately ugly module and work out what it does |
| 09 | a `/proc/mymounts` file listing mount points |

## Building and booting

Assignment 00 is mostly patience: clone the tree, `make menuconfig`, set `CONFIG_LOCALVERSION`, build,
`make modules_install install`, reboot, and check `uname -r`. The practical lesson is disk space.
Every kernel build, plus its installed modules under `/lib/modules`, takes gigabytes. My notes say to
give the VM at least 50 GB, and even that filled up by assignment 02, until I deleted the modules of
old kernels.

## A module is two functions

```c
static int __init hello_init(void)
{
    printk(KERN_INFO "Hello world!\n");
    return 0;
}

static void __exit hello_exit(void)
{
    printk(KERN_INFO "Cleaning up module.\n");
}

module_init(hello_init);
module_exit(hello_exit);
```

`printk` doesn't write to a terminal. It writes to the kernel log ring buffer,
read with `dmesg`. `KERN_INFO` isn't a function argument at all: it's a string literal (`"\001" "6"`)
that the compiler pastes in front of the format string, and the log level is parsed from that prefix.

## Hot-plug with udev

Assignment 04 wants the module loaded when a USB keyboard is plugged in. The kernel announces the new
device, and **udev**, in userspace, matches it against rules. A USB keyboard is interface class 3
(HID), protocol 1 (keyboard):

```text
ACTION=="add", SUBSYSTEM=="usb", ATTR{bInterfaceClass}=="03", ATTR{bInterfaceProtocol}=="01", \
    RUN+="/sbin/insmod /root/main.ko"
```

The "proper" kernel way is a `MODULE_DEVICE_TABLE(usb, …)` so that `modprobe` loads the module from
the device's modalias, but the udev rule makes the mechanism visible.

## A misc character device

`/dev/fortytwo` must return my login on `read`, and accept a `write` only if it's exactly my login,
otherwise fail with `EINVAL`. A misc device gets a dynamic minor number under the shared misc major,
so registration is one call. The work is in the `file_operations`:

```c
static ssize_t ft_read(struct file *f, char __user *buf, size_t count, loff_t *ppos)
{
    return simple_read_from_buffer(buf, count, ppos, LOGIN, strlen(LOGIN));
}

static ssize_t ft_write(struct file *f, const char __user *buf, size_t count, loff_t *ppos)
{
    char kbuf[32];
    ssize_t len = simple_write_to_buffer(kbuf, sizeof(kbuf) - 1, ppos, buf, count);

    if (len < 0)
        return len;
    kbuf[len] = '\0';
    return strcmp(kbuf, LOGIN) == 0 ? count : -EINVAL;
}
```

The kernel can never dereference a userspace pointer directly (`__user` marks them for static
checking). The `simple_*_buffer` helpers wrap `copy_to_user`/`copy_from_user` and handle the file
offset for you.

That offset caught me while testing. `read` and `write` on the **same** file descriptor share `*ppos`.
After writing 5 bytes, a `read` starts at offset 5, past the end of the login, and returns nothing.
Closing and reopening, or `lseek(fd, 0, SEEK_SET)`, resets it. Every regular file behaves this way, but
it's easy to forget when you're the one implementing the file.

## debugfs and a mutex

Assignment 07 creates `/sys/kernel/debug/fortytwo/` with three files: `id` (the same behavior as
`/dev/fortytwo`), `jiffies` (read-only, the kernel tick counter, exposed as a `u32` or `u64` depending on the
architecture's word size), and `foo` (one page of storage that
root can write and anyone can read). Since two processes may write `foo` at once, access goes through a
mutex:

```c
static char foo_page[PAGE_SIZE];
static size_t foo_page_len;
static DEFINE_MUTEX(foo_mutex);

static ssize_t foo_write(struct file *f, const char __user *buf, size_t count, loff_t *ppos)
{
    ssize_t ret;

    if (count > PAGE_SIZE)
        return -EINVAL;
    mutex_lock(&foo_mutex);
    ret = simple_write_to_buffer(foo_page, PAGE_SIZE, ppos, buf, count);
    foo_page_len = *ppos;
    mutex_unlock(&foo_mutex);
    return ret;
}
```

A mutex is fine here because we're in process context and allowed to sleep. In an interrupt handler,
it would have to be a spinlock.

## Walking the mount tree

Assignment 09 lists every mount point in `/proc/mymounts`, like a simplified `/proc/mounts`. procfs
files are generated on read, so the module uses the `seq_file` interface: `single_open` plus a `show`
callback that prints into a kernel-managed buffer.

Finding the mounts is where kernel versions matter. Older tutorials walk `mnt_ns->list`, a linked list.
In recent kernels the mounts of a namespace live in a **red-black tree**, `mnt_ns->mounts`, so the
module walks it with `rb_entry`:

```c
static void iter_mount_tree(struct seq_file *m, struct rb_node *node)
{
    if (!node)
        return;
    show_mount(m, rb_entry(node, struct mount, mnt_node));
    iter_mount_tree(m, node->rb_left);
    iter_mount_tree(m, node->rb_right);
}
```

Two honest caveats: `struct mount` is private (it comes from `fs/mount.h`, not a public header), so the
module only builds against a full kernel source tree. And the walk takes no lock, so a mount or unmount
happening at the same time could change the tree under it. The real `/proc/mounts` holds the namespace
lock while iterating.
