//! Access-control policy: unattended access vs. "ask the local user".
//!
//! Two real-world modes a remote-access product must support:
//!
//! * **Unattended** — servers, kiosks, a developer's own always-on box: the
//!   operator connects without anyone sitting at the console. Allowed only for
//!   hosts/operators explicitly placed on an allow-list.
//! * **Attended ("ask user")** — a support technician helping a person at their
//!   desk: the person physically present must approve the incoming connection,
//!   and the grant lasts only for that session.
//!
//! This module decides, for a given operator + host + mode, whether to grant
//! immediately, prompt the local user, or deny outright. The actual prompt UI
//! lives in the agent/desktop client; this is the policy brain that is unit
//! testable in isolation.

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
pub enum AccessMode {
    /// No console prompt; rely on policy + credentials.
    Unattended,
    /// Require the local user to approve.
    AskUser,
}

/// The outcome of evaluating an access request.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AccessOutcome {
    /// Grant access right away (no prompt needed).
    Grant,
    /// Show a consent prompt to the local user; access depends on their answer.
    Prompt,
    /// Refuse the connection. Carries a human-readable reason.
    Deny(String),
}

/// Configurable access policy for a host/agent.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct AccessPolicy {
    /// Master switch: is unattended access permitted on this host at all?
    pub allow_unattended: bool,
    /// Operators (usernames) explicitly allowed to connect unattended.
    pub unattended_allowlist: Vec<String>,
    /// If true, an attended request is auto-denied when no local user is
    /// present to answer the prompt (nobody at the console). If false, an
    /// absent user is treated as implicit consent (legacy behaviour).
    pub require_present_user_for_attended: bool,
}

impl Default for AccessPolicy {
    fn default() -> Self {
        // Secure-by-default: attended only, no unattended allow-list, and
        // refuse attended requests when nobody is at the console.
        Self {
            allow_unattended: false,
            unattended_allowlist: Vec::new(),
            require_present_user_for_attended: true,
        }
    }
}

/// Context describing the host state at the moment of the request.
#[derive(Debug, Clone, Copy)]
pub struct HostContext {
    /// Is a local interactive user currently logged in at the console?
    pub local_user_present: bool,
}

impl AccessPolicy {
    /// Decide what to do with an access request.
    pub fn evaluate(
        &self,
        operator: &str,
        mode: AccessMode,
        ctx: HostContext,
    ) -> AccessOutcome {
        match mode {
            AccessMode::Unattended => {
                if !self.allow_unattended {
                    return AccessOutcome::Deny(
                        "Unattended access is disabled on this host".to_string(),
                    );
                }
                if self
                    .unattended_allowlist
                    .iter()
                    .any(|o| o.eq_ignore_ascii_case(operator))
                {
                    AccessOutcome::Grant
                } else {
                    AccessOutcome::Deny(format!(
                        "Operator '{operator}' is not on the unattended allow-list"
                    ))
                }
            }
            AccessMode::AskUser => {
                if ctx.local_user_present {
                    AccessOutcome::Prompt
                } else if self.require_present_user_for_attended {
                    AccessOutcome::Deny(
                        "No local user is present to approve the connection".to_string(),
                    )
                } else {
                    // Legacy fall-through: nobody to ask, implicitly allow.
                    AccessOutcome::Grant
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn present() -> HostContext {
        HostContext {
            local_user_present: true,
        }
    }
    fn absent() -> HostContext {
        HostContext {
            local_user_present: false,
        }
    }

    #[test]
    fn unattended_denied_by_default() {
        let policy = AccessPolicy::default();
        assert_eq!(
            policy.evaluate("operator", AccessMode::Unattended, absent()),
            AccessOutcome::Deny("Unattended access is disabled on this host".to_string())
        );
    }

    #[test]
    fn unattended_allowed_only_for_allowlisted_operator() {
        let policy = AccessPolicy {
            allow_unattended: true,
            unattended_allowlist: vec!["alice".to_string()],
            ..Default::default()
        };
        assert_eq!(
            policy.evaluate("Alice", AccessMode::Unattended, absent()),
            AccessOutcome::Grant
        );
        assert!(matches!(
            policy.evaluate("mallory", AccessMode::Unattended, absent()),
            AccessOutcome::Deny(_)
        ));
    }

    #[test]
    fn attended_prompts_when_user_present() {
        let policy = AccessPolicy::default();
        assert_eq!(
            policy.evaluate("tech", AccessMode::AskUser, present()),
            AccessOutcome::Prompt
        );
    }

    #[test]
    fn attended_denied_when_no_user_and_strict() {
        let policy = AccessPolicy::default();
        assert!(matches!(
            policy.evaluate("tech", AccessMode::AskUser, absent()),
            AccessOutcome::Deny(_)
        ));
    }

    #[test]
    fn attended_grants_when_no_user_and_lenient() {
        let policy = AccessPolicy {
            require_present_user_for_attended: false,
            ..Default::default()
        };
        assert_eq!(
            policy.evaluate("tech", AccessMode::AskUser, absent()),
            AccessOutcome::Grant
        );
    }
}
