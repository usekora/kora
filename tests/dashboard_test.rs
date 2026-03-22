use kora::terminal::dashboard::{render_bar, Dashboard};

#[test]
fn test_render_bar_zero_is_all_empty() {
    let bar = render_bar(0);
    assert_eq!(bar.chars().filter(|&c| c == '█').count(), 0);
    assert_eq!(bar.chars().filter(|&c| c == '░').count(), 12);
}

#[test]
fn test_render_bar_100_is_all_filled() {
    let bar = render_bar(100);
    assert_eq!(bar.chars().filter(|&c| c == '█').count(), 12);
    assert_eq!(bar.chars().filter(|&c| c == '░').count(), 0);
}

#[test]
fn test_render_bar_50_is_half() {
    let bar = render_bar(50);
    assert_eq!(bar.chars().filter(|&c| c == '█').count(), 6);
    assert_eq!(bar.chars().filter(|&c| c == '░').count(), 6);
}

#[test]
fn test_dashboard_new_total_tasks() {
    let dashboard = Dashboard::new(vec!["T1".to_string(), "T2".to_string(), "T3".to_string()]);
    assert_eq!(dashboard.total_tasks(), 3);
}

#[test]
fn test_dashboard_show_task_roundtrip() {
    let mut dashboard = Dashboard::new(vec!["T1".to_string()]);
    assert!(dashboard.showing_task().is_none());
    dashboard.set_show_task(Some("T1".to_string()));
    assert_eq!(dashboard.showing_task(), Some("T1"));
    dashboard.set_show_task(None);
    assert!(dashboard.showing_task().is_none());
}

#[test]
fn test_dashboard_task_order() {
    let dashboard = Dashboard::new(vec![
        "T1".to_string(),
        "T3".to_string(),
        "T2".to_string(),
    ]);
    assert_eq!(dashboard.task_order(), &["T1", "T3", "T2"]);
}
