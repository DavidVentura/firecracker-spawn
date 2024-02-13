# Firecracker-spawn

Create and run a VM without root, through the firecracker vmm crate.

If you plan to use networking, you need to have a TAP device already set up (you can use the [tun](https://github.com/meh/rust-tun) crate or the `ip` command for that).

## Examples

###  Networking
```rust
let v = Vm {
	vcpu_count: 1,
	mem_size_mib: 32,
	kernel_cmdline: "panic=-1 reboot=t init=/goinit".to_string(),
	kernel_path: PathBuf::from("/home/david/git/lk/vmlinux-mini-net"),
	rootfs_path: PathBuf::from("/home/david/git/lk/rootfs.ext4"),
	rootfs_readonly: false,
	extra_disks: vec![],
	net_config: Some(NetConfig {
		tap_iface_name: "mytap0".to_string(),
		vm_mac: None,
	}),
};
v.make().unwrap();
```

###  Multiple disks
```rust
let v = Vm {
	vcpu_count: 1,
	mem_size_mib: 32,
	kernel_cmdline: "panic=-1 reboot=t init=/goinit".to_string(),
	kernel_path: PathBuf::from("/home/david/git/lk/vmlinux-mini-net"),
	rootfs_path: PathBuf::from("/home/david/git/lk/rootfs.ext4"),
	rootfs_readonly: false,
	extra_disks: vec![PathBuf::from("/home/david/git/lk/disk.tar.gz")],
	net_config: None,
};
v.make().unwrap();
```
