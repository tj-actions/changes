use std::fs;
use std::path::PathBuf;
use std::process::Command;

use git2::{Commit, Delta, Diff, DiffFile, DiffOptions, Oid, Repository, Submodule};
use glob::{MatchOptions, Pattern};

// Utility function to get the version number as a 4-digit integer
pub fn version_number(version: &str) -> u32 {
    let parts: Vec<&str> = version.split('.').collect();
    let mut number: u32 = 0;
    for (i, part) in parts.iter().enumerate() {
        number += part.parse::<u32>().unwrap_or_default() * 1000u32.pow((3 - i) as u32);
    }
    number
}

// Utility function to retrieve the git version
pub fn git_version() -> String {
    println!("Retrieving git version...");
    let git_version_output = Command::new("git").arg("--version").output().unwrap();
    if !git_version_output.status.success() {
        println!("::error::git not installed");
        std::process::exit(1);
    }
    let git_output = String::from_utf8_lossy(&git_version_output.stdout);
    let git_version = git_output.split_whitespace().nth(2).unwrap_or_default().to_string();

    println!("git version: {}", git_version);
    git_version
}

// Utility function to read environment variables
fn get_env_var(name: &str) -> String {
    std::env::var(name).unwrap_or_default()
}

// Utility function to retrieve the required environment variables
pub fn get_env_vars() -> (String, String, String, String, String, String, String, String, String, String, String, bool) {
    let github_workspace: String = get_env_var("GITHUB_WORKSPACE");
    let github_output: String = get_env_var("GITHUB_OUTPUT");
    let github_ref: String = get_env_var("GITHUB_REF");
    let github_event_base_ref: String = get_env_var("GITHUB_EVENT_BASE_REF");
    let github_event_head_repo_fork: String = get_env_var("GITHUB_EVENT_HEAD_REPO_FORK");
    let github_event_pull_request_number: String = get_env_var("GITHUB_EVENT_PULL_REQUEST_NUMBER");
    let github_event_pull_request_base_ref: String = get_env_var("GITHUB_EVENT_PULL_REQUEST_BASE_REF");
    let github_event_pull_request_head_ref: String = get_env_var("GITHUB_EVENT_PULL_REQUEST_HEAD_REF");
    let github_event_pull_request_base_sha: String = get_env_var("GITHUB_EVENT_PULL_REQUEST_BASE_SHA");
    let github_refname: String = get_env_var("GITHUB_REFNAME");
    let github_event_before: String = get_env_var("GITHUB_EVENT_BEFORE");
    let github_event_forced = get_env_var("GITHUB_EVENT_FORCED") == "true";
    (
        github_workspace,
        github_output,
        github_ref,
        github_event_base_ref,
        github_event_head_repo_fork,
        github_event_pull_request_number,
        github_event_pull_request_base_ref,
        github_event_pull_request_head_ref,
        github_event_pull_request_base_sha,
        github_refname,
        github_event_before,
        github_event_forced,
    )
}

// Utility function to retrieve the git repository
pub fn get_repo(path: &PathBuf) -> Repository {
    println!("::debug::Resolving repository path: {}", path.display());
    let repo = match Repository::open(path) {
        Ok(repo) => repo,
        Err(e) => {
            // output the path as a string
            println!("::error::Invalid repository path: {}", path.display());
            panic!("failed to open: {}", e);
        },
    };
    println!("::debug::Repository found: {}", repo.path().display());
    repo
}

fn is_initial_commit(commit: &Commit) -> bool {
    commit.parents().len() == 0
}

