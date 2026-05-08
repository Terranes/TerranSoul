//! TerranSoul Hive Relay — reference federation server.
//!
//! Accepts Ed25519-signed envelopes (BUNDLE, OP, JOB), verifies signatures,
//! persists bundles to Postgres, routes OPs to subscribers, and manages
//! the distributed job queue.

pub mod db;
pub mod relay;
pub mod verify;

pub mod proto {
    tonic::include_proto!("hive");
}
