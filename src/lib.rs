use std::error::Error;
use std::fs::File;
use std::path::PathBuf;
use utils::net::mac::MacAddr;
use vmm::builder::build_microvm_for_boot;
pub use vmm::devices::legacy::serial::SerialOut;
use vmm::devices::virtio::block::CacheType;
use vmm::resources::VmResources;
use vmm::seccomp_filters::get_empty_filters;
use vmm::vmm_config::boot_source::{BootConfig, BootSource, BootSourceConfig};
use vmm::vmm_config::drive::{BlockBuilder, BlockDeviceConfig};
use vmm::vmm_config::instance_info::{InstanceInfo, VmState};
use vmm::vmm_config::machine_config::HugePageConfig;
use vmm::vmm_config::machine_config::VmConfig;
use vmm::vmm_config::net::{NetBuilder, NetworkInterfaceConfig};
use vmm::{EventManager, FcExitCode};

#[derive(Clone)]
pub struct Disk {
    pub path: PathBuf,
    pub read_only: bool,
}

#[derive(Clone)]
pub struct NetConfig {
    /// Name of an unused TAP interface on the host, must exist
    pub tap_iface_name: String,
    /// Mac address - Leave blank for a default
    pub vm_mac: Option<[u8; 6]>,
}

pub struct Vm {
    pub vcpu_count: u8,
    pub mem_size_mib: usize,
    pub kernel: File,
    pub kernel_cmdline: String,
    pub initrd: Option<File>,
    pub rootfs: Option<Disk>,
    pub extra_disks: Vec<Disk>,
    pub net_config: Option<NetConfig>,
    pub use_hugepages: bool,
}

impl Vm {
    pub fn make(&self, output: Box<dyn SerialOut>) -> Result<(), Box<dyn Error>> {
        let instance_info = InstanceInfo {
            id: "anonymous-instance".to_string(),
            state: VmState::NotStarted,
            vmm_version: "Amazing version".to_string(),
            app_name: "cpu-template-helper".to_string(),
        };

        let vm_config = VmConfig {
            vcpu_count: self.vcpu_count,
            mem_size_mib: self.mem_size_mib,
            smt: false,
            cpu_template: None,
            track_dirty_pages: false,
            huge_pages: if self.use_hugepages {
                HugePageConfig::Hugetlbfs2M
            } else {
                HugePageConfig::None
            },
        };
        let initrd = match &self.initrd {
            None => None,
            Some(f) => Some(f.try_clone()?),
        };
        let boot_source = BootSource {
            config: BootSourceConfig::default(),
            builder: Some(BootConfig {
                cmdline: linux_loader::cmdline::Cmdline::try_from(&self.kernel_cmdline, 4096)?,
                kernel_file: self.kernel.try_clone()?,
                initrd_file: initrd,
            }),
        };

        let mut net_builder = NetBuilder::new();
        match &self.net_config {
            Some(nc) => {
                let mac = nc.vm_mac.unwrap_or([0x0, 0x2, 0x0, 0x0, 0x0, 0x0]);
                net_builder
                    .build(NetworkInterfaceConfig {
                        iface_id: "net0".to_string(),
                        host_dev_name: nc.tap_iface_name.clone(),
                        guest_mac: Some(MacAddr::from_bytes_unchecked(&mac)),
                        rx_rate_limiter: None,
                        tx_rate_limiter: None,
                    })
                    .unwrap();
            }
            None => (),
        };

        let mut block = BlockBuilder::new();

        if let Some(rootfs) = &self.rootfs {
            block
                .insert(BlockDeviceConfig {
                    drive_id: "block0".to_string(),
                    partuuid: None,
                    is_root_device: true,
                    cache_type: CacheType::Unsafe,

                    is_read_only: Some(rootfs.read_only),
                    path_on_host: Some(rootfs.path.as_path().display().to_string()),
                    rate_limiter: None,
                    file_engine_type: None,

                    socket: None,
                })
                .unwrap();
        };

        for (i, disk) in self.extra_disks.iter().enumerate() {
            block
                .insert(BlockDeviceConfig {
                    drive_id: format!("block{}", i + 0),
                    partuuid: None,
                    is_root_device: false,
                    cache_type: CacheType::Unsafe,

                    is_read_only: Some(disk.read_only),
                    path_on_host: Some(disk.path.as_path().display().to_string()),
                    rate_limiter: None,
                    file_engine_type: None,

                    socket: None,
                })
                .unwrap();
        }

        let vm_resources = VmResources {
            vm_config,
            boot_source,
            net_builder,
            block,
            boot_timer: false,
            ..Default::default()
        };

        let mut event_manager = EventManager::new().unwrap();
        let seccomp_filters = get_empty_filters();

        let vm = build_microvm_for_boot(
            &instance_info,
            &vm_resources,
            &mut event_manager,
            &seccomp_filters,
            output,
        )?;
        vm.lock().unwrap().resume_vm()?;
        loop {
            event_manager.run().unwrap();
            match vm.lock().unwrap().shutdown_exit_code() {
                Some(FcExitCode::Ok) => break,
                Some(_) => {
                    println!("vm died??");
                    return Ok(());
                }
                None => continue,
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::{Disk, NetConfig, Vm};
    use std::fs::File;
    use std::io;
    use std::path::PathBuf;
    #[test]
    fn it_works_net() {
        let kernel = File::open("/home/david/git/lk/vmlinux-mini-net").unwrap();
        let v = Vm {
            vcpu_count: 1,
            mem_size_mib: 32,
            kernel,
            kernel_cmdline: "quiet panic=-1 reboot=t init=/goinit".to_string(),
            rootfs: Some(Disk {
                path: PathBuf::from("/home/david/git/lk/rootfs.ext4"),
                read_only: false,
            }),
            initrd: None,
            extra_disks: vec![],
            net_config: Some(NetConfig {
                tap_iface_name: "mytap0".to_string(),
                vm_mac: None,
            }),
            use_hugepages: false,
        };
        v.make(Box::new(io::sink())).unwrap();
    }

    #[test]
    fn it_works_disk() {
        let kernel = File::open("/home/david/git/lk/vmlinux-mini-net").unwrap();
        let v = Vm {
            vcpu_count: 1,
            mem_size_mib: 32,
            kernel,
            kernel_cmdline: "quiet panic=-1 reboot=t init=/goinit".to_string(),
            rootfs: Some(Disk {
                path: PathBuf::from("/home/david/git/lk/rootfs.ext4"),
                read_only: false,
            }),
            initrd: None,
            extra_disks: vec![Disk {
                path: PathBuf::from("/home/david/git/lk/disk.tar.gz"),
                read_only: true,
            }],
            net_config: None,
            use_hugepages: false,
        };
        v.make(Box::new(io::sink())).unwrap();
    }

    #[test]
    fn it_works_initrd() {
        let kernel = File::open("vmlinux").unwrap();
        let v = Vm {
            vcpu_count: 1,
            mem_size_mib: 32,
            kernel,
            kernel_cmdline: "panic=-1 reboot=t init=/init".to_string(),
            rootfs: None,
            initrd: Some(File::open("bootstrap-initrd.cpio.gz").unwrap()),
            extra_disks: vec![],
            net_config: None,
            use_hugepages: false,
        };
        v.make(Box::new(io::stdout())).unwrap();
    }
}