pub fn get_previous_and_current_sha_for_push_event(
    extra_args: &str,
    is_tag: &bool,
    is_shallow_clone: &bool,
    github_refname: &str,
    github_event_forced: &bool,
    github_event_before: &str,
    source_branch: &str,
    has_submodules: &bool,
    fetch_depth: &u32,
    until: &str,
    since: &str,
    sha: &str,
    base_sha: &str,
    since_last_remote_commit: &bool,
    repo: &Repository,
) -> (Commit, Commit, bool) {
    let mut target_branch = github_refname.to_owned();
    let current_branch = target_branch.clone();

    let mut current_sha: String = "".to_string();

    println!("Running on a push event...");

    if *is_shallow_clone {
        println!("Fetching remote refs...");
        println!("::debug::extra_args: {}", extra_args);

        let mut cmd = Command::new("git");
        cmd.arg("fetch").arg(&extra_args).arg("-u").arg("--progress").arg(format!("--deepen={}", fetch_depth)).arg("origin");

        if !is_tag {
            cmd.arg(format!("+refs/heads/{}:refs/remotes/origin/{}", current_branch, current_branch));
        } else if !source_branch.is_empty() {
            cmd.arg(format!("+refs/heads/{}:refs/remotes/origin/{}", source_branch, source_branch));
        }
        cmd.stdout(std::process::Stdio::null()).stderr(std::process::Stdio::null());
        cmd.current_dir(&repo.path());
        cmd.spawn().unwrap().wait().unwrap();

        if *has_submodules {
            let mut submodules = repo.submodules().unwrap();
            for submodule in submodules.iter_mut() {
                let mut cmd = Command::new("git");
                cmd.current_dir(submodule.path());
                cmd.arg("fetch").arg(&extra_args).arg("-u").arg("--progress").arg(format!("--deepen={}", fetch_depth));
                cmd.stdout(std::process::Stdio::null()).stderr(std::process::Stdio::null());
                cmd.spawn().unwrap().wait().unwrap();
            }
        }
    }

    println!("::debug::Getting HEAD SHA...");

    if !until.is_empty() {
        println!("::debug::Getting HEAD SHA for '{}'...", until);
        let until_output= Command::new("git")
            .current_dir(&repo.path())
            .arg("log")
            .arg("-1")
            .arg("--format=%H")
            .arg("--date=local")
            .arg(format!("--until={}", until))
            .output()
            .expect("Failed to execute git command");
        current_sha = String::from_utf8_lossy(&until_output.stdout).trim().to_string();
    } else {
        if sha.is_empty() {
            current_sha = repo.revparse_single("HEAD").unwrap().id().to_string();
        } else {
            current_sha = sha.to_string();
        }
    }

    println!("::debug::Verifying the current commit SHA: {}", current_sha);

    let current_commit = match repo.find_commit(Oid::from_str(&current_sha).unwrap()) {
        Ok(commit) => commit,
        Err(_) => {
            println!("::error::The commit {} doesn't exist in the repository. Make sure that the commit SHA is correct.", current_sha);
            std::process::exit(1);
        }
    };

    let mut previous_sha: String = "".to_string();
    let mut initial_commit = false;

    if base_sha.is_empty() {
        if !since.is_empty() {
            println!("::debug::Getting base SHA for '{}'...", since);
            let since_output = Command::new("git")
                .current_dir(&repo.path())
                .arg("log")
                .arg("--format=%H")
                .arg("--date=local")
                .arg(format!("--since={}", since))
                .output()
                .expect("Failed to execute git command");

            previous_sha = String::from_utf8_lossy(&since_output.stdout).to_string();
        } else if *is_tag {
            let git_tag_output = Command::new("git")
                .current_dir(&repo.path())
                .arg("tag")
                .arg("--sort=-v:refname")
                .output()
                .expect("Failed to execute git command");

            let git_tag_output_str = String::from_utf8_lossy(&git_tag_output.stdout);
            let second_latest_tag = git_tag_output_str
                .lines()
                .nth(1)
                .expect("Could not get second latest tag");

            let git_rev_parse_output = Command::new("git")
                .arg("rev-parse")
                .arg(second_latest_tag)
                .output()
                .expect("Failed to execute git command");

            previous_sha = String::from_utf8_lossy(&git_rev_parse_output.stdout).to_string();
        } else {
            // Previous commit from the current HEAD
            previous_sha = current_commit.parent(0).unwrap().id().to_string();

            if *since_last_remote_commit && !*github_event_forced {
                previous_sha = github_event_before.clone().to_string();
            }

            if previous_sha.is_empty() || previous_sha == "0000000000000000000000000000000000000000" {
                previous_sha = String::from_utf8_lossy(current_commit.parent(0).unwrap().id().as_bytes()).to_string();
            }

            if previous_sha == current_sha {
                match repo.find_commit(Oid::from_str(&previous_sha).unwrap()).unwrap().parent(0) {
                    Ok(parent_commit) => {
                        previous_sha = parent_commit.id().to_string();
                    },
                    Err(_) => {
                        initial_commit = true;
                        previous_sha = current_sha.to_string();
                        println!("::warning::Initial commit detected no previous commit found.");
                    }
                }

            } else {
                if previous_sha.is_empty() {
                    println!("::error::Unable to locate a previous commit.");
                    std::process::exit(1);
                }
            }
        }
    } else {
        previous_sha = base_sha.to_string();
        if *is_tag {
            let target_branch_output = Command::new("git")
                .current_dir(&repo.path())
                .arg("describe")
                .arg("--tags")
                .arg(&previous_sha)
                .output()
                .expect("Failed to execute git command");

            target_branch = String::from_utf8_lossy(&target_branch_output.stdout).to_string();
        }
    }

    println!("::debug::Target branch {}...", target_branch);
    println!("::debug::Current branch {}...", current_branch);

    println!("::debug::Verifying the previous commit SHA: {}", previous_sha);

    if repo.find_commit(Oid::from_str(&previous_sha).unwrap()).is_err() {
        println!("::error::The commit {} doesn't exist in the repository. Make sure that the commit SHA is correct.", previous_sha);
        std::process::exit(1);
    }

    let previous_commit = repo.find_commit(Oid::from_str(&previous_sha).unwrap()).unwrap();

    if previous_sha == current_sha && !initial_commit {
        println!("::error::Similar commit hashes detected: previous sha: {} is equivalent to the current sha: {}.", previous_sha, current_sha);
        println!("::error::Please verify that both commits are valid, and increase the fetch_depth to a number higher than {}.", fetch_depth);
        std::process::exit(1);
    }

    (
        previous_commit,
        current_commit,
        initial_commit,
    )
}

