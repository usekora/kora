use crate::config::Config;
use crate::state::RunState;
use crate::terminal::verbosity::VerbosityState;
use crate::terminal::Renderer;

#[derive(Debug, PartialEq)]
pub enum MetaCommand {
    Status,
    Config,
    Verbose,
    Help,
    Quit,
    None(String),
}

pub fn parse_meta_command(input: &str) -> MetaCommand {
    let trimmed = input.trim();
    match trimmed {
        "/status" => MetaCommand::Status,
        "/config" => MetaCommand::Config,
        "/verbose" => MetaCommand::Verbose,
        "/help" => MetaCommand::Help,
        "/quit" | "/exit" => MetaCommand::Quit,
        _ => MetaCommand::None(trimmed.to_string()),
    }
}

pub fn handle_status(renderer: &mut Renderer, last_run: Option<&RunState>) {
    match last_run {
        Some(run) => {
            let status_label = run.status.label();
            let request_display = truncate_request(&run.request, 50);
            renderer.text(&format!(
                "last run: \"{}\" · {} · id {}",
                request_display, status_label, run.id
            ));
        }
        None => {
            renderer.info("no runs in this session yet");
        }
    }
}

pub fn handle_config(renderer: &mut Renderer, config: &Config) {
    renderer.text(&format!("provider: {}", config.default_provider));
    renderer.text(&format!(
        "checkpoints: {}",
        if config.checkpoints.is_empty() {
            "none".to_string()
        } else {
            config
                .checkpoints
                .iter()
                .map(|c| format!("{:?}", c))
                .collect::<Vec<_>>()
                .join(", ")
        }
    ));
    renderer.text(&format!(
        "review loop: max {} iterations",
        config.review_loop.max_iterations
    ));
    renderer.text(&format!(
        "validation loop: max {} iterations",
        config.validation_loop.max_iterations
    ));
    renderer.text(&format!(
        "parallel limit: {}",
        config.implementation.parallel_limit
    ));
    renderer.text(&format!("verbosity: {:?}", config.output.default_verbosity));
}

pub fn handle_verbose(renderer: &mut Renderer, verbosity: &mut VerbosityState) {
    let new_level = verbosity.cycle();
    renderer.info(&format!("verbosity: {:?}", new_level));
}

pub fn handle_help(renderer: &mut Renderer) {
    renderer.text("/status   — current/recent run info");
    renderer.text("/config   — show configuration");
    renderer.text("/verbose  — cycle verbosity level");
    renderer.text("/help     — this help message");
    renderer.text("/quit     — exit kora");
    renderer.text("");
    renderer.text("or type a request to start a new run");
}

fn truncate_request(request: &str, max_len: usize) -> String {
    let first_line = request.lines().next().unwrap_or(request);
    if first_line.len() > max_len {
        format!("{}...", &first_line[..max_len])
    } else {
        first_line.to_string()
    }
}
