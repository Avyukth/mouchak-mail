//! Strong newtypes for domain identifiers.
//!
//! These newtypes provide compile-time type safety, preventing accidental
//! misuse of IDs (e.g., passing a `MessageId` where a `ProjectId` is expected).
//!
//! # Example
//!
//! ```
//! use lib_core::types::{ProjectId, AgentId, MessageId};
//!
//! fn get_agent(project: ProjectId, agent: AgentId) {
//!     // Compile error if you swap project and agent
//! }
//! ```

use serde::{Deserialize, Serialize};
use std::fmt;

/// Project identifier (database primary key).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ProjectId(pub i64);

impl ProjectId {
    /// Create a new ProjectId.
    #[inline]
    pub const fn new(id: i64) -> Self {
        Self(id)
    }

    /// Get the raw i64 value.
    #[inline]
    pub const fn get(self) -> i64 {
        self.0
    }
}

impl From<i64> for ProjectId {
    fn from(id: i64) -> Self {
        Self(id)
    }
}

impl From<ProjectId> for i64 {
    fn from(id: ProjectId) -> Self {
        id.0
    }
}

impl fmt::Display for ProjectId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Agent identifier (database primary key).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct AgentId(pub i64);

impl AgentId {
    /// Create a new AgentId.
    #[inline]
    pub const fn new(id: i64) -> Self {
        Self(id)
    }

    /// Get the raw i64 value.
    #[inline]
    pub const fn get(self) -> i64 {
        self.0
    }
}

impl From<i64> for AgentId {
    fn from(id: i64) -> Self {
        Self(id)
    }
}

impl From<AgentId> for i64 {
    fn from(id: AgentId) -> Self {
        id.0
    }
}

impl fmt::Display for AgentId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Message identifier (database primary key).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct MessageId(pub i64);

impl MessageId {
    /// Create a new MessageId.
    #[inline]
    pub const fn new(id: i64) -> Self {
        Self(id)
    }

    /// Get the raw i64 value.
    #[inline]
    pub const fn get(self) -> i64 {
        self.0
    }
}

impl From<i64> for MessageId {
    fn from(id: i64) -> Self {
        Self(id)
    }
}

impl From<MessageId> for i64 {
    fn from(id: MessageId) -> Self {
        id.0
    }
}

impl fmt::Display for MessageId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Project slug (URL-safe identifier).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ProjectSlug(pub String);

impl ProjectSlug {
    /// Create a new ProjectSlug.
    pub fn new(slug: impl Into<String>) -> Self {
        Self(slug.into())
    }

    /// Get the raw string value.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl From<String> for ProjectSlug {
    fn from(s: String) -> Self {
        Self(s)
    }
}

impl From<&str> for ProjectSlug {
    fn from(s: &str) -> Self {
        Self(s.to_string())
    }
}

impl AsRef<str> for ProjectSlug {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for ProjectSlug {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Agent name identifier.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct AgentName(pub String);

impl AgentName {
    /// Create a new AgentName.
    pub fn new(name: impl Into<String>) -> Self {
        Self(name.into())
    }

    /// Get the raw string value.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl From<String> for AgentName {
    fn from(s: String) -> Self {
        Self(s)
    }
}

impl From<&str> for AgentName {
    fn from(s: &str) -> Self {
        Self(s.to_string())
    }
}

impl AsRef<str> for AgentName {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for AgentName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Thread identifier (conversation thread ID).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ThreadId(pub String);

impl ThreadId {
    /// Create a new ThreadId.
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    /// Get the raw string value.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl From<String> for ThreadId {
    fn from(s: String) -> Self {
        Self(s)
    }
}

impl From<&str> for ThreadId {
    fn from(s: &str) -> Self {
        Self(s.to_string())
    }
}

impl AsRef<str> for ThreadId {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for ThreadId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

// ============================================================================
// Kani Formal Verification Proofs
// ============================================================================
//
// Run with: cargo kani --package lib-core
// These proofs mathematically verify type invariants and conversion safety.

#[cfg(kani)]
mod verification {
    use super::*;

    /// Proof: ProjectId roundtrip conversion is identity
    #[kani::proof]
    fn proof_project_id_roundtrip() {
        let value: i64 = kani::any();
        let id = ProjectId::new(value);
        kani::assert(id.get() == value, "ProjectId roundtrip must preserve value");
    }

    /// Proof: ProjectId From<i64> and Into<i64> are inverses
    #[kani::proof]
    fn proof_project_id_from_into() {
        let value: i64 = kani::any();
        let id: ProjectId = value.into();
        let back: i64 = id.into();
        kani::assert(back == value, "From/Into must be inverses");
    }

    /// Proof: AgentId roundtrip conversion is identity
    #[kani::proof]
    fn proof_agent_id_roundtrip() {
        let value: i64 = kani::any();
        let id = AgentId::new(value);
        kani::assert(id.get() == value, "AgentId roundtrip must preserve value");
    }

    /// Proof: MessageId roundtrip conversion is identity
    #[kani::proof]
    fn proof_message_id_roundtrip() {
        let value: i64 = kani::any();
        let id = MessageId::new(value);
        kani::assert(id.get() == value, "MessageId roundtrip must preserve value");
    }

    /// Proof: ProjectId equality is reflexive
    #[kani::proof]
    fn proof_project_id_eq_reflexive() {
        let value: i64 = kani::any();
        let id = ProjectId::new(value);
        kani::assert(id == id, "ProjectId equality must be reflexive");
    }

    /// Proof: Different values produce different IDs
    #[kani::proof]
    fn proof_project_id_different() {
        let a: i64 = kani::any();
        let b: i64 = kani::any();
        kani::assume(a != b);
        let id_a = ProjectId::new(a);
        let id_b = ProjectId::new(b);
        kani::assert(id_a != id_b, "Different values must produce different IDs");
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;

    #[test]
    fn test_project_id_conversion() {
        let id = ProjectId::new(42);
        assert_eq!(id.get(), 42);
        assert_eq!(i64::from(id), 42);

        let id2: ProjectId = 100.into();
        assert_eq!(id2.get(), 100);
    }

    #[test]
    fn test_project_slug_conversion() {
        let slug = ProjectSlug::new("my-project");
        assert_eq!(slug.as_str(), "my-project");

        let slug2: ProjectSlug = "another-project".into();
        assert_eq!(slug2.as_str(), "another-project");
    }

    #[test]
    fn test_serde_transparent() {
        let id = ProjectId::new(42);
        let json = serde_json::to_string(&id).unwrap();
        assert_eq!(json, "42");

        let slug = ProjectSlug::new("test-slug");
        let json = serde_json::to_string(&slug).unwrap();
        assert_eq!(json, "\"test-slug\"");
    }

    #[test]
    fn test_type_safety() {
        // This test documents the type safety benefit:
        // The following would NOT compile:
        // fn needs_project_id(_: ProjectId) {}
        // let agent_id = AgentId::new(1);
        // needs_project_id(agent_id); // Compile error!

        let project_id = ProjectId::new(1);
        let agent_id = AgentId::new(1);

        // They have the same inner value but are different types
        assert_eq!(project_id.get(), agent_id.get());
        // But they're not equal (different types)
        // project_id == agent_id would not compile
    }
}
