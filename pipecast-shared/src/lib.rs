use enum_map::Enum;
use serde::{Deserialize, Serialize};
use strum_macros::{Display, EnumIter};

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