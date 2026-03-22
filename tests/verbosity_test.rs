use kora::config::Verbosity;
use kora::terminal::VerbosityState;

#[test]
fn test_verbosity_starts_at_default() {
    let state = VerbosityState::new(Verbosity::Focused);
    assert_eq!(state.current(), Verbosity::Focused);
    assert_eq!(state.label(), "focused");
}

#[test]
fn test_verbosity_cycles_through_modes() {
    let mut state = VerbosityState::new(Verbosity::Focused);

    assert_eq!(state.cycle(), Verbosity::Detailed);
    assert_eq!(state.label(), "detailed");

    assert_eq!(state.cycle(), Verbosity::Verbose);
    assert_eq!(state.label(), "verbose");

    assert_eq!(state.cycle(), Verbosity::Focused);
    assert_eq!(state.label(), "focused");
}