pub fn get_previous_and_current_sha_for_pull_request_event(
    extra_args: &str,
    github_event_before: &str,
    github_event_pull_request_base_ref: &str,
    github_event_pull_request_head_ref: &str,
    github_event_head_repo_fork: &str,
    github_event_pull_request_number: &str,
    github_event_pull_request_base_sha: &str,
    has_submodules: &bool,
    fetch_depth: &u32,
    is_shallow_clone: &bool,
    until: &str,
    sha: &str,
    base_sha: &str,
    since_last_remote_commit: &bool,
    repo: &Repository,
) -> (Commit, Commit, String) {
    let mut target_branch = github_event_pull_request_base_ref.to_string();
    let current_branch = github_event_pull_request_head_ref.to_string();

    let mut current_sha: String = "".to_string();

    println!("Running on a pull request event...");

    if *since_last_remote_commit {
        target_branch = current_branch.clone();
    }

    if *is_shallow_clone {
        println!("Fetching remote refs...");
        println!("::debug::extra_args: {}", extra_args);

        let mut cmd = Command::new("git");
        cmd.arg("fetch").arg(&extra_args).arg("-u").arg("--progress").arg("origin").arg(format!("pull/{}/head:{}", &github_event_pull_request_number, current_branch));
        cmd.stdout(std::process::Stdio::null()).stderr(std::process::Stdio::null());
        cmd.spawn().unwrap().wait().unwrap();

        // Check if the exit code is 0, if not, try to fetch the branch
        if cmd.status().unwrap().code().unwrap() != 0 {
            println!("First fetch failed, falling back to second fetch");
            let mut cmd = Command::new("git");
            cmd.arg("fetch").arg(&extra_args).arg("-u").arg("--progress").arg(format!("--deepen={}", &fetch_depth)).arg("origin").arg(format!("+refs/heads/{}*:refs/remotes/origin/{}*", current_branch, current_branch));
            cmd.stdout(std::process::Stdio::null()).stderr(std::process::Stdio::null());
            cmd.spawn().unwrap().wait().unwrap();
        } else {
            println!("First fetch succeeded");
        }

        if *since_last_remote_commit {
            println!("::debug::Fetching remote target branch...");
            let mut cmd = Command::new("git");
            cmd.arg("fetch").arg(&extra_args).arg("-u").arg("--progress").arg(format!("--deepen={}", fetch_depth)).arg("origin").arg(format!("+refs/heads/{}:refs/remotes/origin/{}", target_branch, target_branch));
            cmd.stdout(std::process::Stdio::null()).stderr(std::process::Stdio::null());
            cmd.spawn().unwrap().wait().unwrap();

            let mut cmd = Command::new("git");
            cmd.arg("branch").arg("--track").arg(&target_branch).arg(format!("origin/{}", target_branch));
            cmd.stdout(std::process::Stdio::null()).stderr(std::process::Stdio::null());
            cmd.spawn().unwrap().wait().unwrap();
        }

        if *has_submodules {
            let mut submodules = repo.submodules().unwrap();
            for submodule in submodules.iter_mut() {
                let mut cmd = Command::new("git");
                cmd.current_dir(submodule.path());
                cmd.arg("fetch").arg(&extra_args).arg("-u").arg("--progress").arg(format!("--deepen={}", fetch_depth));
                cmd.stdout(std::process::Stdio::null()).stderr(std::process::Stdio::null());
                cmd.spawn().unwrap().wait().unwrap();
            }
        }
    }

    println!("::debug::Getting HEAD SHA...");

    if !until.is_empty() {
        println!("::debug::Getting HEAD SHA for '{}'...", until);
        let current_sha_output = Command::new("git")
            .arg("log")
            .arg("-1")
            .arg("--format=%H")
            .arg("--date=local")
            .arg(format!("--until={}", until))
            .output()
            .expect(format!("::error::Invalid until date: {}", until).as_str());

        current_sha = String::from_utf8(current_sha_output.stdout).unwrap().to_string();
    } else {
        if sha.is_empty() {
            let current_sha_output = Command::new("git")
                .arg("rev-list")
                .arg("-n")
                .arg("1")
                .arg("HEAD")
                .output()
                .expect("::error::Unable to locate the current sha");

            current_sha = String::from_utf8(current_sha_output.stdout).unwrap().to_string();
        } else {
            current_sha = sha.to_string();
        }
    }

    println!("::debug::Verifying the current commit SHA: {}", current_sha);

    let current_commit = match repo.find_commit(Oid::from_str(&current_sha).unwrap()) {
        Ok(commit) => commit,
        Err(_) => {
            println!("::error::Unable to locate the current sha: {}", current_sha);
            println!("::error::Please verify that the current sha is valid. and increase the fetch_depth to a number higher than {}", fetch_depth);
            std::process::exit(1);
        }
    };

    println!("::debug::Current SHA: {}", current_sha);

    let mut previous_sha: String = "".to_string();
    let mut diff = "...";

    if github_event_pull_request_base_ref.is_empty() || github_event_head_repo_fork == "true" {
        diff = "..";
    }

    if base_sha.is_empty() {
        if since_last_remote_commit {
            previous_sha = github_event_before.to_string();

            if !repo.find_commit(Oid::from_str(&previous_sha).unwrap()).is_ok() {
                previous_sha = github_event_pull_request_base_sha.to_string();
            }
        } else {
            let mut previous_sha_output = Command::new("git")
                .arg("rev-parse")
                .arg(format!("origin/{}", target_branch))
                .output()
                .expect("::error::Unable to locate the previous sha");

            previous_sha = String::from_utf8(previous_sha_output.stdout).unwrap().to_string();

            if *is_shallow_clone {
                // Check if the merge base is in the local history
                if match repo.merge_base(
                    Oid::from_str(&previous_sha).unwrap(),
                    current_commit.id()
                ) {
                    Ok(_) => true,
                    Err(_) => false,
                } {
                    println!("::debug::Merge base is in the local history");
                } else {
                    println!("::debug::Merge base is not in the local history, fetching remote target branch...");

                    // Fetch more of the target branch history until the merge base is found
                    for i in 1..10 {
                        Command::new("git")
                            .arg("fetch")
                            .arg("-u")
                            .arg("--progress")
                            .arg(format!("--deepen={}", fetch_depth))
                            .arg("origin")
                            .arg(format!("+refs/heads/{}:refs/remotes/origin/{}", target_branch, target_branch))
                            .output()
                            .expect("::error::Unable to fetch remote target branch");

                        if match repo.merge_base(
                            Oid::from_str(&previous_sha).unwrap(),
                            current_commit.id()
                        ) {
                            Ok(_) => true,
                            Err(_) => false,
                        } {
                            break;
                        }

                        println!("::debug::Merge base is not in the local history, fetching remote target branch again...");
                        println!("::debug::Attempt {}/10", i);
                    }
                }
            }
        }

        if previous_sha.is_empty() || previous_sha == current_sha {
            previous_sha = github_event_pull_request_base_sha.to_string();
        }

        println!("::debug::Previous SHA: {}", previous_sha);
    } else {
        previous_sha = base_sha.to_string();
    }

    // Check if the merge base is in the local history if not set diff to ..
    if match repo.merge_base(
        Oid::from_str(&previous_sha).unwrap(),
        current_commit.id()
    ) {
        Ok(_) => true,
        Err(_) => false,
    } {
        println!("::debug::Merge base is in the local history");
    } else {
        println!("::debug::Merge base is not in the local history, setting diff to ..");
        diff = "..";
    }

    println!("::debug::Target branch: {}", target_branch);
    println!("::debug::Current branch: {}", current_branch);

    println!("::debug::Verifying the previous commit SHA: {}", previous_sha);
    let previous_commit = match repo.find_commit(Oid::from_str(&previous_sha).unwrap()) {
        Ok(commit) => commit,
        Err(_) => {
            println!("::error::Unable to locate the previous sha: {}", previous_sha);
            println!("::error::Please verify that the previous sha is valid, and increase the fetch_depth to a number higher than {}", fetch_depth);
            std::process::exit(1);
        }
    };

    println!("::debug::Verifying the difference between {}{}{}", previous_sha, diff, current_sha);

    let ancestor_commit = match diff {
        ".." => &previous_commit,
        "..." => repo.merge_base(previous_commit.id(), current_commit.id()).unwrap(),
        _ => panic!("Invalid diff operator: {}", diff),
    };

    let mut diff_options = DiffOptions::new();
    diff_options.ignore_submodules(true);

    let diff_of_commits = repo.diff_tree_to_tree(Some(&ancestor_commit.tree().unwrap()), Some(&current_commit.tree().unwrap()), Some(&mut diff_options)).unwrap();

    if diff_of_commits.deltas().count() == 0 {
        println!("::error::Unable to determine a difference between {}{}{}", previous_sha, diff, current_sha);
        std::process::exit(1);
    }

    if previous_sha == current_sha {
        println!("::error::Similar commit hashes detected: previous sha: {} is equivalent to the current sha: {}.", previous_sha, current_sha);
        println!("::error::Please verify that both commits are valid, and increase the fetch_depth to a number higher than {}.", fetch_depth);
        std::process::exit(1);
    }

    (
        previous_commit,
        current_commit,
        diff.to_string(),
    )
}

