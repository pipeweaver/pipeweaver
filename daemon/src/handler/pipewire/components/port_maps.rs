use anyhow::Result;
use ulid::Ulid;

#[allow(unused)]
pub trait PortMap {
    fn create_port_map(node_id: u32, name: String, left: String, right: String) -> Result<()>;
    fn delete_port_map(id: Ulid) -> Result<()>;

    fn attach_port_map(target: Ulid, map: Ulid) -> Result<()>;
    fn detach_port_map(target: Ulid, map: Ulid) -> Result<()>;
}
