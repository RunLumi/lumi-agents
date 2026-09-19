//! Git as a bounded project capability (spec 26 §26.16–26.17).
//!
//! The [`GitRepo`] wrapper exposes local READ operations (status, diff,
//! log, branches) and local WRITE operations (create/switch branch,
//! stage, commit, worktrees). External and destructive operations
//! (push, pull, remote branch changes, reset --hard, clean, force
//! operations, history rewrite) are deliberately NOT exposed here: they
//! are normalized external-write/DESTRUCTIVE actions that must pass the
//! policy engine and approval rules at the executor boundary — a bare
//! capability wrapper must not be able to reach the network or discard
//! work (spec 26 §26.17; see `tests/git_policy.rs` for the default
//! posture).
//!
//! Execution rules (spec 08 §8.6 applied to git): fixed argv (never a
//! shell), environment cleared to PATH/HOME plus git hardening
//! (`GIT_TERMINAL_PROMPT=0` refuses interactive credential prompts,
//! `GIT_CONFIG_NOSYSTEM=1` ignores machine-wide config, cleared env
//! defeats `GIT_DIR`/`GIT_WORK_TREE` injection), bounded output, and a
//! deadline on every command. Commits run with `--no-verify`: repo
//! hooks are repository-provided code and stay policy-gated (§26.27).

use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

/// Default deadline for one git command.
pub const GIT_TIMEOUT_MS: u64 = 30_000;

/// Maximum captured stdout bytes per command.
pub const GIT_MAX_OUTPUT_BYTES: usize = 1_048_576;

/// Why a git operation failed (§26.30 git failures are explicit).
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum GitError {
    NotARepository {
        path: String,
    },
    /// Requested path is not inside the repository worktree.
    OutsideRepo {
        path: String,
    },
    CommandFailed {
        command: String,
        stderr: String,
    },
    Timeout {
        command: String,
    },
    Io(String),
}

impl std::fmt::Display for GitError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotARepository { path } => write!(f, "not a git repository: {path}"),
            Self::OutsideRepo { path } => write!(f, "path is outside the repository: {path}"),
            Self::CommandFailed { command, stderr } => {
                write!(f, "git {command} failed: {stderr}")
            }
            Self::Timeout { command } => write!(f, "git {command} timed out"),
            Self::Io(e) => write!(f, "git execution failed: {e}"),
        }
    }
}

impl std::error::Error for GitError {}

/// One changed file from porcelain status (`XY` codes retained verbatim
/// so the UI can render git's own vocabulary).
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct FileStatus {
    /// Worktree-relative path (destination for renames).
    pub path: String,
    /// Original path for renames/copies.
    pub original_path: Option<String>,
    /// Index (staged) state code: M/A/D/R/C/T/U/…
    pub index_state: Option<char>,
    /// Worktree (unstaged) state code.
    pub worktree_state: Option<char>,
}

/// Repository status (§26.16): branch, HEAD, and the three buckets a
/// dirty tree is made of. A dirty tree is normal state, never an error.
#[derive(Debug, Clone, PartialEq, Eq, Default, serde::Serialize)]
pub struct GitStatus {
    /// Current branch name; `None` when HEAD is detached.
    pub branch: Option<String>,
    /// Full hash of HEAD; `None` on an unborn branch (fresh repo).
    pub head: Option<String>,
    pub staged: Vec<FileStatus>,
    pub unstaged: Vec<FileStatus>,
    pub untracked: Vec<String>,
}

impl GitStatus {
    /// True when nothing is staged, modified, or untracked.
    #[must_use]
    pub fn is_clean(&self) -> bool {
        self.staged.is_empty() && self.unstaged.is_empty() && self.untracked.is_empty()
    }

    /// Total number of changed entries (staged + unstaged + untracked).
    #[must_use]
    pub fn changed_count(&self) -> usize {
        self.staged.len() + self.unstaged.len() + self.untracked.len()
    }
}

/// One commit from the log.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct CommitInfo {
    pub hash: String,
    pub short_hash: String,
    pub author: String,
    pub subject: String,
    /// Author timestamp (unix seconds).
    pub timestamp: i64,
}

/// One registered worktree.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct WorktreeEntry {
    pub path: String,
    pub head: Option<String>,
    pub branch: Option<String>,
}

