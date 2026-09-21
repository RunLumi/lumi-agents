//! Durable project engagement and schedule definitions. This service creates
//! ordinary tasks; it cannot authorize an executor or bypass the orchestrator.
use chrono::{DateTime, Datelike, Days, NaiveDateTime, NaiveTime, TimeZone, Utc};
use chrono_tz::Tz;
use lumi_protocol::{canonical::sha256_hex, TaskId, Timestamp};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::fs::{File, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

pub const MAX_AUTHORIZATION_SECONDS: i64 = 30 * 86_400;
pub const MAX_RUN_SECONDS: i64 = 20 * 60;
const MAX_STATE_BYTES: u64 = 16 * 1024 * 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ToolId {
    Files,
    Browser,
    Chrome,
    Computer,
    Shell,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CapabilityDescriptor {
    pub id: ToolId,
    pub authority_class: &'static str,
    pub interactive: bool,
    pub unattended: bool,
    pub implemented: bool,
}

/// The backend-owned capability catalog. UI snapshots, selection validation,
/// scheduler admission and runtime tool construction must not invent separate
/// capability lists.
pub const fn capability_catalog() -> [CapabilityDescriptor; 5] {
    [
        CapabilityDescriptor {
            id: ToolId::Files,
            authority_class: "project_files",
            interactive: true,
            unattended: true,
            implemented: true,
        },
        CapabilityDescriptor {
            id: ToolId::Shell,
            authority_class: "project_commands",
            interactive: true,
            unattended: false,
            implemented: true,
        },
        CapabilityDescriptor {
            id: ToolId::Browser,
            authority_class: "public_browser_read",
            interactive: true,
            unattended: true,
            implemented: true,
        },
        CapabilityDescriptor {
            id: ToolId::Chrome,
            authority_class: "authenticated_chrome_session",
            interactive: true,
            unattended: false,
            implemented: false,
        },
        CapabilityDescriptor {
            id: ToolId::Computer,
            authority_class: "native_computer",
            interactive: true,
            unattended: false,
            implemented: false,
        },
    ]
}

pub fn capability_descriptor(id: ToolId) -> CapabilityDescriptor {
    capability_catalog()
        .into_iter()
        .find(|descriptor| descriptor.id == id)
        .expect("capability catalog must contain every ToolId")
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CreateRequest {
    pub project_id: String,
    pub goal: String,
    pub tools: Vec<ToolId>,
    pub request_id: String,
}
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TaskOptions {
    pub tools: Vec<ToolId>,
    pub browser_origins: Vec<String>,
    pub automation_id: Option<String>,
    pub authorization_until: Option<i64>,
}
impl TaskOptions {
    #[must_use]
    pub fn files_only() -> Self {
        Self {
            tools: vec![ToolId::Files],
            ..Self::default()
        }
    }
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum ScheduleSpec {
    Once { local_datetime: String },
    Every { minutes: u32 },
    Daily { time: String },
    Weekly { time: String, weekdays: Vec<u32> },
}
impl ScheduleSpec {
    /// Calendar recurrence uses the named IANA timezone. A missing wall time
    /// is skipped; an ambiguous wall time uses its earlier instant, once.
    pub fn next_after(
        &self,
        timezone: &str,
        anchor: i64,
        after: i64,
    ) -> Result<Option<i64>, String> {
        let zone: Tz = timezone
            .parse()
            .map_err(|_| "unknown IANA timezone".to_owned())?;
        let now = DateTime::<Utc>::from_timestamp(after, 0).ok_or("timestamp out of range")?;
        match self {
            Self::Once { local_datetime } => {
                let local = NaiveDateTime::parse_from_str(local_datetime, "%Y-%m-%dT%H:%M")
                    .map_err(|_| "use a local date/time in YYYY-MM-DDTHH:MM format")?;
                let at = zone
                    .from_local_datetime(&local)
                    .earliest()
                    .ok_or("that local time does not exist in the selected timezone")?
                    .timestamp();
                Ok((at > after).then_some(at))
            }
            Self::Every { minutes } => {
                if !(5..=525_600).contains(minutes) {
                    return Err("interval must be between 5 and 525600 minutes".into());
                }
                let step = i64::from(*minutes) * 60;
                let elapsed = after.checked_sub(anchor).ok_or("interval overflow")?.max(0);
                let delta = (elapsed / step + 1)
                    .checked_mul(step)
                    .ok_or("interval overflow")?;
                Ok(Some(anchor.checked_add(delta).ok_or("interval overflow")?))
            }
            Self::Daily { time } | Self::Weekly { time, .. } => {
                let time = NaiveTime::parse_from_str(time, "%H:%M")
                    .map_err(|_| "use a local time in HH:MM format")?;
                if let Self::Weekly { weekdays, .. } = self {
                    if weekdays.is_empty() || weekdays.iter().any(|day| !(1..=7).contains(day)) {
                        return Err("select weekdays from Monday=1 through Sunday=7".into());
                    }
                }
                let date = now.with_timezone(&zone).date_naive();
                for offset in 0..370 {
                    let Some(day) = date.checked_add_days(Days::new(offset)) else {
                        break;
                    };
                    if let Self::Weekly { weekdays, .. } = self {
                        if !weekdays.contains(&day.weekday().number_from_monday()) {
                            continue;
                        }
                    }
                    if let Some(at) = zone.from_local_datetime(&day.and_time(time)).earliest() {
                        if at.timestamp() > after {
                            return Ok(Some(at.timestamp()));
                        }
                    }
                }
                Err("could not resolve the next calendar occurrence".into())
            }
        }
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CatchUp {
    RunOnce,
    Skip,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutomationInput {
    pub automation_id: Option<String>,
    pub revision: Option<u64>,
    pub name: String,
    pub goal: String,
    pub tools: Vec<ToolId>,
    pub schedule: ScheduleSpec,
    pub timezone: String,
    pub catch_up: CatchUp,
    pub enabled: bool,
    pub authorization_until: Option<i64>,
}
impl AutomationInput {
    pub fn preview(&self, now: i64) -> Result<Vec<i64>, String> {
        let mut after = now;
        let mut values = Vec::new();
        for _ in 0..3 {
            let Some(next) = self.schedule.next_after(&self.timezone, now, after)? else {
                break;
            };
            values.push(next);
            after = next;
        }
        Ok(values)
    }
    pub fn validate(&self, now: i64) -> Result<(), String> {
        validate_goal(&self.goal)?;
        if self.name.trim().is_empty() || self.name.len() > 240 {
            return Err("automation name is required (maximum 240 bytes)".into());
        }
        normalized_tools(&self.goal, &self.tools, true)?;
        self.schedule.next_after(&self.timezone, now, now)?;
        if self.enabled {
            let until = self
                .authorization_until
                .ok_or("unattended authorization is required")?;
            if until <= now || until > now + MAX_AUTHORIZATION_SECONDS {
                return Err(
                    "authorize unattended work for a future time no more than 30 days away".into(),
                );
            }
        }
        Ok(())
    }
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutomationRun {
    pub occurrence_id: String,
    pub scheduled_at: i64,
    pub task_id: Option<String>,
    pub status: String,
    pub note: Option<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Automation {
    #[serde(flatten)]
    pub input: AutomationInput,
    pub project_id: String,
    pub created_at: i64,
    pub next_run_at: Option<i64>,
    pub runs: Vec<AutomationRun>,
}
impl Automation {
    pub fn id(&self) -> Result<&str, String> {
        self.input
            .automation_id
            .as_deref()
            .ok_or_else(|| "automation has no identity".into())
    }
    pub fn revision(&self) -> u64 {
        self.input.revision.unwrap_or(0)
    }
}
#[derive(Debug, Clone, Serialize, Deserialize)]
struct RequestRecord {
    digest: String,
    task_id: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Snapshot {
    schema_version: u32,
    origins: BTreeMap<String, Vec<String>>,
    options: BTreeMap<String, TaskOptions>,
    requests: BTreeMap<String, RequestRecord>,
    automations: BTreeMap<String, Automation>,
    admitted: BTreeSet<String>,
}
impl Default for Snapshot {
    fn default() -> Self {
        Self {
            schema_version: 1,
            origins: BTreeMap::new(),
            options: BTreeMap::new(),
            requests: BTreeMap::new(),
            automations: BTreeMap::new(),
            admitted: BTreeSet::new(),
        }
    }
}
/// One process owns this store. The OS lock is released on crashes, so no
/// stale lock-file deletion or timestamp guessing is needed after restart.
pub struct EngagementStore {
    path: PathBuf,
    _lock: File,
    snapshot: Snapshot,
    poisoned: Option<String>,
}
impl EngagementStore {
    pub fn open(path: PathBuf) -> Result<Self, String> {
        let parent = path
            .parent()
            .ok_or("engagement store needs a parent directory")?;
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        for candidate in [&path, &path.with_extension("lock")] {
            if std::fs::symlink_metadata(candidate).is_ok_and(|m| m.file_type().is_symlink()) {
                return Err("engagement state and lock must not be symlinks".into());
            }
        }
        let lock = OpenOptions::new()
            .create(true)
            .truncate(false)
            .read(true)
            .write(true)
            .open(path.with_extension("lock"))
            .map_err(|e| e.to_string())?;
        lock.try_lock()
            .map_err(|e| format!("another runtime owns engagement state: {e}"))?;
        let snapshot = match std::fs::metadata(&path) {
            Ok(metadata) => {
                if metadata.len() > MAX_STATE_BYTES {
                    return Err("engagement state exceeds its size limit".into());
                }
                serde_json::from_slice::<Snapshot>(
                    &std::fs::read(&path).map_err(|e| e.to_string())?,
                )
                .map_err(|e| format!("invalid engagement state; no schedules activated: {e}"))?
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Snapshot::default(),
            Err(e) => return Err(e.to_string()),
        };
        if snapshot.schema_version != 1 {
            return Err("unsupported engagement schema version".into());
        }
        for (id, item) in &snapshot.automations {
            if item.id()? != id || item.revision() == 0 {
                return Err("invalid automation identity or revision".into());
            }
        }
        let mut store = Self {
            path,
            _lock: lock,
            snapshot,
            poisoned: None,
        };
        // Never replay a possibly dispatched occurrence after an unclean exit.
        if store.snapshot.automations.values().any(|a| {
            a.runs
                .iter()
                .any(|r| matches!(r.status.as_str(), "CLAIMED" | "QUEUED" | "RUNNING"))
        }) {
            store.mutate(|state| {
                for automation in state.automations.values_mut() {
                    let mut interrupted = false;
                    for run in &mut automation.runs {
                        if matches!(run.status.as_str(), "CLAIMED" | "QUEUED" | "RUNNING") {
                            run.status = "INTERRUPTED".into();
                            run.note = Some("Runtime restarted. Review the linked task and any prior effects before enabling this automation again.".into());
                            interrupted = true;
                        }
                    }
                    if interrupted { automation.input.enabled = false; automation.input.revision = Some(automation.revision() + 1); }
                }
                Ok(())
            })?;
        }
        Ok(store)
    }
    fn healthy(&self) -> Result<(), String> {
        match &self.poisoned {
            Some(reason) => Err(reason.clone()),
            None => Ok(()),
        }
    }
    fn mutate<T>(
        &mut self,
        update: impl FnOnce(&mut Snapshot) -> Result<T, String>,
    ) -> Result<T, String> {
        self.healthy()?;
        let mut candidate = self.snapshot.clone();
        let result = update(&mut candidate)?;
        if let Err(error) = atomic_save(&self.path, &candidate) {
            let reason = format!(
                "engagement persistence failed; restart and reconcile before more work: {error}"
            );
            self.poisoned = Some(reason.clone());
            return Err(reason);
        }
        self.snapshot = candidate;
        Ok(result)
    }
    pub fn origins(&self, project: &str) -> Result<Vec<String>, String> {
        self.healthy()?;
        Ok(self
            .snapshot
            .origins
            .get(project)
            .cloned()
            .unwrap_or_default())
    }
    pub fn set_origins(&mut self, project: &str, origins: Vec<String>) -> Result<(), String> {
        let normalized = crate::browser_tools::normalize_origins(&origins)?;
        self.mutate(|s| {
            if s.origins.get(project) != Some(&normalized) {
                for automation in s
                    .automations
                    .values_mut()
                    .filter(|a| a.project_id == project)
                {
                    if automation.input.tools.contains(&ToolId::Browser) {
                        automation.input.enabled = false;
                        automation.input.revision = Some(automation.revision() + 1);
                    }
                }
            }
            s.origins.insert(project.into(), normalized);
            Ok(())
        })
    }
    pub fn task_options(&self, task: &str) -> Result<TaskOptions, String> {
        self.healthy()?;
        // No metadata means no added capability, never a broad default.
        Ok(self
            .snapshot
            .options
            .get(task)
            .cloned()
            .unwrap_or_else(TaskOptions::files_only))
    }
    pub fn existing_request(&self, request: &CreateRequest) -> Result<Option<String>, String> {
        self.healthy()?;
        validate_request(request)?;
        let key = format!("{}:{}", request.project_id, request.request_id);
        match self.snapshot.requests.get(&key) {
            Some(record) if record.digest == request_digest(request)? => {
                Ok(Some(record.task_id.clone()))
            }
            Some(_) => Err("request identity was already used with different task contents".into()),
            None => Ok(None),
        }
    }
    pub fn attach_task(
        &mut self,
        request: &CreateRequest,
        task_id: &str,
        options: TaskOptions,
    ) -> Result<(), String> {
        let digest = request_digest(request)?;
        let key = format!("{}:{}", request.project_id, request.request_id);
        self.mutate(|s| {
            if s.requests.len() >= 100_000 {
                return Err("task request ledger requires maintenance".into());
            }
            if let Some(previous) = s.requests.get(&key) {
                if previous.task_id != task_id || previous.digest != digest {
                    return Err("duplicate task request".into());
                }
            }
            s.requests.insert(
                key,
                RequestRecord {
                    digest,
                    task_id: task_id.into(),
                },
            );
            s.options.insert(task_id.into(), options);
            Ok(())
        })
    }
    pub fn list(&self, project: &str) -> Result<Vec<Automation>, String> {
        self.healthy()?;
        Ok(self
            .snapshot
            .automations
            .values()
            .filter(|a| a.project_id == project)
            .cloned()
            .collect())
    }
    pub fn get(&self, project: &str, id: &str) -> Result<Automation, String> {
        self.healthy()?;
        self.snapshot
            .automations
            .get(id)
            .filter(|a| a.project_id == project)
            .cloned()
            .ok_or_else(|| "automation not found in this project".into())
    }
    pub fn save(
        &mut self,
        project: &str,
        mut input: AutomationInput,
        now: i64,
    ) -> Result<Automation, String> {
        input.validate(now)?;
        input.tools = normalized_tools(&input.goal, &input.tools, true)?;
        if input.tools.contains(&ToolId::Browser) && self.origins(project)?.is_empty() {
            return Err("approve website access in this project's task composer first".into());
        }
        let previous = input
            .automation_id
            .as_ref()
            .map(|id| self.get(project, id))
            .transpose()?;
        if let Some(previous) = &previous {
            if input.revision != Some(previous.revision()) {
                return Err("automation changed; refresh before saving".into());
            }
        }
        let id = input
            .automation_id
            .clone()
            .unwrap_or_else(|| format!("automation-{}", TaskId::generate()));
        let anchor = previous.as_ref().map_or(now, |a| a.created_at);
        let next_run_at = input.schedule.next_after(&input.timezone, anchor, now)?;
        if next_run_at.is_none() {
            return Err("schedule has no future occurrence".into());
        }
        input.automation_id = Some(id.clone());
        input.revision = Some(previous.as_ref().map_or(1, |a| a.revision() + 1));
        let automation = Automation {
            input,
            project_id: project.into(),
            created_at: anchor,
            next_run_at,
            runs: previous.map_or_else(Vec::new, |a| a.runs),
        };
        self.mutate(|s| {
            if s.automations.len() >= 1000 && !s.automations.contains_key(&id) {
                return Err("automation limit reached".into());
            }
            s.automations.insert(id, automation.clone());
            Ok(automation)
        })
    }
    pub fn toggle(
        &mut self,
        project: &str,
        id: &str,
        revision: u64,
        enabled: bool,
        now: i64,
    ) -> Result<(), String> {
        let automation = self.get(project, id)?;
        if automation.revision() != revision {
            return Err("automation changed; refresh before toggling".into());
        }
        if enabled {
            let mut candidate = automation.input.clone();
            candidate.enabled = true;
            candidate.validate(now)?;
        }
        self.mutate(|s| {
            let item = s.automations.get_mut(id).ok_or("automation disappeared")?;
            item.input.enabled = enabled;
            item.input.revision = Some(revision + 1);
            // Enabling is an explicit fresh start, not approval of a hidden backlog.
            if enabled {
                item.next_run_at =
                    item.input
                        .schedule
                        .next_after(&item.input.timezone, item.created_at, now)?;
            }
            Ok(())
        })
    }
    pub fn remove(&mut self, project: &str, id: &str, revision: u64) -> Result<(), String> {
        if self.get(project, id)?.revision() != revision {
            return Err("automation changed; refresh before deleting".into());
        }
        self.mutate(|s| {
            s.automations.remove(id);
            Ok(())
        })
    }
    /// Park a blocked definition without overwriting a newer user edit.
    pub fn pause_blocked_if_current(
        &mut self,
        project: &str,
        id: &str,
        revision: u64,
        reason: String,
    ) -> Result<bool, String> {
        if self.get(project, id)?.revision() != revision {
            return Ok(false);
        }
        self.mutate(|state| {
            let item = state
                .automations
                .get_mut(id)
                .ok_or("automation disappeared")?;
            item.input.enabled = false;
            item.input.revision = Some(revision + 1);
            item.runs.push(AutomationRun {
                occurrence_id: format!("blocked-{}", TaskId::generate()),
                scheduled_at: now(),
                task_id: None,
                status: "BLOCKED".into(),
                note: Some(reason),
            });
            trim_runs(&mut item.runs);
            Ok(true)
        })
    }

    /// At most one occurrence is admitted per tick; misses never create a burst.
    pub fn due(&mut self, now: i64) -> Result<Option<Automation>, String> {
        self.healthy()?;
        let mut items: Vec<_> = self
            .snapshot
            .automations
            .values()
            .filter(|a| {
                a.input.enabled
                    && a.input.authorization_until.is_some_and(|until| until > now)
                    && a.next_run_at.is_some_and(|at| at <= now)
            })
            .cloned()
            .collect();
        items.sort_by_key(|a| a.next_run_at);
        for item in items {
            let at = item.next_run_at.ok_or("due automation has no occurrence")?;
            // A 60-second dispatch tolerance is not a missed window. Beyond it,
            // the user's explicit skip/run-once policy decides what happens.
            if item.input.catch_up == CatchUp::Skip && now.saturating_sub(at) > 60 {
                let id = item.id()?.to_owned();
                let next =
                    item.input
                        .schedule
                        .next_after(&item.input.timezone, item.created_at, now)?;
                self.mutate(|s| {
                    let a = s.automations.get_mut(&id).ok_or("automation disappeared")?;
                    a.next_run_at = next;
                    a.runs.push(AutomationRun {
                        occurrence_id: format!("{id}:{at}"),
                        scheduled_at: at,
                        task_id: None,
                        status: "SKIPPED".into(),
                        note: Some("Missed occurrence skipped by schedule policy.".into()),
                    });
                    trim_runs(&mut a.runs);
                    Ok(())
                })?;
                continue;
            }
            return Ok(Some(item));
        }
        Ok(None)
    }
    /// Durable intent BEFORE task creation, dispatch, or external I/O. A crash
    /// between claim and binding parks this occurrence for review, not replay.
    #[allow(clippy::too_many_arguments)]
    pub fn claim(
        &mut self,
        project: &str,
        id: &str,
        occurrence: &str,
        at: i64,
        now: i64,
        manual: bool,
    ) -> Result<AutomationRun, String> {
        let item = self.get(project, id)?;
        if occurrence.len() > 300 {
            return Err("occurrence identity too long".into());
        }
        if !manual
            && (!item.input.enabled
                || item
                    .input
                    .authorization_until
                    .is_none_or(|until| until <= now))
        {
            return Err("automation is paused or its authorization expired".into());
        }
        if self.snapshot.admitted.contains(occurrence) {
            return Err(
                "occurrence was already admitted; inspect its task instead of replaying".into(),
            );
        }
        let next = if manual {
            item.next_run_at
        } else {
            item.input
                .schedule
                .next_after(&item.input.timezone, item.created_at, now)?
        };
        let run = AutomationRun {
            occurrence_id: occurrence.into(),
            scheduled_at: at,
            task_id: None,
            status: "CLAIMED".into(),
            note: None,
        };
        self.mutate(|s| {
            s.admitted.insert(occurrence.into());
            let automation = s.automations.get_mut(id).ok_or("automation disappeared")?;
            automation.next_run_at = next;
            automation.runs.push(run.clone());
            trim_runs(&mut automation.runs);
            Ok(run)
        })
    }
    pub fn bind_occurrence(
        &mut self,
        project: &str,
        id: &str,
        occurrence: &str,
        task: &str,
        options: TaskOptions,
    ) -> Result<(), String> {
        self.get(project, id)?;
        self.mutate(|s| {
            let run = s
                .automations
                .get_mut(id)
                .and_then(|a| a.runs.iter_mut().find(|r| r.occurrence_id == occurrence))
                .ok_or("claimed occurrence not found")?;
            if run.task_id.is_some() {
                return Err("occurrence already has a task".into());
            }
            run.task_id = Some(task.into());
            run.status = "QUEUED".into();
            s.options.insert(task.into(), options);
            Ok(())
        })
    }
    pub fn record_outcome(
        &mut self,
        task: &str,
        status: &str,
        note: Option<String>,
    ) -> Result<(), String> {
        self.mutate(|s| {
            for automation in s.automations.values_mut() {
                if let Some(run) = automation
                    .runs
                    .iter_mut()
                    .find(|r| r.task_id.as_deref() == Some(task))
                {
                    run.status = status.into();
                    run.note = note.clone();
                }
            }
            Ok(())
        })
    }
}
fn trim_runs(runs: &mut Vec<AutomationRun>) {
    if runs.len() > 100 {
        runs.drain(..runs.len() - 100);
    }
}
fn atomic_save(path: &Path, state: &Snapshot) -> Result<(), String> {
    let data = serde_json::to_vec(state).map_err(|e| e.to_string())?;
    if data.len() as u64 > MAX_STATE_BYTES {
        return Err("engagement state requires maintenance before more admissions".into());
    }
    let temp = path.with_extension(format!("{}.tmp", TaskId::generate()));
    let result = (|| -> Result<(), String> {
        let mut file = OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(&temp)
            .map_err(|e| e.to_string())?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            file.set_permissions(std::fs::Permissions::from_mode(0o600))
                .map_err(|e| e.to_string())?;
        }
        file.write_all(&data)
            .and_then(|()| file.sync_all())
            .map_err(|e| e.to_string())?;
        drop(file);
        std::fs::rename(&temp, path).map_err(|e| e.to_string())?;
        #[cfg(unix)]
        {
            File::open(path.parent().ok_or("no parent")?)
                .and_then(|dir| dir.sync_all())
                .map_err(|e| e.to_string())?;
        }
        Ok(())
    })();
    if result.is_err() {
        let _ = std::fs::remove_file(&temp);
    }
    result
}
pub fn validate_goal(goal: &str) -> Result<(), String> {
    if goal.trim().is_empty() || goal.len() > 65_536 {
        return Err("task goal is required and must be at most 65536 bytes".into());
    }
    Ok(())
}
pub fn validate_request(request: &CreateRequest) -> Result<(), String> {
    validate_goal(&request.goal)?;
    if request.request_id.is_empty()
        || request.request_id.len() > 128
        || request.project_id.is_empty()
    {
        return Err("valid project and request identities are required".into());
    }
    normalized_tools(&request.goal, &request.tools, false)?;
    Ok(())
}
fn request_digest(request: &CreateRequest) -> Result<String, String> {
    Ok(sha256_hex(
        &serde_json::to_vec(request).map_err(|e| e.to_string())?,
    ))
}
/// Normalize structured capability selections. Goal prose is deliberately not
/// parsed here: pasted documents, quoted text, code, webpages and model output
/// cannot grant a capability. The composer must send the structured selection.
pub fn normalized_tools(
    _goal: &str,
    tools: &[ToolId],
    unattended: bool,
) -> Result<Vec<ToolId>, String> {
    let mut selected: BTreeSet<_> = tools.iter().copied().collect();
    selected.insert(ToolId::Files);
    for tool in &selected {
        let descriptor = capability_descriptor(*tool);
        if !descriptor.implemented {
            return Err(format!(
                "{} is not implemented in this build",
                descriptor.authority_class
            ));
        }
        if unattended && !descriptor.unattended {
            return Err(format!(
                "{} cannot run unattended",
                descriptor.authority_class
            ));
        }
    }
    Ok(selected.into_iter().collect())
}
#[must_use]
pub fn now() -> i64 {
    Timestamp::now().epoch_seconds()
}

#[cfg(test)]
mod tests {
    use super::*;
    fn epoch(value: &str) -> i64 {
        DateTime::parse_from_rfc3339(value).unwrap().timestamp()
    }
    fn input() -> AutomationInput {
        AutomationInput {
            automation_id: None,
            revision: None,
            name: "Weekly summary".into(),
            goal: "Read inputs and create a summary".into(),
            tools: vec![ToolId::Files],
            schedule: ScheduleSpec::Every { minutes: 240 },
            timezone: "Asia/Ho_Chi_Minh".into(),
            catch_up: CatchUp::RunOnce,
            enabled: true,
            authorization_until: Some(2_000_000_000 + 86400),
        }
    }
    fn path() -> PathBuf {
        std::env::temp_dir()
            .join(format!("lumi-engagement-{}", TaskId::generate()))
            .join("state.json")
    }
    #[test]
    fn dst_gap_is_skipped_and_fold_is_not_duplicated() {
        let gap = ScheduleSpec::Daily {
            time: "02:30".into(),
        };
        assert_eq!(
            gap.next_after("America/New_York", 0, epoch("2026-03-08T06:59:00Z"))
                .unwrap(),
            Some(epoch("2026-03-09T06:30:00Z"))
        );
        let fold = ScheduleSpec::Daily {
            time: "01:30".into(),
        };
        let first = fold
            .next_after("America/New_York", 0, epoch("2026-11-01T04:00:00Z"))
            .unwrap()
            .unwrap();
        assert_eq!(first, epoch("2026-11-01T05:30:00Z"));
        assert_eq!(
            fold.next_after("America/New_York", 0, first).unwrap(),
            Some(epoch("2026-11-02T06:30:00Z"))
        );
    }
    #[test]
    fn invalid_schedules_and_unsupported_tools_are_refused() {
        assert!(ScheduleSpec::Every { minutes: 0 }
            .next_after("UTC", 0, 0)
            .is_err());
        assert!(ScheduleSpec::Daily {
            time: "25:00".into()
        }
        .next_after("UTC", 0, 0)
        .is_err());
        assert!(ScheduleSpec::Every { minutes: 5 }
            .next_after("Mars/Olympus", 0, 0)
            .is_err());
        assert_eq!(
            normalized_tools("@chrome do work", &[], false).unwrap(),
            vec![ToolId::Files]
        );
        assert_eq!(
            normalized_tools("@shell run", &[], true).unwrap(),
            vec![ToolId::Files]
        );
        assert!(normalized_tools("run", &[ToolId::Shell], true).is_err());
        assert_eq!(
            normalized_tools("@browser in pasted prose", &[], false).unwrap(),
            vec![ToolId::Files]
        );
        assert_eq!(
            normalized_tools("quoted @shell", &[ToolId::Browser], false).unwrap(),
            vec![ToolId::Files, ToolId::Browser]
        );
    }
    #[test]
    fn single_owner_and_restart_do_not_replay_claimed_occurrences() {
        let path = path();
        let mut store = EngagementStore::open(path.clone()).unwrap();
        assert!(EngagementStore::open(path.clone()).is_err());
        let created = store.save("project-a", input(), 2_000_000_000).unwrap();
        let id = created.id().unwrap().to_owned();
        store
            .claim(
                "project-a",
                &id,
                "occurrence-1",
                2_000_014_400,
                2_000_014_400,
                false,
            )
            .unwrap();
        assert!(store
            .claim(
                "project-a",
                &id,
                "occurrence-1",
                2_000_014_400,
                2_000_014_400,
                false
            )
            .is_err());
        drop(store);
        let store = EngagementStore::open(path.clone()).unwrap();
        let restored = store.get("project-a", &id).unwrap();
        assert!(!restored.input.enabled);
        assert_eq!(restored.runs[0].status, "INTERRUPTED");
        assert!(store.get("project-b", &id).is_err());
        drop(store);
        std::fs::remove_dir_all(path.parent().unwrap()).unwrap();
    }
    #[test]
    fn skip_advances_without_burst_and_expired_authority_is_not_admitted() {
        let path = path();
        let mut store = EngagementStore::open(path.clone()).unwrap();
        let mut definition = input();
        definition.catch_up = CatchUp::Skip;
        let created = store.save("project", definition, 2_000_000_000).unwrap();
        assert!(store.due(2_000_014_500).unwrap().is_none());
        let read = store.get("project", created.id().unwrap()).unwrap();
        assert_eq!(read.runs[0].status, "SKIPPED");
        assert_eq!(read.next_run_at, Some(2_000_028_800));
        assert!(store.due(2_000_100_000).unwrap().is_none());
        drop(store);
        std::fs::remove_dir_all(path.parent().unwrap()).unwrap();
    }
    #[test]
    fn duplicate_request_cannot_change_its_contents() {
        let path = path();
        let mut store = EngagementStore::open(path.clone()).unwrap();
        let mut request = CreateRequest {
            project_id: "p".into(),
            goal: "one\n\ntwo".into(),
            tools: vec![ToolId::Files],
            request_id: "request".into(),
        };
        store
            .attach_task(&request, "task", TaskOptions::files_only())
            .unwrap();
        assert_eq!(
            store.existing_request(&request).unwrap().as_deref(),
            Some("task")
        );
        request.goal = "different".into();
        assert!(store.existing_request(&request).is_err());
        drop(store);
        std::fs::remove_dir_all(path.parent().unwrap()).unwrap();
    }
}

#[cfg(test)]
mod blocked_schedule_tests {
    use super::*;
    #[test]
    fn a_blocker_is_visible_and_cannot_overwrite_a_newer_user_edit() {
        let path = std::env::temp_dir()
            .join(format!("lumi-blocked-{}", TaskId::generate()))
            .join("state.json");
        let mut store = EngagementStore::open(path.clone()).unwrap();
        let at = 2_000_000_000;
        let input = AutomationInput {
            automation_id: None,
            revision: None,
            name: "test".into(),
            goal: "Read project files".into(),
            tools: vec![ToolId::Files],
            schedule: ScheduleSpec::Every { minutes: 5 },
            timezone: "UTC".into(),
            catch_up: CatchUp::RunOnce,
            enabled: true,
            authorization_until: Some(at + 3600),
        };
        let created = store.save("project", input, at).unwrap();
        let id = created.id().unwrap().to_owned();
        assert!(store
            .pause_blocked_if_current(
                "project",
                &id,
                created.revision(),
                "Missing browser setup".into()
            )
            .unwrap());
        let parked = store.get("project", &id).unwrap();
        assert!(!parked.input.enabled);
        assert_eq!(parked.runs[0].status, "BLOCKED");
        assert!(!store
            .pause_blocked_if_current("project", &id, created.revision(), "Stale failure".into())
            .unwrap());
        assert_eq!(store.get("project", &id).unwrap().runs.len(), 1);
        drop(store);
        std::fs::remove_dir_all(path.parent().unwrap()).unwrap();
    }
}
