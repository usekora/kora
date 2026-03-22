use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;

/// Runtime pipeline profile — determines which stages execute for a given request.
/// Selected automatically by the Researcher's classification or overridden via `--profile`.
#[derive(Debug, Default, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PipelineProfile {
    /// Typo, rename, config tweak. Research → single implementor → merge.
    Trivial,
    /// Bug fix, small feature. Research → plan → implement → code review → validate.
    Simple,
    /// Full pipeline (default). Research → review loop → plan → test architect → fleet → code review → validate.
    #[default]
    Standard,
    /// Auth, payments, user data. Full pipeline with all security agents force-enabled.
    SecurityCritical,
}

impl PipelineProfile {
    pub fn has_review_loop(self) -> bool {
        matches!(
            self,
            PipelineProfile::Standard | PipelineProfile::SecurityCritical
        )
    }

    pub fn has_planner(self) -> bool {
        !matches!(self, PipelineProfile::Trivial)
    }

    pub fn has_test_architect(self) -> bool {
        matches!(
            self,
            PipelineProfile::Standard | PipelineProfile::SecurityCritical
        )
    }

    pub fn has_code_review(self) -> bool {
        !matches!(self, PipelineProfile::Trivial)
    }

    pub fn has_security_audit(self) -> bool {
        matches!(
            self,
            PipelineProfile::Standard | PipelineProfile::SecurityCritical
        )
    }

    pub fn has_validation(self) -> bool {
        !matches!(self, PipelineProfile::Trivial)
    }
}

impl fmt::Display for PipelineProfile {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PipelineProfile::Trivial => write!(f, "trivial"),
            PipelineProfile::Simple => write!(f, "simple"),
            PipelineProfile::Standard => write!(f, "standard"),
            PipelineProfile::SecurityCritical => write!(f, "security-critical"),
        }
    }
}

impl FromStr for PipelineProfile {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.trim().to_lowercase().as_str() {
            "trivial" => Ok(PipelineProfile::Trivial),
            "simple" => Ok(PipelineProfile::Simple),
            "standard" => Ok(PipelineProfile::Standard),
            "security-critical" | "security_critical" | "securitycritical" => {
                Ok(PipelineProfile::SecurityCritical)
            }
            _ => Err(format!(
                "unknown pipeline profile '{}' (expected: trivial, simple, standard, security-critical)",
                s
            )),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum Checkpoint {
    AfterResearcher,
    AfterReviewLoop,
    AfterPlanner,
    AfterImplementation,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum Stage {
    Researching,
    Reviewing,
    SecurityAuditing,
    Judging,
    Planning,
    TestArchitecting,
    Implementing,
    Validating,
    Fixing,
    AwaitingApproval(Box<Stage>),
    Complete,
    Failed(String),
}

impl Stage {
    pub fn label(&self) -> &str {
        match self {
            Stage::Researching => "researcher",
            Stage::Reviewing => "reviewer",
            Stage::SecurityAuditing => "security auditor",
            Stage::Judging => "judge",
            Stage::Planning => "planner",
            Stage::TestArchitecting => "test architect",
            Stage::Implementing => "implementing",
            Stage::Validating => "validator",
            Stage::Fixing => "fixing",
            Stage::AwaitingApproval(_) => "awaiting approval",
            Stage::Complete => "complete",
            Stage::Failed(_) => "failed",
        }
    }
}

pub fn can_transition(from: &Stage, to: &Stage) -> bool {
    match (from, to) {
        (Stage::Researching, Stage::Reviewing) => true,
        (Stage::Researching, Stage::SecurityAuditing) => true,
        (Stage::Researching, Stage::Planning) => true, // profiles that skip review loop
        (Stage::Researching, Stage::AwaitingApproval(_)) => true,
        (Stage::Reviewing, Stage::Judging) => true,
        (Stage::SecurityAuditing, Stage::Judging) => true,
        (Stage::Judging, Stage::Researching) => true,
        (Stage::Judging, Stage::Planning) => true,
        (Stage::Planning, Stage::TestArchitecting) => true,
        (Stage::Planning, Stage::Implementing) => true, // profiles that skip test architect
        (Stage::TestArchitecting, Stage::Implementing) => true,
        (Stage::TestArchitecting, Stage::AwaitingApproval(_)) => true,
        (Stage::Implementing, Stage::Validating) => true,
        (Stage::Implementing, Stage::Complete) => true, // profiles that skip validation
        (Stage::Validating, Stage::Fixing) => true,
        (Stage::Validating, Stage::Complete) => true,
        (Stage::Fixing, Stage::Validating) => true,
        (Stage::AwaitingApproval(next), to) if next.as_ref() == to => true,
        _ => false,
    }
}

pub fn checkpoint_for_stage(next_stage: &Stage, checkpoints: &[Checkpoint]) -> Option<Checkpoint> {
    match next_stage {
        Stage::Reviewing => {
            if checkpoints.contains(&Checkpoint::AfterResearcher) {
                Some(Checkpoint::AfterResearcher)
            } else {
                None
            }
        }
        Stage::Planning => {
            if checkpoints.contains(&Checkpoint::AfterReviewLoop) {
                Some(Checkpoint::AfterReviewLoop)
            } else {
                None
            }
        }
        Stage::Implementing => {
            if checkpoints.contains(&Checkpoint::AfterPlanner) {
                Some(Checkpoint::AfterPlanner)
            } else {
                None
            }
        }
        Stage::Complete => {
            if checkpoints.contains(&Checkpoint::AfterImplementation) {
                Some(Checkpoint::AfterImplementation)
            } else {
                None
            }
        }
        _ => None,
    }
}
