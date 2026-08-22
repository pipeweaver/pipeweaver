//! Data types owned by the `Store`: the managed-object records (`NodeStore`,
//! `FilterStore`, `LinkStore`, ...) plus small value types shared across the
//! `store` submodules. Nothing in here touches `Store` itself - see the
//! individual submodules (`nodes`, `filters`, `links`, ...) for behaviour.

use crate::manager::FilterData;
use crate::{Direction, LinkType};
use anyhow::{Result, bail};
use enum_map::{Enum, EnumMap};
use oneshot::Sender;
use parking_lot::RwLock;
use pipewire::filter::{FilterListener, FilterPort, FilterRc};
use pipewire::link::{Link, LinkListener};
use pipewire::node::{Node, NodeListener, NodeState};
use pipewire::properties::PropertiesBox;
use pipewire::proxy::ProxyListener;
use std::cell::RefCell;
use std::collections::HashMap;
use std::fmt::{Display, Formatter};
use std::rc::Rc;
use std::str::FromStr;
use strum_macros::EnumIter;
use ulid::Ulid;

/// How a client is asking to target a node: either by raw pipewire node id,
/// or by object serial (used when the target was learned from metadata).
pub(crate) enum TargetType {
    Node(Option<u32>),
    Serial(Option<u32>),
}

pub(crate) struct NodeStore {
    pub(crate) pw_id: Option<u32>,
    pub(crate) object_serial: Option<u32>,

    pub(crate) id: Ulid,
    pub(crate) props: PropertiesBox,

    pub(crate) proxy: Node,
    pub(crate) _proxy_listener: ProxyListener,
    pub(crate) _listener: NodeListener,

    // Nodes will always have inputs and outputs which directly link together, so we
    // don't need to track each side, we just need the ID and Location
    pub(crate) port_map: EnumMap<PortLocation, Option<u32>>,
    pub(crate) ports_ready: bool,
    pub(crate) node_state: NodeStoreState,

    pub(crate) ready_sender: Option<Option<Sender<()>>>,
}

#[derive(Debug, Clone)]
pub enum NodeStoreState {
    Error(String),
    Creating,
    Suspending,
    Idle,
    Running,
}

impl From<NodeState<'_>> for NodeStoreState {
    fn from(state: NodeState) -> Self {
        match state {
            NodeState::Error(e) => NodeStoreState::Error(e.to_owned()),
            NodeState::Creating => NodeStoreState::Creating,
            NodeState::Suspended => NodeStoreState::Suspending,
            NodeState::Idle => NodeStoreState::Idle,
            NodeState::Running => NodeStoreState::Running,
        }
    }
}

pub struct FilterStore {
    /// The Pipewire Node ID for this Filter
    pub(crate) pw_id: Option<u32>,

    pub(crate) _listener: FilterListener<Rc<RwLock<FilterData>>>,

    /// The Ulid Identifier
    pub(crate) id: Ulid,

    // This maintains a general port map of location -> index
    pub(crate) port_map: EnumMap<Direction, EnumMap<PortLocation, u32>>,

    /// Details of the ports assigned to this filter
    pub(crate) _input_ports: Rc<RefCell<Vec<FilterPort>>>,
    pub(crate) _output_ports: Rc<RefCell<Vec<FilterPort>>>,

    /// These two fields need to exist purely to prevent the filter and the listener from
    /// being dropped, they're never directly accessed, they're just a store.
    pub(crate) _filter: FilterRc,

    /// The 'Ready Sender' is called once the filter is setup and ready-to-go
    pub(crate) ready_sender: Option<Option<Sender<()>>>,

    /// The Data related to the filter, including the sample processing callback
    pub data: Rc<RwLock<FilterData>>,
}

pub struct LinkStore {
    pub(crate) source: LinkType,
    pub(crate) destination: LinkType,

    pub(crate) links: EnumMap<PortLocation, Option<LinkStoreMap>>,

    pub(crate) ready_sender: Option<Sender<()>>,
}

pub struct LinkStoreMap {
    pub(crate) pw_id: Option<u32>,

    /// An internal ID so we can find this link
    pub(crate) internal_id: Ulid,

    /// Variables needed to keep this link alive
    pub(crate) pending_seq_id: Option<i32>,
    pub(crate) _link: Option<Link>,
    pub(crate) _proxy_listener: Option<ProxyListener>,
    pub(crate) _info_listener: Option<LinkListener>,

    /// Internal Port Index Mapping
    pub(crate) source_port: (u32, u32),
    pub(crate) destination_port: (u32, u32),
}

#[derive(Debug, Enum, EnumIter, Copy, Clone, PartialEq)]
pub(crate) enum PortLocation {
    Left,
    Right,
}

impl Display for PortLocation {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            PortLocation::Left => write!(f, "FL"),
            PortLocation::Right => write!(f, "FR"),
        }
    }
}

impl FromStr for PortLocation {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "FL" | "AUX0" => Ok(Self::Left),
            "FR" | "AUX1" => Ok(Self::Right),
            _ => bail!("Unknown Channel"),
        }
    }
}

pub struct PendingLinkSync {
    pub parent_id: Ulid,
    pub group: LinkStore,
    pub bound_ids: HashMap<Ulid, u32>, // link_id -> pw_id collected during sync wait
}
