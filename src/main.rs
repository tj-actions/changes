mod args;
mod utils;

use clap::Parser;

use git2::{Commit, Config, Repository};
use json2file::{writer};

use crate::args::Args;
use crate::utils::DiffType;

fn main() {
    let args: Args = args::Args::parse();
    let git_version = utils::git_version();

    println!("::group::changed-files-diff-sha");

    if utils::version_number(&git_version) < utils::version_number("2.18.0") {
        println!(
            "::error::Invalid git version. Please upgrade ({}) to >= (2.18.0)",
            git_version
        );
        std::process::exit(1);
    } else {
        println!("Valid git version found: ({})", git_version);
    }

    let (
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
        github_event_forced
    ) = utils::get_env_vars();

    // join the workspace path with the args.path
    let path = std::path::Path::new(&github_workspace).join(&args.path);
    let repo = utils::get_repo(&path);

    let mut config = Config::open_default().unwrap();

    let quotepath_value = if args.quotepath == "false" { "off" } else { "on" };
    println!("::debug::quotepath: {}", quotepath_value);
    config.set_str("core.quotepath", quotepath_value).unwrap();

    if !args.diff_relative.is_empty() {
        println!("::debug::diff_relative: true");
        config.set_str("diff.relative", &args.diff_relative).unwrap();
    }

    let submodules = repo.submodules().unwrap();
    let has_submodules = submodules.len() > 0;

    let is_shallow_clone = repo.is_shallow();
    println!("::debug::is_shallow_clone: {}", is_shallow_clone);

    let mut current_commit: git2::Commit = Commit::default();
    let mut previous_commit: git2::Commit = Commit::default();
    let mut diff : String = "..".to_string();
    let mut is_tag = false;
    let mut extra_args = "--no-tags --prune --recurse-submodules";
    let mut source_branch = String::new();
    let mut initial_commit = false;

    if github_ref.starts_with("refs/tags/") {
        is_tag = true;
        extra_args = "--prune --no-recurse-submodules";
        source_branch = github_event_base_ref.replace("refs/heads/", "");

        println!("::debug::is_tag: {}", is_tag);
        println!("::debug::source_branch: {}", source_branch);
    }

    println!("::debug::extra_args: {}", extra_args);

    if github_event_pull_request_base_ref.is_empty() {
        (
            current_commit,
            previous_commit,
            initial_commit,
        ) = utils::get_previous_and_current_sha_for_push_event(
            &extra_args,
            &is_tag,
            &is_shallow_clone,
            &github_refname,
            &github_event_forced,
            &github_event_before,
            &source_branch,
            &has_submodules,
            &args.fetch_depth,
            &args.until,
            &args.since,
            &args.sha,
            &args.base_sha,
            &args.since_last_remote_commit,
            &repo,
        );

        if initial_commit {
            println!("Initial commit detected, skipping...");
            std::process::exit(0);
        }
    } else {
        (
            current_commit,
            previous_commit,
            diff
        ) = utils::get_previous_and_current_sha_for_pull_request_event(
            &extra_args,
            &github_event_before,
            &github_event_pull_request_base_ref,
            &github_event_pull_request_head_ref,
            &github_event_head_repo_fork,
            &github_event_pull_request_number,
            &github_event_pull_request_base_sha,
            &has_submodules,
            &args.fetch_depth,
            &is_shallow_clone,
            &args.since,
            &args.sha,
            &args.base_sha,
            &args.since_last_remote_commit,
            &repo,
        );
    }

    let glob_patterns = utils::get_glob_patterns(
        &args.files,
        &args.files_separator,
        &args.files_from_source_file,
        &args.files_from_source_file_separator,
        &args.files_ignore,
        &args.files_ignore_separator,
        &args.files_ignore_from_source_file,
        &args.files_ignore_from_source_file_separator,
        &args.path,
    );

    let added_files = utils::get_diff(
        &repo,
        &previous_commit,
        &current_commit,
        &[DiffType::Added],
        &diff,
        &glob_patterns,
    );

    let copied_files = utils::get_diff(
        &repo,
        &previous_commit,
        &current_commit,
        &[DiffType::Copied],
        &diff,
        &glob_patterns,
    );

    let deleted_files = utils::get_diff(
        &repo,
        &previous_commit,
        &current_commit,
        &[DiffType::Deleted],
        &diff,
        &glob_patterns,
    );

    let modified_files = utils::get_diff(
        &repo,
        &previous_commit,
        &current_commit,
        &[DiffType::Modified],
        &diff,
        &glob_patterns,
    );

    let renamed_files = utils::get_diff(
        &repo,
        &previous_commit,
        &current_commit,
        &[DiffType::Renamed],
        &diff,
        &glob_patterns,
    );

    let type_changed_files = utils::get_diff(
        &repo,
        &previous_commit,
        &current_commit,
        &[DiffType::TypeChanged],
        &diff,
        &glob_patterns,
    );

    let unmerged_files = utils::get_diff(
        &repo,
        &previous_commit,
        &current_commit,
        &[DiffType::Unmerged],
        &diff,
        &glob_patterns,
    );

    let unknown_files = utils::get_diff(
        &repo,
        &previous_commit,
        &current_commit,
        &[DiffType::Unknown],
        &diff,
        &glob_patterns,
    );

    let all_changed_and_modified_files = utils::get_diff(
        &repo,
        &previous_commit,
        &current_commit,
        &[
            DiffType::Added,
            DiffType::Copied,
            DiffType::Deleted,
            DiffType::Modified,
            DiffType::Renamed,
            DiffType::TypeChanged,
            DiffType::Unmerged,
            DiffType::Unknown
        ],
        &diff,
        &glob_patterns,
    );



    // writer::write_outputs(
    //     &args.skip_missing_keys,
    //     &keys,
    //     &args.outputs,
    //     &output_directory,
    //     &args.extension,
    //     &args.verbose,
    // );

    println!("::endgroup::");
}
