//! Managed browser reading exposed as an AgentTool through the existing gate.
//! Native/Chrome-session control is intentionally not advertised as operational.
use lumi_agent::tools::{AgentTool, ToolContext, ToolError};
use lumi_browser::{BrowserOp, BrowserWorkerConfig, BrowserWorkerHandle, WorkerResult};
use lumi_protocol::{
    ActionId, ActionProposal, Capability, ExecutionResult, ExecutionStatus, ExpectedEffect,
    FailureCategory, Grounding, ResourceRef, ResourceType, RiskClass, RunId, SensitivityLabel,
    Target, Timestamp,
};
use lumi_state::CancelToken;
use serde_json::{json, Value};
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use url::{Host, Url};

#[derive(Debug, Clone)]
pub struct BrowserConfig {
    pub script: PathBuf,
}
impl BrowserConfig {
    /// Explicit host configuration, or the source worker in development. The
    /// bytes must match the worker embedded at build time; editing a project
    /// cannot replace a live privileged worker without rebuilding the host.
    pub fn discover(resource_dir: Option<&Path>) -> Option<Self> {
        let explicit = std::env::var_os("LUMI_BROWSER_WORKER").map(PathBuf::from);
        let bundled = resource_dir.map(|p| p.join("playwright/readonly-worker.js"));
        let development = cfg!(debug_assertions).then(|| {
            Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../../workers/playwright/readonly-worker.js")
        });
        [explicit, bundled, development]
            .into_iter()
            .flatten()
            .find(|p| p.is_file())
            .map(|script| Self { script })
    }
    pub fn verify(&self) -> Result<(), String> {
        let worker =
            std::fs::read(&self.script).map_err(|_| "managed browser worker is not installed")?;
        let helper = std::fs::read(self.script.with_file_name("readonly-network.js"))
            .map_err(|_| "managed browser network helper is not installed")?;
        if worker != include_bytes!("../../../workers/playwright/readonly-worker.js")
            || helper != include_bytes!("../../../workers/playwright/readonly-network.js")
        {
            return Err(
                "browser worker differs from this build; rebuild or reinstall the matching worker"
                    .into(),
            );
        }
        Ok(())
    }
    pub fn launch(&self, origins: &[String]) -> Result<BrowserWorkerHandle, String> {
        self.verify()?;
        let mut config =
            BrowserWorkerConfig::playwright(self.script.to_string_lossy().into_owned());
        config.args = vec![serde_json::to_string(origins).map_err(|e| e.to_string())?];
        config.working_dir = self.script.parent().map(Path::to_path_buf);
        BrowserWorkerHandle::spawn(&config).map_err(|_| {
            "could not start the browser worker; install the matching Node/Playwright runtime"
                .into()
        })
    }
    pub fn probe(&self) -> Result<(), String> {
        let handle = self.launch(&[])?;
        let response = handle.request(BrowserOp::Navigate { url: "about:blank".into(), wait_until: None, trace_path: None }, Duration::from_secs(25))
            .map_err(|_| "browser setup check failed; verify Node, the pinned Playwright package, and its Chromium installation")?;
        if !response.ok {
            return Err("browser setup check failed; the managed browser did not open".into());
        }
        Ok(())
    }
}
pub fn normalize_origins(origins: &[String]) -> Result<Vec<String>, String> {
    if origins.len() > 32 {
        return Err("approve at most 32 website origins per project".into());
    }
    let mut normalized = BTreeSet::new();
    for raw in origins {
        let url = public_url(raw)?;
        if url.path() != "/" || url.query().is_some() || url.fragment().is_some() {
            return Err(
                "website access uses exact HTTPS origins, without a path, query, or fragment"
                    .into(),
            );
        }
        normalized.insert(url.origin().ascii_serialization());
    }
    Ok(normalized.into_iter().collect())
}
fn public_url(raw: &str) -> Result<Url, String> {
    if raw.len() > 4096 {
        return Err("browser URL is too long".into());
    }
    let url = Url::parse(raw).map_err(|_| "invalid browser URL")?;
    let domain = match url.host() {
        Some(Host::Domain(domain)) => domain,
        _ => return Err("browser IP literals are not allowed".into()),
    };
    if url.scheme() != "https"
        || !url.username().is_empty()
        || url.password().is_some()
        || !domain.contains('.')
        || domain.ends_with(".local")
        || domain.ends_with(".localhost")
        || domain.ends_with(".internal")
    {
        return Err("only public HTTPS websites without embedded credentials are supported".into());
    }
    Ok(url)
}
fn scoped_url(raw: &str, origins: &[String]) -> Result<String, String> {
    let mut url = public_url(raw)?;
    if !origins.contains(&url.origin().ascii_serialization()) {
        return Err("browser URL is outside this project's approved origins".into());
    }
    url.set_fragment(None);
    Ok(url.to_string())
}
struct BrowserSession {
    handle: Option<BrowserWorkerHandle>,
    observed_urls: BTreeSet<String>,
}
pub struct BrowserReadTool {
    config: BrowserConfig,
    origins: Vec<String>,
    session: Mutex<BrowserSession>,
    cancel: CancelToken,
    revoked: Arc<AtomicBool>,
    expires_at: i64,
}
impl BrowserReadTool {
    pub fn new(
        config: BrowserConfig,
        origins: Vec<String>,
        goal: &str,
        cancel: CancelToken,
        revoked: Arc<AtomicBool>,
        expires_at: i64,
    ) -> Result<Self, String> {
        let origins = normalize_origins(&origins)?;
        if origins.is_empty() {
            return Err("browser use requires approved website origins".into());
        }
        let mut observed_urls: BTreeSet<_> =
            origins.iter().map(|origin| format!("{origin}/")).collect();
        // Only direct user-provided URLs, approved roots, and subsequently
        // observed links may be navigated. The model cannot construct a URL
        // containing file contents and disguise that export as a page read.
        for (start, _) in goal.match_indices("https://") {
            let candidate: String = goal[start..]
                .chars()
                .take_while(|c| !c.is_whitespace() && !['\"', '\'', '<', '>', ')', ']'].contains(c))
                .collect();
            if let Ok(url) = scoped_url(candidate.trim_end_matches([',', ';']), &origins) {
                observed_urls.insert(url);
            }
        }
        Ok(Self {
            config,
            origins,
            session: Mutex::new(BrowserSession {
                handle: None,
                observed_urls,
            }),
            cancel,
            revoked,
            expires_at,
        })
    }
    fn check(&self, url: &str) -> Result<String, String> {
        if self.cancel.is_cancelled()
            || self.revoked.load(Ordering::SeqCst)
            || crate::engagement::now() >= self.expires_at
        {
            return Err("browser authority was stopped, revoked, or expired".into());
        }
        let normalized = scoped_url(url, &self.origins)?;
        if !self
            .session
            .lock()
            .map_err(|_| "browser session lock poisoned")?
            .observed_urls
            .contains(&normalized)
        {
            return Err("URL was neither supplied by the user nor observed as a page link; constructed navigation is refused".into());
        }
        Ok(normalized)
    }
    fn read(&self, url: &str) -> Result<Value, String> {
        let url = self.check(url)?;
        let handle = {
            let mut session = self
                .session
                .lock()
                .map_err(|_| "browser session lock poisoned")?;
            if session.handle.is_none() {
                session.handle = Some(self.config.launch(&self.origins)?);
            }
            session
                .handle
                .as_ref()
                .ok_or("browser handle missing")?
                .clone()
        };
        let _stop = StopMonitor::new(
            handle.clone(),
            self.cancel.clone(),
            Arc::clone(&self.revoked),
            self.expires_at,
        );
        let navigated = handle
            .request(
                BrowserOp::Navigate {
                    url,
                    wait_until: None,
                    trace_path: None,
                },
                Duration::from_secs(25),
            )
            .map_err(|e| format!("browser navigation failed: {e:?}"))?;
        if !navigated.ok {
            return Err(
                "approved website could not be read; inspect browser setup and network access"
                    .into(),
            );
        }
        let read = handle
            .request(BrowserOp::Read { selector: None }, Duration::from_secs(20))
            .map_err(|e| format!("browser observation failed: {e:?}"))?;
        if !read.ok {
            return Err("browser did not return a page observation".into());
        }
        let Some(WorkerResult::Json(result)) = read.result else {
            return Err("browser returned an incompatible observation".into());
        };
        let observation = result
            .get("observation")
            .ok_or("browser observation is missing")?;
        let observed_url = scoped_url(
            observation
                .get("url")
                .and_then(Value::as_str)
                .ok_or("observation URL missing")?,
            &self.origins,
        )?;
        let text = observation
            .get("text")
            .and_then(Value::as_str)
            .ok_or("observation text missing")?;
        let links: Vec<String> = observation
            .get("links")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .filter_map(Value::as_str)
            .filter_map(|link| scoped_url(link, &self.origins).ok())
            .take(24)
            .collect();
        let mut session = self
            .session
            .lock()
            .map_err(|_| "browser session lock poisoned")?;
        session.observed_urls.insert(observed_url.clone());
        for link in &links {
            if session.observed_urls.len() < 1024 {
                session.observed_urls.insert(link.clone());
            }
        }
        Ok(
            json!({ "source": "browser", "trust": "untrusted_page_content", "url": observed_url,
            "title": observation.get("title").and_then(Value::as_str).unwrap_or(""),
            "text": text.chars().take(16_000).collect::<String>(), "links": links }),
        )
    }
}
impl AgentTool for BrowserReadTool {
    fn name(&self) -> &'static str {
        "browser_read"
    }
    fn description(&self) -> &'static str {
        "Read an approved public website in the isolated browser. Use an exact user-provided URL, an approved origin root, or a link returned by an earlier browser_read. No sign-in, forms, JavaScript, arbitrary URLs, or personal browser sessions."
    }
    fn parameters(&self) -> Value {
        json!({"type":"object","properties":{"url":{"type":"string"}},"required":["url"],"additionalProperties":false})
    }
    fn capability(&self) -> Capability {
        Capability::well_known(lumi_protocol::capabilities::BROWSER_READ)
    }
    fn risk_class(&self) -> RiskClass {
        RiskClass::Read
    }
    fn build_proposal(
        &self,
        arguments: Value,
        ctx: &ToolContext<'_>,
        run_id: &RunId,
    ) -> Result<ActionProposal, ToolError> {
        let raw = arguments
            .get("url")
            .and_then(Value::as_str)
            .ok_or_else(|| ToolError("browser_read requires a URL".into()))?;
        if arguments.as_object().is_none_or(|object| object.len() != 1) {
            return Err(ToolError("browser_read accepts only a URL".into()));
        }
        let url = self.check(raw).map_err(ToolError)?;
        let origin = Url::parse(&url)
            .map_err(|e| ToolError(e.to_string()))?
            .origin()
            .ascii_serialization();
        ActionProposal::builder(
            ActionId::generate(),
            ctx.task_id.clone(),
            run_id.clone(),
            ctx.principal.clone(),
            self.capability(),
            ResourceRef {
                resource_type: ResourceType::well_known(ResourceType::BROWSER_ORIGIN),
                id: origin,
                sensitivity: Some(SensitivityLabel::Public),
            },
            Target::canonical(url.clone()),
            self.name(),
            RiskClass::Read,
        )
        .arguments(json!({"url":url}))
        .expected_effect(ExpectedEffect {
            summary: "Read the approved page and return a bounded, untrusted observation".into(),
            external_visibility: false,
            reversible: true,
        })
        .build()
        .map_err(|e| ToolError(e.to_string()))
    }
    fn execute(&self, action: &ActionProposal, _ctx: &ToolContext<'_>) -> ExecutionResult {
        let started = Timestamp::now();
        let outcome = self.read(
            action
                .arguments
                .get("url")
                .and_then(Value::as_str)
                .unwrap_or(""),
        );
        let cancelled = self.cancel.is_cancelled()
            || self.revoked.load(Ordering::SeqCst)
            || crate::engagement::now() >= self.expires_at;
        let (status, observations, error) = if cancelled {
            (ExecutionStatus::Cancelled, vec![], None)
        } else {
            match outcome {
                Ok(observation) => (
                    ExecutionStatus::Success,
                    vec![observation.to_string()],
                    None,
                ),
                Err(message) => (
                    ExecutionStatus::Failed,
                    vec![],
                    Some(lumi_protocol::ErrorEnvelope::new(
                        FailureCategory::BrowserState,
                        message,
                    )),
                ),
            }
        };
        ExecutionResult {
            action_id: action.action_id.clone(),
            task_id: action.task_id.clone(),
            run_id: action.run_id.clone(),
            status,
            started_at: started,
            ended_at: Timestamp::now(),
            grounding: Some(Grounding::SemanticLocator),
            observation_ids: observations,
            error,
        }
    }
}
struct StopMonitor {
    done: Arc<AtomicBool>,
    thread: Option<std::thread::JoinHandle<()>>,
}
impl StopMonitor {
    fn new(
        handle: BrowserWorkerHandle,
        cancel: CancelToken,
        revoked: Arc<AtomicBool>,
        expires_at: i64,
    ) -> Self {
        let done = Arc::new(AtomicBool::new(false));
        let flag = Arc::clone(&done);
        let thread = std::thread::spawn(move || {
            while !flag.load(Ordering::SeqCst) {
                if cancel.is_cancelled()
                    || revoked.load(Ordering::SeqCst)
                    || crate::engagement::now() >= expires_at
                {
                    handle.kill();
                    break;
                }
                std::thread::sleep(Duration::from_millis(20));
            }
        });
        Self {
            done,
            thread: Some(thread),
        }
    }
}
impl Drop for StopMonitor {
    fn drop(&mut self) {
        self.done.store(true, Ordering::SeqCst);
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}
/// Wrap every selected tool, including files, with the same expiring/revocable
/// run authority. The policy grant is checked too; this is defense in depth.
pub struct BoundedTool {
    pub inner: Box<dyn AgentTool>,
    pub revoked: Arc<AtomicBool>,
    pub expires_at: i64,
}
impl BoundedTool {
    fn allowed(&self) -> bool {
        !self.revoked.load(Ordering::SeqCst) && crate::engagement::now() < self.expires_at
    }
}
impl AgentTool for BoundedTool {
    fn name(&self) -> &'static str {
        self.inner.name()
    }
    fn description(&self) -> &'static str {
        self.inner.description()
    }
    fn parameters(&self) -> Value {
        self.inner.parameters()
    }
    fn capability(&self) -> Capability {
        self.inner.capability()
    }
    fn risk_class(&self) -> RiskClass {
        self.inner.risk_class()
    }
    fn build_proposal(
        &self,
        args: Value,
        ctx: &ToolContext<'_>,
        run: &RunId,
    ) -> Result<ActionProposal, ToolError> {
        if !self.allowed() {
            return Err(ToolError("run authority expired or was revoked".into()));
        }
        self.inner.build_proposal(args, ctx, run)
    }
    fn execute(&self, action: &ActionProposal, ctx: &ToolContext<'_>) -> ExecutionResult {
        if self.allowed() {
            return self.inner.execute(action, ctx);
        }
        ExecutionResult {
            action_id: action.action_id.clone(),
            task_id: action.task_id.clone(),
            run_id: action.run_id.clone(),
            status: ExecutionStatus::Cancelled,
            started_at: Timestamp::now(),
            ended_at: Timestamp::now(),
            grounding: None,
            observation_ids: vec![],
            error: None,
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn origin_scope_is_exact_and_cannot_hide_credentials_or_local_access() {
        assert_eq!(
            normalize_origins(&["https://example.com/".into()]).unwrap(),
            vec!["https://example.com"]
        );
        for origin in [
            "http://example.com",
            "https://user:secret@example.com",
            "https://127.0.0.1",
            "https://a.local",
            "file:///etc/passwd",
            "https://example.com/path",
        ] {
            assert!(normalize_origins(&[origin.into()]).is_err(), "{origin}");
        }
        assert!(scoped_url(
            "https://example.com.evil.test/",
            &["https://example.com".into()]
        )
        .is_err());
    }
    #[test]
    fn modified_worker_is_refused_before_it_can_start() {
        let path =
            std::env::temp_dir().join(format!("lumi-worker-{}", lumi_protocol::TaskId::generate()));
        std::fs::create_dir_all(&path).unwrap();
        std::fs::write(path.join("readonly-worker.js"), "modified").unwrap();
        std::fs::write(path.join("readonly-network.js"), "modified").unwrap();
        assert!(BrowserConfig {
            script: path.join("readonly-worker.js")
        }
        .verify()
        .is_err());
        std::fs::remove_dir_all(path).unwrap();
    }
}
