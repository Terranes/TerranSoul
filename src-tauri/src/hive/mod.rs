//! Hive protocol — opt-in federation for TerranSoul instances.
//!
//! Spec: `docs/hive-protocol.md`
//!
//! This module defines the wire types for BUNDLE, OP, and JOB messages,
//! plus Ed25519 signing/verification helpers that operate on the existing
//! `DeviceIdentity` from `identity/`.

pub mod jobs;
pub mod privacy;
pub mod protocol;
pub mod signing;
