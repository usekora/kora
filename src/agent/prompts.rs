pub const RESEARCHER_PROMPT: &str = include_str!("../../prompts/researcher.md");
pub const REVIEWER_PROMPT: &str = include_str!("../../prompts/reviewer.md");
pub const SECURITY_AUDITOR_PROMPT: &str = include_str!("../../prompts/security_auditor.md");
pub const JUDGE_PROMPT: &str = include_str!("../../prompts/judge.md");
pub const PLANNER_PROMPT: &str = include_str!("../../prompts/planner.md");
pub const TEST_ARCHITECT_PROMPT: &str = include_str!("../../prompts/test_architect.md");
pub const IMPLEMENTOR_PROMPT: &str = include_str!("../../prompts/implementor.md");
pub const VALIDATOR_PROMPT: &str = include_str!("../../prompts/validator.md");

pub fn assemble_prompt(base: &str, custom_instructions: Option<&str>, context: &str) -> String {
    let mut prompt = base.to_string();

    if let Some(custom) = custom_instructions {
        prompt.push_str("\n\n---\n\n## Additional Instructions\n\n");
        prompt.push_str(custom);
    }

    prompt.push_str("\n\n---\n\n");
    prompt.push_str(context);

    prompt
}