#[derive(Debug, PartialEq)]
pub enum DiffType {
    Added,
    Copied,
    Modified,
    Deleted,
    Renamed,
    TypeChanged,
    Unmerged,
    Unknown,
}

impl From<Delta> for DiffType {
    fn from(delta: Delta) -> Self {
        match delta.status() {
            Delta::Added => DiffType::Added,
            Delta::Copied => DiffType::Copied,
            Delta::Deleted => DiffType::Deleted,
            Delta::Modified => DiffType::Modified,
            Delta::Renamed => DiffType::Renamed,
            Delta::Typechange => DiffType::TypeChanged,
            Delta::Untracked => DiffType::Added,
            Delta::Ignored => DiffType::Added,
            Delta::Unreadable => DiffType::Added,
            Delta::Conflicted => DiffType::Unmerged,
        }
    }
}

pub fn get_diff(
    repo: &Repository,
    previous_commit: &Commit,
    current_commit: &Commit,
    diff_types: &[DiffType],
    diff: &str,
    glob_patterns: &Vec<Pattern>,
) -> Diff {
    let ancestor_commit = match diff {
        ".." => previous_commit,
        "..." => repo.merge_base(previous_commit.id(), current_commit.id()).unwrap(),
        _ => panic!("Invalid diff operator: {}", diff),
    };

    let mut diff_options = DiffOptions::new();
    diff_options.ignore_submodules(true);

    let diff_of_commits = repo.diff_tree_to_tree(Some(&ancestor_commit.tree().unwrap()), Some(&current_commit.tree().unwrap()), Some(&mut diff_options)).unwrap();

    let mut file_diff = Diff::new();

    for delta in diff_of_commits.deltas() {
        let delta_type = match delta.status() {
            Delta::Added => DiffType::Added,
            Delta::Copied => DiffType::Copied,
            Delta::Deleted => DiffType::Deleted,
            Delta::Modified => DiffType::Modified,
            Delta::Renamed => DiffType::Renamed,
            Delta::Typechange => DiffType::TypeChanged,
            Delta::Unmodified => DiffType::Unknown,
            Delta::Unreadable => DiffType::Unknown,
            Delta::Untracked => DiffType::Unknown,
            Delta::Ignored => DiffType::Unknown,
            Delta::Conflicted => DiffType::Unmerged,
        };

        if diff_types.contains(&delta_type) {
            let path = delta.new_file().path().unwrap().to_str().unwrap().to_string();

            if glob_patterns.is_empty() || glob_patterns.iter().any(|pattern| pattern.matches(&path)) {
                let mut diff_file = DiffFile::new();
                diff_file.path = path;
                diff_file.diff_type = delta_type;
                file_diff.files.push(diff_file);
            }
        }
    }

    for submodule in repo.submodules().unwrap() {
        let submodule_diff = get_submodule_diff(
            &repo,
            &submodule,
            &previous_commit,
            &current_commit,
            &diff_types,
            &diff,
            &glob_patterns,
        );

        if !submodule_diff.files.is_empty() {
            file_diff.push(submodule_diff);
        }
    }

    file_diff
}

