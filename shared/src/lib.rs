use enum_map::Enum;
use serde::{Deserialize, Serialize};
use std::fmt::Debug;
use strum_macros::{Display, EnumIter};
use ulid::Ulid;

#[derive(Debug, Display, Copy, Clone, PartialEq, Enum, EnumIter, Serialize, Deserialize)]
pub enum NodeType {
    PhysicalSource,
    PhysicalTarget,
    VirtualSource,
    VirtualTarget,
}

#[derive(Default, Debug, Copy, Clone, Enum, EnumIter, Serialize, Deserialize, PartialEq)]
pub enum Mix {
    #[default]
    A,
    B,
}

#[derive(Default, Debug, Copy, Clone, Enum, EnumIter, Serialize, Deserialize, PartialEq)]
pub enum DeviceType {
    #[default]
    Source,
    Target,
}

#[derive(Default, Debug, Copy, Clone, Serialize, Deserialize, Eq, PartialEq, Enum, EnumIter)]
pub enum MuteState {
    #[default]
    Unmuted,
    Muted,
}

#[derive(Default, Debug, Copy, Clone, Hash, Enum, EnumIter, Serialize, Deserialize, Eq, PartialEq)]
pub enum MuteTarget {
    #[default]
    TargetA,
    TargetB,
}

#[derive(Default, Debug, Copy, Clone, Hash, Enum, EnumIter, Serialize, Deserialize, Eq, PartialEq)]
pub enum OrderGroup {
    #[default]
    Default,
    Pinned,
    Hidden,
}

#[derive(Debug, Serialize, Deserialize, Eq, PartialEq, Hash, Clone)]
pub enum ApplicationMatch {
    Exact(String, Ulid),
    Glob(String, Ulid),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Colour {
    pub red: u8,
    pub green: u8,
    pub blue: u8,
}

impl Default for Colour {
    fn default() -> Self {
        Colour {
            red: 255,
            green: 255,
            blue: 0,
        }
    }
}