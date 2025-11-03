use serde::{Deserialize, Serialize};
use std::process::Command;

/// CI status from GitHub/GitLab checks
/// Matches the statusline.sh color scheme:
/// - Passed: Green (all checks passed)
/// - Running: Blue (checks in progress)
/// - Failed: Red (checks failed)
/// - Conflicts: Yellow (merge conflicts)
/// - NoCI: Gray (no PR/checks)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CiStatus {
    Passed,
    Running,
    Failed,
    Conflicts,
    NoCI,
}

/// PR/MR status including CI state and staleness
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrStatus {
    pub ci_status: CiStatus,
    /// True if local HEAD differs from PR HEAD (unpushed changes)
    pub is_stale: bool,
}

impl PrStatus {
    /// Detect CI status for a branch using gh/glab CLI
    /// First tries to find PR/MR status, then falls back to workflow/pipeline runs
    /// Returns None if no CI found or CLI tools unavailable
    pub fn detect(branch: &str, local_head: &str) -> Option<Self> {
        // Try GitHub PR first
        if let Some(status) = Self::detect_github(branch, local_head) {
            return Some(status);
        }

        // Try GitHub workflow runs (for branches without PRs)
        if let Some(status) = Self::detect_github_workflow(branch, local_head) {
            return Some(status);
        }

        // Try GitLab MR
        if let Some(status) = Self::detect_gitlab(branch, local_head) {
            return Some(status);
        }

        // Fall back to GitLab pipeline (for branches without MRs)
        Self::detect_gitlab_pipeline(branch, local_head)
    }

    fn detect_github(branch: &str, local_head: &str) -> Option<Self> {
        // Check if gh is available and authenticated
        if !Command::new("gh")
            .args(["auth", "status"])
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
        {
            return None;
        }

        // Get PR info for the branch
        let output = Command::new("gh")
            .args([
                "pr",
                "view",
                branch,
                "--json",
                "state,headRefOid,mergeStateStatus,statusCheckRollup",
            ])
            .output()
            .ok()?;

        if !output.status.success() {
            return None;
        }

        let pr_info: GitHubPrInfo = serde_json::from_slice(&output.stdout).ok()?;

        // Only process open PRs
        if pr_info.state != "OPEN" {
            return None;
        }

        // Determine CI status using priority: conflicts > running > failed > passed > no_ci
        let ci_status = if pr_info.merge_state_status.as_deref() == Some("DIRTY") {
            CiStatus::Conflicts
        } else {
            Self::analyze_github_checks(&pr_info.status_check_rollup)
        };

        let is_stale = pr_info
            .head_ref_oid
            .as_ref()
            .map(|pr_head| pr_head != local_head)
            .unwrap_or(false);

        Some(PrStatus {
            ci_status,
            is_stale,
        })
    }

    fn analyze_github_checks(checks: &Option<Vec<GitHubCheck>>) -> CiStatus {
        let Some(checks) = checks else {
            return CiStatus::NoCI;
        };

        if checks.is_empty() {
            return CiStatus::NoCI;
        }

        let has_pending = checks.iter().any(|c| {
            matches!(
                c.status.as_deref(),
                Some("IN_PROGRESS" | "QUEUED" | "PENDING" | "EXPECTED")
            )
        });

        let has_failure = checks.iter().any(|c| {
            matches!(
                c.conclusion.as_deref(),
                Some("FAILURE" | "ERROR" | "CANCELLED")
            )
        });

        if has_pending {
            CiStatus::Running
        } else if has_failure {
            CiStatus::Failed
        } else {
            CiStatus::Passed
        }
    }

    fn detect_gitlab(branch: &str, local_head: &str) -> Option<Self> {
        // Check if glab is available
        if !Command::new("glab")
            .arg("--version")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
        {
            return None;
        }

        // Get MR info for the branch
        let output = Command::new("glab")
            .args(["mr", "view", branch, "--output", "json"])
            .output()
            .ok()?;

        if !output.status.success() {
            return None;
        }

        let mr_info: GitLabMrInfo = serde_json::from_slice(&output.stdout).ok()?;

        // Only process open MRs
        if mr_info.state != "opened" {
            return None;
        }

        // Determine CI status using priority: conflicts > running > failed > passed > no_ci
        let ci_status = if mr_info.has_conflicts
            || mr_info.detailed_merge_status.as_deref() == Some("conflict")
        {
            CiStatus::Conflicts
        } else if mr_info.detailed_merge_status.as_deref() == Some("ci_still_running") {
            CiStatus::Running
        } else if mr_info.detailed_merge_status.as_deref() == Some("ci_must_pass") {
            CiStatus::Failed
        } else {
            Self::analyze_gitlab_pipeline(mr_info.pipeline_status())
        };

        let is_stale = mr_info.sha != local_head;

        Some(PrStatus {
            ci_status,
            is_stale,
        })
    }

