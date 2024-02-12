use std::error::Error;
use std::fs::File;
use std::path::PathBuf;
use utils::net::mac::MacAddr;
use vmm::builder::build_and_boot_microvm;
use vmm::devices::virtio::block_common::CacheType;
use vmm::resources::VmResources;
use vmm::seccomp_filters::get_empty_filters;
use vmm::vmm_config::boot_source::{BootConfig, BootSource, BootSourceConfig};
use vmm::vmm_config::drive::{BlockBuilder, BlockDeviceConfig};
use vmm::vmm_config::instance_info::{InstanceInfo, VmState};
use vmm::vmm_config::machine_config::VmConfig;
use vmm::vmm_config::net::{NetBuilder, NetworkInterfaceConfig};
use vmm::{EventManager, FcExitCode};

pub struct Vm {
    vcpu_count: u8,
    mem_size_mib: usize,
    kernel_cmdline: String,
    kernel_path: PathBuf,
    rootfs_path: PathBuf,
    rootfs_readonly: bool,
    /// Name of an unused TAP interface on the host, must exist
    tap_iface_name: String,
    /// Mac address - Leave blank for a default
    vm_mac: Option<[u8; 6]>,
}

impl Vm {
    pub fn make(&self) -> Result<(), Box<dyn Error>> {
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
            backed_by_hugepages: true,
        };
        let boot_source = BootSource {
            config: BootSourceConfig::default(),
            builder: Some(BootConfig {
                cmdline: linux_loader::cmdline::Cmdline::try_from(&self.kernel_cmdline, 4096)?,
                kernel_file: File::open(&self.kernel_path)?,
                initrd_file: None,
            }),
        };

        let mac = self.vm_mac.unwrap_or([0x0, 0x2, 0x0, 0x0, 0x0, 0x0]);
        let mut net_builder = NetBuilder::new();
        net_builder
            .build(NetworkInterfaceConfig {
                iface_id: "net0".to_string(),
                host_dev_name: self.tap_iface_name.clone(),
                guest_mac: Some(MacAddr::from_bytes_unchecked(&mac)),
                rx_rate_limiter: None,
                tx_rate_limiter: None,
            })
            .unwrap();

        let mut block = BlockBuilder::new();
        block
            .insert(BlockDeviceConfig {
                drive_id: "block1".to_string(),
                partuuid: None, //Some("0eaa91a0-01".to_string()),
                is_root_device: false,
                cache_type: CacheType::Unsafe,

                is_read_only: Some(self.rootfs_readonly),
                path_on_host: Some(self.rootfs_path.as_path().display().to_string()),
                rate_limiter: None,
                file_engine_type: None,

                socket: None,
            })
            .unwrap();
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

        let vm = build_and_boot_microvm(
            &instance_info,
            &vm_resources,
            &mut event_manager,
            &seccomp_filters,
        )?;
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
    use crate::Vm;
    use std::path::PathBuf;
    #[test]
    fn it_works() {
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
}