fn get_submodule_diff(
    repo: &Repository,
    submodule: &Submodule,
    parent_previous_commit: &Commit,
    parent_current_commit: &Commit,
    diff_types: &[DiffType],
    diff: &str,
    glob_patterns: &Vec<Pattern>,
) -> Diff {
    let submodule_path = submodule.path().unwrap().to_str().unwrap();

    let submodule_previous_commit = repo.find_commit(parent_previous_commit.tree().unwrap().get_path(submodule_path).unwrap().id()).unwrap();
    let submodule_current_commit = repo.find_commit(parent_current_commit.tree().unwrap().get_path(submodule_path).unwrap().id()).unwrap();

    let submodule_ancestor_commit = match diff {
        ".." => &submodule_previous_commit,
        "..." => repo.merge_base(submodule_previous_commit.id(), submodule_current_commit.id()).unwrap(),
        _ => panic!("Invalid diff operator: {}", diff),
    };

    let mut diff_options = DiffOptions::new();
    diff_options.ignore_submodules(true);

    let submodule_diff = repo.diff_tree_to_tree(Some(&submodule_ancestor_commit.tree().unwrap()), Some(&submodule_current_commit.tree().unwrap()), Some(&mut diff_options)).unwrap();

    let mut file_diff = Diff::new();

    for delta in submodule_diff.deltas() {
        let delta_type = match delta.status() {
            Delta::Added => DiffType::Added,
            Delta::Copied => DiffType::Copied,
            Delta::Deleted => DiffType::Deleted,
            Delta::Modified => DiffType::Modified,
            Delta::Renamed => DiffType::Renamed,
            Delta::Typechange => DiffType::TypeChanged,
            Delta::Unmodified => DiffType::Unknown,
            Delta::Unreadable => DiffType::Unknown,
            Delta::Untracked => DiffType::Unknown,
            Delta::Ignored => DiffType::Unknown,
            Delta::Conflicted => DiffType::Unmerged,
        };

        if diff_types.contains(&delta_type) {
            let path = delta.new_file().path().unwrap().to_str().unwrap().to_string();

            if glob_patterns.is_empty() || glob_patterns.iter().any(|pattern| pattern.matches(&path)) {
                let mut diff_file = DiffFile::new();
                diff_file.path = path;
                diff_file.diff_type = delta_type;
                file_diff.files.push(diff_file);
            }
        }
    }

    file_diff
}

