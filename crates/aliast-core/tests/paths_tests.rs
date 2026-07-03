use aliast_core::predict::paths::{complete, is_eligible};

/// Create dirs/files under a fresh tempdir; returns the tempdir guard.
fn fixture(dirs: &[&str], files: &[&str]) -> tempfile::TempDir {
    let tmp = tempfile::tempdir().unwrap();
    for dir in dirs {
        std::fs::create_dir_all(tmp.path().join(dir)).unwrap();
    }
    for file in files {
        std::fs::write(tmp.path().join(file), b"x").unwrap();
    }
    tmp
}

#[test]
fn eligibility_requires_allowlisted_command_and_space() {
    assert!(is_eligible("cd "));
    assert!(is_eligible("cd cr"));
    assert!(is_eligible("ls src"));
    assert!(is_eligible("pushd x"));
    assert!(is_eligible("mkdir new"));
    assert!(is_eligible("rmdir old"));
    assert!(!is_eligible("cd")); // no space yet
    assert!(!is_eligible("cat x")); // not allowlisted
    assert!(!is_eligible("cd x && ls")); // separators
    assert!(!is_eligible("cd \"My")); // quotes
}

#[test]
fn completes_subdirectory_by_prefix_alphabetically() {
    let tmp = fixture(&["crates", "plugin", "packaging"], &[]);
    let cwd = tmp.path().to_str().unwrap();

    let got = complete("cd p", Some(cwd), &[], 8);
    assert_eq!(got, vec![format!("cd packaging/"), format!("cd plugin/")]);
}

#[test]
fn empty_partial_lists_all_subdirectories() {
    let tmp = fixture(&["b_dir", "a_dir"], &[]);
    let got = complete("cd ", Some(tmp.path().to_str().unwrap()), &[], 8);
    assert_eq!(got, vec!["cd a_dir/", "cd b_dir/"]);
}

#[test]
fn files_are_not_suggested_directories_only() {
    let tmp = fixture(&["src"], &["setup.py", "script.sh"]);
    let got = complete("cd s", Some(tmp.path().to_str().unwrap()), &[], 8);
    assert_eq!(got, vec!["cd src/"]);
}

#[test]
fn nested_parent_is_scanned() {
    let tmp = fixture(&["crates/aliast-core", "crates/aliast-daemon"], &[]);
    let got = complete(
        "ls crates/aliast-c",
        Some(tmp.path().to_str().unwrap()),
        &[],
        8,
    );
    assert_eq!(got, vec!["ls crates/aliast-core/"]);
}

#[test]
fn hidden_dirs_excluded_unless_prefix_is_dotted() {
    let tmp = fixture(&[".git", ".github", "src"], &[]);
    let cwd = tmp.path().to_str().unwrap();

    let all = complete("cd ", Some(cwd), &[], 8);
    assert_eq!(all, vec!["cd src/"]);

    let dotted = complete("cd .g", Some(cwd), &[], 8);
    assert_eq!(dotted, vec!["cd .git/", "cd .github/"]);
}

#[test]
fn tilde_prefix_expands_against_home() {
    let tmp = fixture(&["projects"], &[]);
    // Point HOME at the fixture for this test via explicit env override.
    // SAFETY: test process; no other thread reads HOME concurrently here.
    unsafe { std::env::set_var("HOME", tmp.path()) };
    let got = complete("cd ~/pro", None, &[], 8);
    assert_eq!(got, vec!["cd ~/projects/"]);
}

#[test]
fn relative_path_without_cwd_yields_nothing() {
    assert!(complete("cd pro", None, &[], 8).is_empty());
}

#[test]
fn missing_parent_dir_yields_nothing_not_error() {
    let tmp = fixture(&[], &[]);
    let got = complete("cd nope/x", Some(tmp.path().to_str().unwrap()), &[], 8);
    assert!(got.is_empty());
}

#[test]
fn flag_token_is_not_path_completed() {
    let tmp = fixture(&["src"], &[]);
    assert!(complete("ls -l", Some(tmp.path().to_str().unwrap()), &[], 8).is_empty());
}

#[test]
fn exact_dir_name_still_gets_trailing_slash() {
    let tmp = fixture(&["src"], &[]);
    let got = complete("cd src", Some(tmp.path().to_str().unwrap()), &[], 8);
    assert_eq!(got, vec!["cd src/"]);
}
