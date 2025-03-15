use enum_map::Enum;
use serde::{Deserialize, Serialize};
use strum_macros::{Display, EnumIter};

#[derive(Debug, Display, Copy, Clone, PartialEq, Enum, EnumIter, Serialize, Deserialize)]
pub enum NodeType {
    PhysicalSource,
    PhysicalTarget,
    VirtualSource,
    VirtualTarget,
}

#[derive(Default, Debug, Display, Copy, Clone, Enum, EnumIter, Serialize, Deserialize)]
pub enum Mix {
    #[default]
    A,
    B,
}

#[derive(Default, Debug, Copy, Clone, Serialize, Deserialize)]
pub enum MuteState {
    #[default]
    Unmuted,
    MuteTargetA,
    MuteTargetB,
}

#[derive(Default, Debug, Copy, Clone, Enum, EnumIter, Serialize, Deserialize)]
pub enum MuteTarget {
    #[default]
    TargetA,
    TargetB,
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