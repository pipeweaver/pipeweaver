use enum_map::Enum;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
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

#[derive(
    Default, Debug, Copy, Clone, Hash, Enum, EnumIter, Serialize, Deserialize, Eq, PartialEq,
)]
pub enum MuteTarget {
    #[default]
    TargetA,
    TargetB,
}

#[derive(
    Default, Debug, Copy, Clone, Hash, Enum, EnumIter, Serialize, Deserialize, Eq, PartialEq,
)]
pub enum OrderGroup {
    #[default]
    Default,
    Pinned,
    Hidden,
}

#[derive(Default, Debug, Copy, Clone, Serialize, Deserialize, Eq, PartialEq)]
pub enum Quantum {
    Quantum8,
    Quantum16,
    Quantum32,
    Quantum64,
    Quantum128,
    Quantum256,
    #[default]
    Quantum512,
    Quantum1024,
}

impl From<Quantum> for u32 {
    fn from(value: Quantum) -> Self {
        match value {
            Quantum::Quantum8 => 8,
            Quantum::Quantum16 => 16,
            Quantum::Quantum32 => 32,
            Quantum::Quantum64 => 64,
            Quantum::Quantum128 => 128,
            Quantum::Quantum256 => 256,
            Quantum::Quantum512 => 512,
            Quantum::Quantum1024 => 1024,
        }
    }
}

impl From<u32> for Quantum {
    fn from(value: u32) -> Self {
        match value {
            8 => Quantum::Quantum8,
            16 => Quantum::Quantum16,
            32 => Quantum::Quantum32,
            64 => Quantum::Quantum64,
            128 => Quantum::Quantum128,
            256 => Quantum::Quantum256,
            512 => Quantum::Quantum512,
            1024 => Quantum::Quantum1024,
            _ => panic!("Unsupported quantum size: {}", value),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Eq, PartialEq, Hash, Clone)]
pub enum AppTarget {
    Managed(Ulid),
    Unmanaged(u32),
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
pub struct AppDefinition {
    pub device_type: DeviceType,
    pub process: String,
    pub name: String,
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FilterValue {
    Int32(i32),
    Float32(f32),
    UInt8(u8),
    UInt32(u32),
    String(String),
    Bool(bool),
    Enum(String, u32),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FilterProperty {
    pub id: u32,
    pub name: String,
    pub symbol: String,
    pub value: FilterValue,

    pub min: f32,
    pub max: f32,

    pub is_input: bool,

    pub enum_def: Option<HashMap<u32, String>>,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct FilterConfig {
    pub name: String,
    pub parameters: Vec<FilterProperty>,
}
