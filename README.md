
Create and run a VM without root. Requires a TAP device to be set up.
```rust
use crate::Vm;
use std::path::PathBuf;
fn main() {
	let v = Vm {
		vcpu_count: 1,
		mem_size_mib: 32,
		kernel_cmdline: "panic=-1 reboot=t root=/dev/vda init=/goinit".to_string(),
		kernel_path: PathBuf::from("/home/david/git/lk/vmlinux-mini-net"),
		rootfs_path: PathBuf::from("/home/david/git/lk/rootfs.ext4"),
		rootfs_readonly: false,
		tap_iface_name: "mytap0".to_string(),
		vm_mac: None,
	};
	v.make().unwrap();
}
```