    fn analyze_gitlab_pipeline(pipeline_status: Option<&String>) -> CiStatus {
        match pipeline_status.map(|s| s.as_str()) {
            Some(
                "running"
                | "pending"
                | "preparing"
                | "waiting_for_resource"
                | "created"
                | "scheduled",
            ) => CiStatus::Running,
            Some("failed" | "canceled") => CiStatus::Failed,
            Some("success") => CiStatus::Passed,
            Some("skipped" | "manual") | None => CiStatus::NoCI,
            _ => CiStatus::NoCI,
        }
    }

    fn detect_github_workflow(branch: &str, _local_head: &str) -> Option<Self> {
        // Check if gh is available and authenticated
        if !Command::new("gh")
            .args(["auth", "status"])
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
        {
            return None;
        }

        // Get most recent workflow run for the branch
        let output = Command::new("gh")
            .args([
                "run",
                "list",
                "--branch",
                branch,
                "--limit",
                "1",
                "--json",
                "status,conclusion",
            ])
            .output()
            .ok()?;

        if !output.status.success() {
            return None;
        }

        let runs: Vec<GitHubWorkflowRun> = serde_json::from_slice(&output.stdout).ok()?;
        let run = runs.first()?;

        // Analyze workflow run status
        let ci_status = Self::analyze_github_workflow_run(run);

        // Workflow runs don't have staleness concept (no PR to compare against)
        Some(PrStatus {
            ci_status,
            is_stale: false,
        })
    }

    fn analyze_github_workflow_run(run: &GitHubWorkflowRun) -> CiStatus {
        match run.status.as_deref() {
            Some("in_progress" | "queued" | "pending" | "waiting") => CiStatus::Running,
            Some("completed") => match run.conclusion.as_deref() {
                Some("success") => CiStatus::Passed,
                Some("failure" | "cancelled" | "timed_out" | "action_required") => CiStatus::Failed,
                Some("skipped" | "neutral") | None => CiStatus::NoCI,
                _ => CiStatus::NoCI,
            },
            _ => CiStatus::NoCI,
        }
    }

    fn detect_gitlab_pipeline(branch: &str, _local_head: &str) -> Option<Self> {
        // Check if glab is available
        if !Command::new("glab")
            .arg("--version")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
        {
            return None;
        }

        // Get most recent pipeline for the branch
        let output = Command::new("glab")
            .args(["ci", "list", "--per-page", "1"])
            .env("BRANCH", branch) // glab ci list uses BRANCH env var
            .output()
            .ok()?;

        if !output.status.success() {
            return None;
        }

        // Parse glab ci list output (format: "• (<status>) <pipeline-info>")
        let output_str = String::from_utf8(output.stdout).ok()?;
        let first_line = output_str.lines().next()?;

        // Extract status from format like "• (running) #12345"
        let status_start = first_line.find('(')?;
        let status_end = first_line.find(')')?;
        let status = &first_line[status_start + 1..status_end];

        let ci_status = Self::analyze_gitlab_pipeline(Some(&status.to_string()));

        Some(PrStatus {
            ci_status,
            is_stale: false,
        })
    }
}

#[derive(Debug, Deserialize)]
struct GitHubPrInfo {
    state: String,
    #[serde(rename = "headRefOid")]
    head_ref_oid: Option<String>,
    #[serde(rename = "mergeStateStatus")]
    merge_state_status: Option<String>,
    #[serde(rename = "statusCheckRollup")]
    status_check_rollup: Option<Vec<GitHubCheck>>,
}

#[derive(Debug, Deserialize)]
struct GitHubCheck {
    status: Option<String>,
    conclusion: Option<String>,
}

#[derive(Debug, Deserialize)]
struct GitHubWorkflowRun {
    status: Option<String>,
    conclusion: Option<String>,
}

#[derive(Debug, Deserialize)]
struct GitLabMrInfo {
    state: String,
    sha: String,
    has_conflicts: bool,
    detailed_merge_status: Option<String>,
    head_pipeline: Option<GitLabPipeline>,
    pipeline: Option<GitLabPipeline>,
}

impl GitLabMrInfo {
    fn pipeline_status(&self) -> Option<&String> {
        self.head_pipeline
            .as_ref()
            .or(self.pipeline.as_ref())
            .and_then(|p| p.status.as_ref())
    }
}

#[derive(Debug, Deserialize)]
struct GitLabPipeline {
    status: Option<String>,
}
