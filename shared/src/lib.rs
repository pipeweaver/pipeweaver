use clap::ValueEnum;
use enum_map::Enum;
use serde::{Deserialize, Serialize};
use std::fmt;
use std::fmt::Debug;
use std::str::FromStr;
use strum_macros::{Display, EnumIter};
use ulid::Ulid;

#[derive(
    Debug, Display, Copy, Clone, PartialEq, Enum, EnumIter, Serialize, Deserialize, ValueEnum,
)]
pub enum NodeType {
    PhysicalSource,
    PhysicalTarget,
    VirtualSource,
    VirtualTarget,
}

#[derive(
    Default, Debug, Copy, Clone, Enum, EnumIter, Serialize, Deserialize, PartialEq, ValueEnum,
)]
pub enum Mix {
    #[default]
    A,
    B,
}

#[derive(
    Default, Debug, Copy, Clone, Enum, EnumIter, Serialize, Deserialize, PartialEq, ValueEnum,
)]
pub enum DeviceType {
    #[default]
    Source,
    Target,
}

#[derive(
    Default, Debug, Copy, Clone, Enum, EnumIter, Serialize, Deserialize, PartialEq, ValueEnum,
)]
pub enum PortDirection {
    #[default]
    In,
    Out,
}

#[derive(
    Default, Debug, Copy, Clone, Enum, EnumIter, Serialize, Deserialize, Eq, PartialEq, ValueEnum,
)]
pub enum MuteState {
    #[default]
    Unmuted,
    Muted,
}

#[derive(
    Default,
    Debug,
    Copy,
    Clone,
    Hash,
    Enum,
    EnumIter,
    Serialize,
    Deserialize,
    Eq,
    PartialEq,
    ValueEnum,
)]
pub enum MuteTarget {
    #[default]
    TargetA,
    TargetB,
}

#[derive(
    Default,
    Debug,
    Copy,
    Clone,
    Hash,
    Enum,
    EnumIter,
    Serialize,
    Deserialize,
    Eq,
    PartialEq,
    ValueEnum,
)]
pub enum OrderGroup {
    #[default]
    Default,
    Pinned,
    Hidden,
}

#[derive(Default, Debug, Copy, Clone, Serialize, Deserialize, Eq, PartialEq, ValueEnum)]
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
    Quantum2048,
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
            Quantum::Quantum2048 => 2048,
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

#[derive(Debug)]
pub struct InvalidColour;

impl fmt::Display for InvalidColour {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "invalid colour (expected #RGB or #RRGGBB)")
    }
}

impl std::error::Error for InvalidColour {}

impl FromStr for Colour {
    type Err = InvalidColour;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let hex = s.strip_prefix('#').unwrap_or(s);

        match hex.len() {
            6 => {
                // Full form: RRGGBB
                Ok(Colour {
                    red: parse_byte(&hex[0..2])?,
                    green: parse_byte(&hex[2..4])?,
                    blue: parse_byte(&hex[4..6])?,
                })
            }
            3 => {
                // Shorthand: RGB → duplicate each nibble
                Ok(Colour {
                    red: parse_nibble(hex.as_bytes()[0])?,
                    green: parse_nibble(hex.as_bytes()[1])?,
                    blue: parse_nibble(hex.as_bytes()[2])?,
                })
            }
            _ => Err(InvalidColour),
        }
    }
}

fn parse_byte(s: &str) -> Result<u8, InvalidColour> {
    u8::from_str_radix(s, 16).map_err(|_| InvalidColour)
}

fn parse_nibble(b: u8) -> Result<u8, InvalidColour> {
    let value = match b {
        b'0'..=b'9' => b - b'0',
        b'a'..=b'f' => b - b'a' + 10,
        b'A'..=b'F' => b - b'A' + 10,
        _ => return Err(InvalidColour),
    };

    // Expand nibble: e.g. A → AA
    Ok((value << 4) | value)
}
