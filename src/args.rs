use clap::{Parser, ValueEnum};
use std::fmt;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
pub struct Args {
    /// Split character for output strings
    #[clap(short, long, default_value = " ")]
    pub separator: String,

    /// Include `all_old_new_renamed_files` output. Note this can generate a large output
    #[clap(short, long, default_value = "false")]
    pub include_all_old_new_renamed_files: bool,

    /// Split character for old and new filename pairs.
    #[clap(short, long, default_value = ",")]
    pub old_new_separator: String,

    /// Split character for old and new renamed filename pairs.
    #[clap(short, long, default_value = " ")]
    pub old_new_files_separator: String,

    /// File and directory patterns to detect changes using only these list of file(s) (Defaults to the entire repo) **NOTE:** Multiline file/directory patterns should not include quotes.
    #[clap(short, long)]
    pub files: String,

    /// Separator used to split the `files` input
    #[clap(short, long, default_value = "\n")]
    pub files_separator: String,

    /// Source file(s) used to populate the `files` input.
    #[clap(short, long)]
    pub files_from_source_file: String,

    /// Separator used to split the `files_from_source_file` input
    #[clap(short, long, default_value = "\n")]
    pub files_from_source_file_separator: String,

    /// Ignore changes to these file(s) **NOTE:** Multiline file/directory patterns should not include quotes.
    #[clap(short, long)]
    pub files_ignore: String,

    /// Separator used to split the `files_ignore` input
    #[clap(short, long, default_value = "\n")]
    pub files_ignore_separator: String,

    /// Source file(s) used to populate the `files_ignore` input
    #[clap(short, long)]
    pub files_ignore_from_source_file: String,

    /// Separator used to split the `files_ignore_from_source_file` input
    #[clap(short, long, default_value = "\n")]
    pub files_ignore_from_source_file_separator: String,

    /// Specify a different commit SHA used for comparing changes
    #[clap(short, long)]
    pub sha: String,

    /// Specify a different base commit SHA used for comparing changes
    #[clap(short, long)]
    pub base_sha: String,

    /// Get changed files for commits whose timestamp is older than the given time.
    #[clap(short, long)]
    pub since: String,

    /// Get changed files for commits whose timestamp is earlier than the given time.
    #[clap(short, long)]
    pub until: String,

    /// Specify a relative path under `$GITHUB_WORKSPACE` to locate the repository.
    #[clap(short, long, default_value = ".")]
    pub path: String,

    /// Use non ascii characters to match files and output the filenames completely verbatim by setting this to `false`
    #[clap(short, long, default_value = "true")]
    pub quotepath: String,

    /// Exclude changes outside the current directory and show path names relative to it. **NOTE:** This requires you to specify the top level directory via the `path` input.
    #[clap(short, long)]
    pub diff_relative: String,

    /// Output unique changed directories instead of filenames. **NOTE:** This returns `.` for changed files located in the root of the project.
    #[clap(short, long, default_value = "false")]
    pub dir_names: bool,

    /// Maximum depth of directories to output. e.g `test/test1/test2` with max depth of `2` returns `test/test1`.
    #[clap(short, long)]
    pub dir_names_max_depth: String,

    /// Exclude the root directory represented by `.` from the output when `dir_names`is set to `true`.
    #[clap(short, long, default_value = "false")]
    pub dir_names_exclude_root: bool,

    /// Output list of changed files in a JSON formatted string which can be used for matrix jobs.
    #[clap(short, long, default_value = "false")]
    pub json: bool,

    /// Output list of changed files in [jq](https://devdocs.io/jq/) raw output format which means that the output will not be surrounded by quotes and special characters will not be escaped.
    #[clap(short, long, default_value = "false")]
    pub json_raw_format: bool,

    /// Depth of additional branch history fetched. **NOTE**: This can be adjusted to resolve errors with insufficient history.
    #[clap(short, long, default_value = "50")]
    pub fetch_depth: u32,

    /// Use the last commit on the remote branch as the `base_sha`. Defaults to the last non merge commit on the target branch for pull request events and the previous remote commit of the current branch for push events.
    #[clap(short, long, default_value = "false")]
    pub since_last_remote_commit: bool,

    /// Write outputs to files in the `.github/outputs` folder by default.
    #[clap(short, long, default_value = "false")]
    pub write_output_files: bool,

    /// Directory to store output files.
    #[clap(short, long, default_value = ".github/outputs")]
    pub output_dir: String,

    /// Indicates whether to include match directories
    #[clap(short, long, default_value = "true")]
    pub match_directories: bool,
}