/// A git repository rooted at (or below) a project root.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GitRepo {
    /// Canonical worktree root (`git rev-parse --show-toplevel`).
    root: PathBuf,
}

impl GitRepo {
    /// Discovers the repository containing `dir`, if any. Requires the
    /// `git` binary; discovery is read-only.
    #[must_use]
    pub fn discover(dir: &Path) -> Option<Self> {
        let dotgit = dir.join(".git");
        if !dotgit.exists() {
            return None;
        }
        let output = run(dir, &["rev-parse", "--show-toplevel"]);
        match output {
            Ok(out) if out.succeeded() => {
                let root = PathBuf::from(out.stdout.trim());
                Some(Self { root })
            }
            _ => None,
        }
    }

    /// The canonical repository root.
    #[must_use]
    pub const fn root(&self) -> &PathBuf {
        &self.root
    }

    /// `status --porcelain=v1 -b` parsed into staged/unstaged/untracked
    /// (§26.16, §26.19).
    ///
    /// # Errors
    /// [`GitError`] on command failure.
    pub fn status(&self) -> Result<GitStatus, GitError> {
        let out = run_expect(&self.root, &["status", "--porcelain=v1", "-b"])?;
        let mut status = GitStatus::default();
        for line in out.stdout.lines() {
            if let Some(branch) = line.strip_prefix("## ") {
                if branch.starts_with("HEAD (no branch)") || branch.starts_with("No commits yet") {
                    if branch.starts_with("No commits yet") {
                        status.branch = branch
                            .trim_start_matches("No commits yet on ")
                            .split(',')
                            .next()
                            .map(str::trim)
                            .map(str::to_owned);
                    }
                    continue;
                }
                status.branch = branch.split("...").next().map(str::trim).map(str::to_owned);
                continue;
            }
            if line.len() < 3 {
                continue;
            }
            let (codes, rest) = line.split_at(2);
            let (index_c, worktree_c) = (
                codes.chars().next().unwrap_or(' '),
                codes.chars().nth(1).unwrap_or(' '),
            );
            let (path_part, original) = match rest.trim_start().split_once(" -> ") {
                Some((from, to)) => (to.to_owned(), Some(from.to_owned())),
                None => (rest.trim_start().to_owned(), None),
            };
            let path = unquote(path_part);
            if index_c == '?' && worktree_c == '?' {
                status.untracked.push(path);
                continue;
            }
            let entry = FileStatus {
                path,
                original_path: original,
                index_state: (index_c != ' ' && index_c != '?').then_some(index_c),
                worktree_state: (worktree_c != ' ' && worktree_c != '?').then_some(worktree_c),
            };
            if entry.index_state.is_some() {
                status.staged.push(entry.clone());
            }
            if entry.worktree_state.is_some() {
                status.unstaged.push(entry);
            }
        }
        status.head = self.head_hash().ok().flatten();
        Ok(status)
    }

    /// Full hash of HEAD; `None` on an unborn branch or unresolvable
    /// HEAD (explicit absence — branch/status carry the rest).
    ///
    /// # Errors
    /// [`GitError`] on command execution failure (not on absence).
    pub fn head_hash(&self) -> Result<Option<String>, GitError> {
        let out = run(&self.root, &["rev-parse", "--verify", "--quiet", "HEAD"])?;
        Ok(out.succeeded().then(|| out.stdout.trim().to_owned()))
    }

    /// Bounded summary diff of tracked changes (staged + unstaged vs
    /// HEAD). Untracked files appear in [`GitRepo::status`], not here.
    ///
    /// # Errors
    /// [`GitError`] on command failure.
    pub fn diff_stat(&self) -> Result<String, GitError> {
        let out = run_expect(
            &self.root,
            &["-c", "core.quotepath=false", "diff", "HEAD", "--stat"],
        )?;
        Ok(out.stdout)
    }

    /// Full bounded diff for one path (or the whole tree when `None`).
    ///
    /// # Errors
    /// [`GitError`] on command failure.
    pub fn diff(&self, path: Option<&str>) -> Result<String, GitError> {
        let mut args = vec!["-c", "core.quotepath=false", "diff", "HEAD", "--"];
        if let Some(p) = path {
            args.push(p);
        }
        let out = run_expect(&self.root, &args)?;
        Ok(out.stdout)
    }

