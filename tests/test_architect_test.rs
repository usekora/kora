use kora::agent::output_parser::parse_test_strategy;

#[test]
fn test_parse_test_strategy_with_multiple_tasks() {
    let json = r#"{
        "per_task": {
            "T1": {
                "unit_tests": [
                    {
                        "description": "Test theme context creation",
                        "file": "src/theme.test.ts",
                        "setup": "mock localStorage",
                        "expected": "context provides theme values",
                        "rationale": "core functionality"
                    }
                ],
                "integration_tests": [],
                "edge_case_tests": []
            },
            "T2": {
                "unit_tests": [],
                "integration_tests": [
                    {
                        "description": "Test CSS vars applied on theme change",
                        "file": "src/integration.test.ts",
                        "setup": "render app with theme provider",
                        "expected": "CSS variables update",
                        "rationale": "integration between T1 and T2"
                    }
                ],
                "edge_case_tests": []
            }
        },
        "post_merge": {
            "integration_tests": [
                {
                    "description": "Full theme toggle end to end",
                    "tasks_involved": ["T1", "T2"],
                    "setup": "render full app",
                    "expected": "theme toggles and CSS updates",
                    "rationale": "cross-task integration"
                }
            ]
        },
        "testing_patterns": {
            "framework": "jest",
            "conventions": "describe/it with RTL"
        }
    }"#;
    let strategy = parse_test_strategy(json).unwrap();
    assert_eq!(strategy.per_task.len(), 2);
    assert_eq!(strategy.per_task["T1"].unit_tests.len(), 1);
    assert_eq!(strategy.per_task["T2"].integration_tests.len(), 1);
    assert_eq!(strategy.post_merge.integration_tests.len(), 1);
    assert_eq!(
        strategy.post_merge.integration_tests[0].tasks_involved,
        vec!["T1", "T2"]
    );
}

#[test]
fn test_parse_test_strategy_empty_per_task() {
    let json = "{
        \"per_task\": {},
        \"post_merge\": {
            \"integration_tests\": []
        },
        \"testing_patterns\": {
            \"framework\": \"cargo test\",
            \"conventions\": \"#[test] functions\"
        }
    }";

    let strategy = parse_test_strategy(json).unwrap();
    assert!(strategy.per_task.is_empty());
    assert!(strategy.post_merge.integration_tests.is_empty());
}

#[test]
fn test_parse_test_strategy_preserves_rationale() {
    let json = r#"{
        "per_task": {
            "T1": {
                "unit_tests": [
                    {
                        "description": "test",
                        "file": "test.rs",
                        "setup": "none",
                        "expected": "passes",
                        "rationale": "catches regression in theme toggle"
                    }
                ],
                "integration_tests": [],
                "edge_case_tests": []
            }
        },
        "post_merge": { "integration_tests": [] },
        "testing_patterns": { "framework": "jest", "conventions": "standard" }
    }"#;
    let strategy = parse_test_strategy(json).unwrap();
    assert_eq!(
        strategy.per_task["T1"].unit_tests[0].rationale,
        "catches regression in theme toggle"
    );
}
