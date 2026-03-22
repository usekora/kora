use serde::{Deserialize, Serialize};

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
        (Stage::Researching, Stage::AwaitingApproval(_)) => true,
        (Stage::Reviewing, Stage::Judging) => true,
        (Stage::SecurityAuditing, Stage::Judging) => true,
        (Stage::Judging, Stage::Researching) => true,
        (Stage::Judging, Stage::Planning) => true,
        (Stage::Planning, Stage::TestArchitecting) => true,
        (Stage::TestArchitecting, Stage::Implementing) => true,
        (Stage::TestArchitecting, Stage::AwaitingApproval(_)) => true,
        (Stage::Implementing, Stage::Validating) => true,
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
