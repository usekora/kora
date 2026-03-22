use kora::git::worktree::{MergeResult, WorktreeManager};
use tempfile::TempDir;

async fn setup_git_repo() -> TempDir {
    let tmp = TempDir::new().unwrap();
    let path = tmp.path();

    let output = tokio::process::Command::new("git")
        .current_dir(path)
        .args(["init"])
        .output()
        .await
        .unwrap();
    assert!(output.status.success(), "git init failed");

    let output = tokio::process::Command::new("git")
        .current_dir(path)
        .args(["config", "user.email", "test@test.com"])
        .output()
        .await
        .unwrap();
    assert!(output.status.success());

    let output = tokio::process::Command::new("git")
        .current_dir(path)
        .args(["config", "user.name", "Test"])
        .output()
        .await
        .unwrap();
    assert!(output.status.success());

    std::fs::write(path.join("README.md"), "initial").unwrap();

    let output = tokio::process::Command::new("git")
        .current_dir(path)
        .args(["add", "."])
        .output()
        .await
        .unwrap();
    assert!(output.status.success());

    let output = tokio::process::Command::new("git")
        .current_dir(path)
        .args(["commit", "-m", "initial commit"])
        .output()
        .await
        .unwrap();
    assert!(output.status.success(), "git commit failed: {:?}", String::from_utf8_lossy(&output.stderr));

    tmp
}

#[tokio::test]
async fn test_create_and_remove_worktree() {
    let tmp = setup_git_repo().await;
    let manager = WorktreeManager::new(tmp.path());

    let wt_path = manager.create_worktree("create1", "kora/create1").await.unwrap();
    assert!(wt_path.exists());
    assert!(wt_path.join("README.md").exists());

    manager.remove_worktree(&wt_path).await.unwrap();
    assert!(!wt_path.exists());
}

#[tokio::test]
async fn test_list_worktrees_includes_created() {
    let tmp = setup_git_repo().await;
    let manager = WorktreeManager::new(tmp.path());

    let wt_path = manager.create_worktree("list1", "kora/list1").await.unwrap();

    let worktrees = manager.list_worktrees().await.unwrap();
    let kora_worktrees: Vec<_> = worktrees.iter().filter(|w| w.task_id.is_some()).collect();
    assert_eq!(kora_worktrees.len(), 1);
    assert_eq!(kora_worktrees[0].task_id.as_deref(), Some("list1"));

    manager.remove_worktree(&wt_path).await.unwrap();
}

#[tokio::test]
async fn test_cleanup_all_removes_kora_worktrees() {
    let tmp = setup_git_repo().await;
    let manager = WorktreeManager::new(tmp.path());

    manager.create_worktree("clean1", "kora/clean1").await.unwrap();
    manager.create_worktree("clean2", "kora/clean2").await.unwrap();

    let removed = manager.cleanup_all().await.unwrap();
    assert_eq!(removed, 2);

    let worktrees = manager.list_worktrees().await.unwrap();
    let kora_worktrees: Vec<_> = worktrees.iter().filter(|w| w.task_id.is_some()).collect();
    assert_eq!(kora_worktrees.len(), 0);
}

#[tokio::test]
async fn test_current_branch() {
    let tmp = setup_git_repo().await;
    let manager = WorktreeManager::new(tmp.path());

    let branch = manager.current_branch().await.unwrap();
    assert!(!branch.is_empty());
}

#[tokio::test]
async fn test_merge_branch_clean() {
    let tmp = setup_git_repo().await;
    let manager = WorktreeManager::new(tmp.path());

    let wt_path = manager.create_worktree("merge1", "kora/merge1").await.unwrap();

    std::fs::write(wt_path.join("new_file.txt"), "content").unwrap();
    let output = tokio::process::Command::new("git")
        .current_dir(&wt_path)
        .args(["add", "."])
        .output()
        .await
        .unwrap();
    assert!(output.status.success());

    let output = tokio::process::Command::new("git")
        .current_dir(&wt_path)
        .args(["commit", "-m", "add new file"])
        .output()
        .await
        .unwrap();
    assert!(output.status.success());

    let result = manager.merge_branch(tmp.path(), "kora/merge1").await.unwrap();
    assert_eq!(result, MergeResult::Success);

    assert!(tmp.path().join("new_file.txt").exists());

    manager.remove_worktree(&wt_path).await.unwrap();
}

#[tokio::test]
async fn test_merge_branch_conflict() {
    let tmp = setup_git_repo().await;
    let manager = WorktreeManager::new(tmp.path());

    let wt_path = manager.create_worktree("conflict1", "kora/conflict1").await.unwrap();

    std::fs::write(wt_path.join("README.md"), "branch content").unwrap();
    let output = tokio::process::Command::new("git")
        .current_dir(&wt_path)
        .args(["add", "."])
        .output()
        .await
        .unwrap();
    assert!(output.status.success());

    let output = tokio::process::Command::new("git")
        .current_dir(&wt_path)
        .args(["commit", "-m", "modify readme"])
        .output()
        .await
        .unwrap();
    assert!(output.status.success());

    std::fs::write(tmp.path().join("README.md"), "main content").unwrap();
    let output = tokio::process::Command::new("git")
        .current_dir(tmp.path())
        .args(["add", "."])
        .output()
        .await
        .unwrap();
    assert!(output.status.success());

    let output = tokio::process::Command::new("git")
        .current_dir(tmp.path())
        .args(["commit", "-m", "modify readme on main"])
        .output()
        .await
        .unwrap();
    assert!(output.status.success());

    let result = manager.merge_branch(tmp.path(), "kora/conflict1").await.unwrap();
    assert!(matches!(result, MergeResult::Conflict { .. }));

    manager.remove_worktree(&wt_path).await.unwrap();
}