pub fn get_glob_patterns(
    files: &str,
    files_separator: &str,
    files_from_source_file: &str,
    files_from_source_file_separator: &str,
    files_ignore: &str,
    files_ignore_separator: &str,
    files_ignore_from_source_file: &str,
    files_ignore_from_source_file_separator: &str,
    path: &str,
) -> Vec<Pattern> {
    let mut glob_patterns: Vec<Pattern> = Vec::new();

    if !files.is_empty() {
        for file in files.split(files_separator) {
            let glob_pattern = match Pattern::new(file) {
                Ok(glob_pattern) => glob_pattern,
                Err(_) => println!("::warning::Invalid glob pattern: {}", file),
            };
            glob_patterns.push(glob_pattern);
        }
    }

    if !files_from_source_file.is_empty() {
        for source_file in files_from_source_file.split(files_from_source_file_separator) {
            let mut file_path = PathBuf::from(path);
            file_path.push(source_file);

            let file_contents = match fs::read_to_string(file_path) {
                Ok(file_contents) => file_contents,
                Err(_) => println!("::warning::Could not read file: {}", file_path.to_str().unwrap()),
            };

            for file in file_contents.split("\n") {
                let glob_pattern = match Pattern::new(file) {
                    Ok(glob_pattern) => glob_pattern,
                    Err(_) => println!("::warning::Invalid glob pattern: {}", file),
                };
                glob_patterns.push(glob_pattern);
            }
        }
    }

    let mut glob_ignore_patterns: Vec<Pattern> = Vec::new();

    if !files_ignore.is_empty() {
        for file in files_ignore.split(files_ignore_separator) {
            let glob_pattern = match Pattern::new(file) {
                Ok(glob_pattern) => glob_pattern,
                Err(_) => println!("::warning::Invalid ignore glob pattern: {}", file),
            };
            glob_ignore_patterns.push(glob_pattern);
        }
    }

    if !files_ignore_from_source_file.is_empty() {
        for source_file in files_ignore_from_source_file.split(files_ignore_from_source_file_separator) {
            let mut file_path = PathBuf::from(path);
            file_path.push(source_file);

            let file_contents = match fs::read_to_string(file_path) {
                Ok(file_contents) => file_contents,
                Err(_) => println!("::warning::Could not read file: {}", file_path.to_str().unwrap()),
            };

            for file in file_contents.split("\n") {
                let glob_pattern = match Pattern::new(file) {
                    Ok(glob_pattern) => glob_pattern,
                    Err(_) => println!("::warning::Invalid ignore glob pattern: {}", file),
                };
                glob_ignore_patterns.push(glob_pattern);
            }
        }
    }

    let mut match_options = MatchOptions::new();
    match_options.case_sensitive = false;

    let mut match_options = MatchOptions::new();
    match_options.case_sensitive = false;

    let non_ignored_glob_patterns: Vec<Pattern> = glob_patterns.into_iter().filter(|glob_pattern| !glob_ignore_patterns.iter().any(|ignore_glob_pattern| ignore_glob_pattern.matches_with(&glob_pattern.as_str(), match_options))).collect();

    non_ignored_glob_patterns
}