    /// Recent commits, newest first (§26.16 log/history).
    ///
    /// # Errors
    /// [`GitError`] on command failure.
    pub fn log(&self, max: usize) -> Result<Vec<CommitInfo>, GitError> {
        let out = run_expect(
            &self.root,
            &[
                "log",
                "-n",
                &max.to_string(),
                "--pretty=format:%H%x1f%h%x1f%an%x1f%at%x1f%s%x1e",
            ],
        )?;
        let mut commits = Vec::new();
        for record in out.stdout.split('\x1e') {
            let record = record.trim_start_matches('\n');
            if record.trim().is_empty() {
                continue;
            }
            let fields: Vec<&str> = record.split('\x1f').collect();
            if fields.len() < 5 {
                continue;
            }
            commits.push(CommitInfo {
                hash: fields[0].to_owned(),
                short_hash: fields[1].to_owned(),
                author: fields[2].to_owned(),
                timestamp: fields[3].parse().unwrap_or(0),
                subject: fields[4].to_owned(),
            });
        }
        Ok(commits)
    }

    /// Local branch names (§26.17 local read).
    ///
    /// # Errors
    /// [`GitError`] on command failure.
    pub fn branches(&self) -> Result<Vec<String>, GitError> {
        let out = run_expect(&self.root, &["branch", "--format=%(refname:short)"])?;
        Ok(out
            .stdout
            .lines()
            .map(str::trim)
            .map(str::to_owned)
            .collect())
    }

    /// Creates a branch at HEAD (§26.17 local write). Does not switch.
    ///
    /// # Errors
    /// [`GitError`] on command failure (e.g. branch exists).
    pub fn create_branch(&self, name: &str) -> Result<(), GitError> {
        run_expect(&self.root, &["branch", name]).map(|_| ())
    }

    /// Switches branches (§26.17 local write).
    ///
    /// # Errors
    /// [`GitError`] on command failure.
    pub fn switch(&self, name: &str) -> Result<(), GitError> {
        run_expect(&self.root, &["switch", name]).map(|_| ())
    }

    /// Stages explicit paths (§26.17 local write). Paths must be inside
    /// the repository worktree; `.git` itself is never stageable.
    ///
    /// # Errors
    /// [`GitError::OutsideRepo`] for out-of-scope paths; [`GitError`]
    /// for command failures.
    pub fn stage(&self, paths: &[&Path]) -> Result<(), GitError> {
        if paths.is_empty() {
            return Ok(());
        }
        let mut args: Vec<String> = vec!["add".to_owned(), "--".to_owned()];
        for path in paths {
            self.validate_in_repo(path)?;
            args.push(path.to_string_lossy().to_string());
        }
        let arg_refs: Vec<&str> = args.iter().map(String::as_str).collect();
        run_expect(&self.root, &arg_refs).map(|_| ())
    }

    /// Rejects `.git` and any path outside the repository worktree.
    ///
    /// Comparisons normalize Windows verbatim paths (`\\?\C:\...`) that
    /// `canonicalize` produces against the plain form `git` reports, so
    /// in-repo paths are never falsely refused cross-platform.
    fn validate_in_repo(&self, path: &Path) -> Result<(), GitError> {
        if path.as_os_str() == ".git" {
            return Err(GitError::OutsideRepo {
                path: path.display().to_string(),
            });
        }
        let absolute = if path.is_absolute() {
            path.to_path_buf()
        } else {
            self.root.join(path)
        };
        let canonical = std::fs::canonicalize(&absolute)
            .map_err(|e| GitError::Io(format!("resolving {}: {e}", path.display())))?;
        let repo_root = std::fs::canonicalize(&self.root)
            .map_err(|e| GitError::Io(format!("resolving repo root: {e}")))?;
        let canonical = normalize_path(canonical);
        let repo_root = normalize_path(repo_root);
        if !canonical.starts_with(&repo_root) || canonical == repo_root.join(".git") {
            return Err(GitError::OutsideRepo {
                path: path.display().to_string(),
            });
        }
        Ok(())
    }

