# Pipeweaver Shared Objects

These objects are used in various places, such as the profile and status to display bits of information.

This crate comes with the following features, which can be enabled to add additional crate support:

* `serde` - Derive `Deserialize` and `Serialize` from the `serde` crate (Default)
* `enum-map` - Derive `Enum` from the `enum-map` crate
* `clap` - Derive `ValueEnum` from the `clap` crate
* `strum` - Derive `Display` and `EnumIter` from `strum` and `strum_macros` crates