//! Topology module for Sonos system discovery and management
//!
//! This module provides functionality for discovering and managing the topology
//! of a Sonos system, including zone groups, speakers, and their relationships.

pub mod client;
pub mod constants;
pub mod parser;
pub mod types;
pub mod impls;
pub mod utils;

// Re-export all public types and functions to maintain backward compatibility
pub use types::{Topology, ZoneGroup, ZoneGroupMember, Satellite, VanishedDevices, VanishedDevice};
pub use client::{TopologyClient, get_topology_from_ip};