use crate::registry::client::RegistryClient;
use crate::store::Store;

pub(super) mod node;

impl Store {
    pub fn unmanaged_client_add(&mut self, id: u32, device: RegistryClient) {
        // Only add this if the node isn't already managed
        self.unmanaged_clients.insert(id, device);
    }

    pub fn unmanaged_client_set_binary(&mut self, id: u32, name: String) {
        let nodes = if let Some(client) = self.unmanaged_clients.get_mut(&id) {
            client.application_binary = Some(name);
            client.nodes.clone()
        } else {
            vec![]
        };

        // Check all the client nodes to see if they were waiting for this
        for node in nodes {
            self.unmanaged_client_node_check(node);
        }
    }

    pub fn unmanaged_client_get(&mut self, id: u32) -> Option<&mut RegistryClient> {
        self.unmanaged_clients.get_mut(&id)
    }

    pub fn unmanaged_client_remove(&mut self, id: u32) {
        self.unmanaged_clients.remove(&id);
    }
}