    /// Commits exactly the given paths' current content (§26.17 local
    /// write). Path-scoped so a Lumi commit can NEVER sweep in
    /// pre-existing user-staged work (§26.19): anything else in the
    /// index stays staged. Uses `--no-verify` so repository hooks are
    /// never executed by Lumi (§26.27); author identity comes from the
    /// repository/git configuration. Returns the new HEAD hash.
    ///
    /// # Errors
    /// [`GitError::OutsideRepo`] for out-of-scope paths; [`GitError`]
    /// for command failures (including missing identity).
    pub fn commit(&self, message: &str, paths: &[&Path]) -> Result<String, GitError> {
        if paths.is_empty() {
            return Err(GitError::CommandFailed {
                command: "commit".to_owned(),
                stderr: "refusing to commit with no explicit paths".to_owned(),
            });
        }
        let mut args: Vec<String> = vec![
            "commit".to_owned(),
            "--no-verify".to_owned(),
            "-m".to_owned(),
            message.to_owned(),
            "--".to_owned(),
        ];
        for path in paths {
            self.validate_in_repo(path)?;
            args.push(path.to_string_lossy().to_string());
        }
        let arg_refs: Vec<&str> = args.iter().map(String::as_str).collect();
        run_expect(&self.root, &arg_refs)?;
        let head = run_expect(&self.root, &["rev-parse", "HEAD"])?;
        Ok(head.stdout.trim().to_owned())
    }

    /// Creates an isolated worktree with a new branch (§26.17 local
    /// write; the ISOLATED_WORKTREE workspace kind builds on this).
    ///
    /// # Errors
    /// [`GitError`] on command failure.
    pub fn create_worktree(&self, path: &Path, new_branch: &str) -> Result<(), GitError> {
        let target = normalize_path(path.to_path_buf());
        run_expect(
            &self.root,
            &[
                "worktree",
                "add",
                "-b",
                new_branch,
                &target.to_string_lossy(),
            ],
        )
        .map(|_| ())
    }

    /// Lists registered worktrees (§26.16 worktree state).
    ///
    /// # Errors
    /// [`GitError`] on command failure.
    pub fn worktrees(&self) -> Result<Vec<WorktreeEntry>, GitError> {
        let out = run_expect(&self.root, &["worktree", "list", "--porcelain"])?;
        let mut trees = Vec::new();
        let mut current = WorktreeEntry {
            path: String::new(),
            head: None,
            branch: None,
        };
        for line in out.stdout.lines() {
            if let Some(path) = line.strip_prefix("worktree ") {
                if !current.path.is_empty() {
                    trees.push(std::mem::replace(
                        &mut current,
                        WorktreeEntry {
                            path: String::new(),
                            head: None,
                            branch: None,
                        },
                    ));
                }
                current.path = path.to_owned();
            } else if let Some(head) = line.strip_prefix("HEAD ") {
                current.head = Some(head.to_owned());
            } else if let Some(branch) = line.strip_prefix("branch ") {
                current.branch = Some(branch.trim_start_matches("refs/heads/").to_owned());
            }
        }
        if !current.path.is_empty() {
            trees.push(current);
        }
        Ok(trees)
    }
}

/// Clones `source` into `destination` (spec 26 §26.27: clone is a
/// network + filesystem action; the caller (desktop user or policy)
/// supplies the authority, and nothing cloned is executed by this
/// crate). Deterministic tests use local path sources, which `git
/// clone` supports natively.
///
/// Returns the cloned worktree root.
///
/// # Errors
/// [`GitError`] on command failure or if the destination already exists.
pub fn clone_repository(source: &str, destination: &Path) -> Result<GitRepo, GitError> {
    // `destination` is the parent directory (must already exist); the
    // clone target is `<destination>/<repo name>` and must NOT exist.
    let destination = normalize_path(destination.to_path_buf());
    if !destination.is_dir() {
        return Err(GitError::Io(format!(
            "clone destination parent is not a directory: {}",
            destination.display()
        )));
    }
    // Folder name: last segment of the source (both separators), minus a
    // trailing ".git". A verbatim Windows source (\\?\C:\...) must not
    // leak into the destination name.
    let source_path = normalize_path(PathBuf::from(source));
    let repo_name = source_path
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_default();
    let repo_name = repo_name.trim_end_matches(".git");
    if repo_name.is_empty() {
        return Err(GitError::Io(
            "cannot derive a folder name from the source".to_owned(),
        ));
    }
    let target = destination.join(repo_name);
    if target.exists() {
        return Err(GitError::Io(format!(
            "clone destination already exists: {}",
            target.display()
        )));
    }
    run_expect(
        &destination,
        &["clone", "--quiet", source, &target.to_string_lossy()],
    )?;
    GitRepo::discover(&target)
        .ok_or_else(|| GitError::Io("cloned destination is not a repository".to_owned()))
}

