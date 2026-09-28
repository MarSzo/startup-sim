//! Shared game logic: used by the server binary and by the load-test bots.
//!
//! - world and movement: [`building`], [`map`], [`sim`], [`nav`], [`elevator`], [`stalls`]
//! - network: [`protocol`], [`net`], [`server`] (the authoritative game loop)
//! - features (pure rules and data; `server` wires them up): [`board`], [`cleaning`],
//!   [`clock`], [`coffee`], [`commute`], [`company`], [`computer`], [`fire`], [`inventory`],
//!   [`lights`], [`lunch`], [`needs`], [`npc`], [`recruitment`], [`security`], [`shop`],
//!   [`treats`], [`weather`]
//! - tools: [`args`]

pub mod args;
pub mod auth;
pub mod board;
pub mod building;
pub mod cleaning;
pub mod clock;
pub mod coffee;
pub mod commute;
pub mod company;
pub mod computer;
pub mod elevator;
pub mod fire;
pub mod http;
pub mod inventory;
pub mod kitchen;
pub mod lights;
pub mod lunch;
pub mod map;
pub mod nav;
pub mod needs;
pub mod net;
pub mod npc;
pub mod persist;
pub mod protocol;
pub mod recruitment;
pub mod security;
pub mod server;
pub mod shop;
pub mod sim;
pub mod stalls;
pub mod tasks;
pub mod treats;
pub mod weather;
pub mod workmail;
