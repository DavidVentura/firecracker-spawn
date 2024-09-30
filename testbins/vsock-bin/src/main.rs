use std::io::Write;

use vsock::{VsockStream, VMADDR_CID_HOST};
fn main() {
    let mut s = VsockStream::connect_with_cid_port(VMADDR_CID_HOST, 1234).unwrap();
    let buf = vec![0x41, 0x42, 0x43, 0x44, 0x45, 0xa];
    s.write_all(&buf).unwrap();
}