struct GitOutput {
    succeeded: bool,
    stdout: String,
    stderr: String,
}

impl GitOutput {
    fn succeeded(&self) -> bool {
        self.succeeded
    }
}

fn run(dir: &Path, args: &[&str]) -> Result<GitOutput, GitError> {
    let display = format!("git {}", args.join(" "));
    let mut command = Command::new("git");
    command
        .args(args)
        .current_dir(dir)
        .env_clear()
        .env("PATH", std::env::var("PATH").unwrap_or_default())
        .env("HOME", std::env::var("HOME").unwrap_or_default())
        .env("GIT_TERMINAL_PROMPT", "0")
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    #[cfg(windows)]
    command.env(
        "SystemRoot",
        std::env::var("SystemRoot").unwrap_or_else(|_| "C:\\Windows".to_owned()),
    );

    let mut child = command
        .spawn()
        .map_err(|e| GitError::Io(format!("spawning {display}: {e}")))?;
    // Drain both pipes concurrently (a full pipe buffer would otherwise
    // deadlock the child while we poll for exit).
    let mut stdout_pipe = child.stdout.take();
    let mut stderr_pipe = child.stderr.take();
    let stdout_reader = std::thread::spawn(move || {
        use std::io::Read;
        let mut buf = Vec::new();
        if let Some(pipe) = stdout_pipe.as_mut() {
            let _ = pipe.read_to_end(&mut buf);
        }
        buf
    });
    let stderr_reader = std::thread::spawn(move || {
        use std::io::Read;
        let mut buf = Vec::new();
        if let Some(pipe) = stderr_pipe.as_mut() {
            let _ = pipe.read_to_end(&mut buf);
        }
        buf
    });

    let deadline = Duration::from_millis(GIT_TIMEOUT_MS);
    let started = Instant::now();
    loop {
        match child.try_wait() {
            Ok(Some(status)) => {
                let mut stdout = stdout_reader.join().unwrap_or_default();
                let mut stderr = stderr_reader.join().unwrap_or_default();
                stdout.truncate(GIT_MAX_OUTPUT_BYTES);
                stderr.truncate(GIT_MAX_OUTPUT_BYTES);
                return Ok(GitOutput {
                    succeeded: status.success(),
                    stdout: String::from_utf8_lossy(&stdout).to_string(),
                    stderr: String::from_utf8_lossy(&stderr).to_string(),
                });
            }
            Ok(None) => {
                if started.elapsed() >= deadline {
                    let _ = child.kill();
                    let _ = child.wait();
                    return Err(GitError::Timeout { command: display });
                }
                std::thread::sleep(Duration::from_millis(10));
            }
            Err(e) => return Err(GitError::Io(format!("waiting on {display}: {e}"))),
        }
    }
}

fn run_expect(dir: &Path, args: &[&str]) -> Result<GitOutput, GitError> {
    let out = run(dir, args)?;
    if out.succeeded() {
        Ok(out)
    } else {
        Err(GitError::CommandFailed {
            command: format!("git {}", args.join(" ")),
            stderr: out.stderr,
        })
    }
}

fn unquote(path: String) -> String {
    if path.starts_with('"') && path.ends_with('"') && path.len() >= 2 {
        path[1..path.len() - 1].to_string()
    } else {
        path
    }
}

/// Removes the Windows verbatim prefix `canonicalize` produces so
/// canonical paths compare equal to the plain paths git reports
/// (`\\?\C:\a` -> `C:\a`, `\\?\UNC\server\share` -> `\\server\share`).
#[must_use]
pub fn normalize_path(path: PathBuf) -> PathBuf {
    let text = path.as_os_str().to_string_lossy();
    if let Some(stripped) = text.strip_prefix(r"\\?\UNC\") {
        return PathBuf::from(format!(r"\\{stripped}"));
    }
    if let Some(stripped) = text.strip_prefix(r"\\?\") {
        return PathBuf::from(stripped);
    }
    path
}
