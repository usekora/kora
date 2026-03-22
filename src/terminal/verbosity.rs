use crate::config::Verbosity;

pub struct VerbosityState {
    current: Verbosity,
}

impl VerbosityState {
    pub fn new(default: Verbosity) -> Self {
        Self { current: default }
    }

    pub fn current(&self) -> Verbosity {
        self.current
    }

    pub fn cycle(&mut self) -> Verbosity {
        self.current = match self.current {
            Verbosity::Focused => Verbosity::Detailed,
            Verbosity::Detailed => Verbosity::Verbose,
            Verbosity::Verbose => Verbosity::Focused,
        };
        self.current
    }

    pub fn label(&self) -> &'static str {
        match self.current {
            Verbosity::Focused => "focused",
            Verbosity::Detailed => "detailed",
            Verbosity::Verbose => "verbose",
        }
    }
}
