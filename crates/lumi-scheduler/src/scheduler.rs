//! The scheduler: schedules, missed-window policy, quiet hours, and
//! admission into the gate pipeline (spec 13 §13.8, §13.11–§13.13).

use crate::trigger::{DeduplicationLedger, TriggerRequest};
use lumi_protocol::{TaskId, Timestamp};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// A persisted schedule with explicit timezone/DST semantics (§13.11)
/// and an explicit catch-up policy (§13.12).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Schedule {
    pub schedule_id: ScheduleId,
    /// Cron-style expression (5-field, as-authored) — interpreted against
    /// `timezone`. The scheduler never silently converts to UTC-first.
    pub cron_expression: String,
    /// IANA timezone, e.g. `Europe/Berlin`. DST transitions are resolved
    /// in this zone each fire (§13.11).
    pub timezone: String,
    /// What to do about windows missed while the device was off/busy
    /// (§13.12). Default RUN_ONCE (§13.12: avoid burst duplicates).
    pub catch_up: CatchUpPolicy,
    /// Quiet hours in the schedule's timezone — starts suppressed in the
    /// window unless the trigger is marked critical (§13.8).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub quiet_hours: Option<QuietHours>,
    /// Critical triggers ignore quiet hours (e.g. security reconciliation).
    #[serde(default)]
    pub critical: bool,
    pub enabled: bool,
    /// The normalized trigger template this schedule fires.
    pub trigger: TriggerRequest,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum CatchUpPolicy {
    /// Missed windows are skipped entirely (default-adjacent, safest).
    Skip,
    /// At most ONE catch-up run for any number of missed windows
    /// (§13.12 default: avoid burst duplicate side effects).
    RunOnce,
    /// Run every missed window (requires the workflow to be idempotent
    /// AND the dedup window to be wide enough; use deliberately).
    RunEachMissed,
    /// Missed windows park in the human exception queue.
    RequireUser,
}

/// Quiet hours window (§13.8), hours in the schedule timezone.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct QuietHours {
    /// Local hour of day the quiet window starts (0–23).
    pub start_hour: u8,
    /// Local hour of day it ends (0–23); may wrap past midnight.
    pub end_hour: u8,
}

impl QuietHours {
    /// True when the given local hour falls inside the quiet window.
    #[must_use]
    pub fn contains_hour(&self, hour: u8) -> bool {
        if self.start_hour <= self.end_hour {
            (self.start_hour..self.end_hour).contains(&hour)
        } else {
            // Wraps midnight: e.g. 22..06
            hour >= self.start_hour || hour < self.end_hour
        }
    }
}

/// Stable schedule identifier.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct ScheduleId(pub String);

impl ScheduleId {
    #[must_use]
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }
}

/// Why a schedule firing was not admitted.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SchedulerError {
    /// No window was due at this time.
    NotDue,
    /// The trigger was admitted to catch-up handling instead of running.
    MissedWindow {
        policy: CatchUpPolicy,
        missed_windows: u32,
    },
    /// Quiet hours suppressed the start.
    QuietHours { until_hour: u8 },
    /// Duplicate within the dedup window (§13.4).
    Duplicate { key: String },
    /// Lease refused admission (§13.5).
    LeaseRefused { reason: String },
    /// Schedule disabled.
    Disabled,
    /// Device unavailable and rerouting is not permitted (§13.9).
    DeviceUnavailable,
}

impl std::fmt::Display for SchedulerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotDue => write!(f, "schedule not due"),
            Self::MissedWindow {
                policy,
                missed_windows,
            } => {
                write!(f, "{missed_windows} missed window(s) handled by {policy:?}")
            }
            Self::QuietHours { until_hour } => write!(f, "quiet hours until {until_hour}:00"),
            Self::Duplicate { key } => write!(f, "duplicate trigger dropped: {key}"),
            Self::LeaseRefused { reason } => write!(f, "lease refused: {reason}"),
            Self::Disabled => write!(f, "schedule disabled"),
            Self::DeviceUnavailable => {
                write!(f, "device unavailable and rerouting not permitted")
            }
        }
    }
}

impl std::error::Error for SchedulerError {}

/// A schedule firing admission: either a fresh task or an explicit
/// skip-with-reason (all skips are auditable, never silent).
#[derive(Debug, Clone, PartialEq)]
pub enum ScheduleAdmission {
    Admit {
        task_id: TaskId,
        trigger: Box<TriggerRequest>,
    },
    Skipped {
        reason: String,
    },
}

/// The scheduler: holds schedules + dedup ledger + device state, and
/// computes admissions. It is a TASK CREATOR, not an authority — the
/// created task still flows through the orchestrator gate.
pub struct Scheduler {
    schedules: BTreeMap<ScheduleId, Schedule>,
    dedup: DeduplicationLedger,
    /// Device availability state (set by the runtime from platform events).
    pub device_online: bool,
    /// Local-only flag for the deployment (§13.9).
    pub local_only: bool,
    task_counter: u64,
}

impl Scheduler {
    #[must_use]
    pub fn new(local_only: bool) -> Self {
        Self {
            schedules: BTreeMap::new(),
            dedup: DeduplicationLedger::new(),
            device_online: true,
            local_only,
            task_counter: 0,
        }
    }

    pub fn add_schedule(&mut self, schedule: Schedule) {
        self.schedules
            .insert(schedule.schedule_id.clone(), schedule);
    }

    pub fn enable(&mut self, schedule_id: &ScheduleId, enabled: bool) {
        if let Some(s) = self.schedules.get_mut(schedule_id) {
            s.enabled = enabled;
        }
    }

    /// Schedules whose next window is due: `due_at <= now`. Windows are
    /// expressed by the CALLER (the cron engine resolves the expression
    /// against the schedule timezone); the scheduler owns the POLICY —
    /// catch-up, quiet hours, dedup, lease, device.
    ///
    /// `windows_missed` counts due windows that elapsed while the device
    /// could not run (offline/busy) — the caller computes it from the
    /// cron expression + device state, the scheduler decides what to do.
    #[allow(clippy::too_many_arguments)]
    pub fn admit_firing(
        &mut self,
        schedule_id: &ScheduleId,
        now: Timestamp,
        local_hour: u8,
        windows_missed: u32,
        device_online: bool,
    ) -> Result<ScheduleAdmission, SchedulerError> {
        let Some(schedule) = self.schedules.get(schedule_id) else {
            return Err(SchedulerError::NotDue);
        };
        if !schedule.enabled {
            return Err(SchedulerError::Disabled);
        }

        // Quiet hours (§13.8): suppress non-critical starts.
        if !schedule.critical {
            if let Some(quiet) = &schedule.quiet_hours {
                if quiet.contains_hour(local_hour) {
                    return Err(SchedulerError::QuietHours {
                        until_hour: quiet.end_hour,
                    });
                }
            }
        }

        // Device availability (§13.9): wait or reroute WITHOUT weakening
        // locality. Local-only tasks can never go to the cloud.
        if !device_online {
            let may_reroute = schedule.trigger.device_availability
                == crate::trigger::DeviceAvailability::MayRerouteIfPolicyPermits
                && !schedule.trigger.cloud_reroute_forbidden(self.local_only);
            if !may_reroute {
                return Err(SchedulerError::DeviceUnavailable);
            }
        }

        // Missed windows (§13.12).
        if windows_missed > 1 {
            match schedule.catch_up {
                CatchUpPolicy::Skip => {
                    return Err(SchedulerError::MissedWindow {
                        policy: CatchUpPolicy::Skip,
                        missed_windows: windows_missed,
                    });
                }
                CatchUpPolicy::RunOnce => {
                    // Proceed with exactly ONE run (the normal path below).
                }
                CatchUpPolicy::RunEachMissed => {
                    // The caller loops admissions; admission here is one
                    // window's worth. No special handling.
                }
                CatchUpPolicy::RequireUser => {
                    return Err(SchedulerError::MissedWindow {
                        policy: CatchUpPolicy::RequireUser,
                        missed_windows: windows_missed,
                    });
                }
            }
        }

        // Deduplication (§13.4): duplicates are dropped BEFORE any task
        // creation.
        let trigger = schedule.trigger.clone();
        if let Some(key) = trigger.deduplication_key() {
            let window = schedule
                .trigger
                .deduplication
                .as_ref()
                .map(|d| d.window_seconds)
                .unwrap_or(0);
            if self.dedup.is_duplicate(&key, window, now) {
                return Err(SchedulerError::Duplicate { key });
            }
        }

        // Lease admission (§13.5) — enforcement lives in the lease
        // registry; the scheduler only records the requirement on the
        // task. An unauthorized lease fails the run at admission, not
        // mid-effect.
        self.task_counter += 1;
        let task_id = TaskId::parse(format!(
            "task-sched-{}-{now_epoch}",
            schedule.schedule_id.0,
            now_epoch = now.epoch_seconds()
        ))
        .map_err(|_| SchedulerError::NotDue)?;
        let _ = self.task_counter;

        Ok(ScheduleAdmission::Admit {
            task_id,
            trigger: Box::new(trigger),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::trigger::{DeduplicationWindow, DeviceAvailability, TriggerKind, TriggerSource};
    use lumi_protocol::TenantId;

    fn schedule(catch_up: CatchUpPolicy, quiet: Option<QuietHours>) -> Schedule {
        Schedule {
            schedule_id: ScheduleId::new("nightly-reconciliation"),
            cron_expression: "0 2 * * *".to_owned(),
            timezone: "Europe/Berlin".to_owned(),
            catch_up,
            quiet_hours: quiet,
            critical: false,
            enabled: true,
            trigger: TriggerRequest {
                tenant_id: TenantId::parse("t-1").unwrap(),
                principal_id: "u-schedule".to_owned(),
                template_id: "invoice-reconciliation".to_owned(),
                payload_refs: serde_json::json!({"invoice_ref": "webhook/payloads/42"}),
                triggered_at: Timestamp::UNIX_EPOCH,
                deduplication: Some(DeduplicationWindow {
                    key: "nightly".to_owned(),
                    window_seconds: 60,
                }),
                source: TriggerSource {
                    kind: TriggerKind::Cron,
                    identity: "cron:0 2 * * *".to_owned(),
                },
                timezone: Some("Europe/Berlin".to_owned()),
                device_availability: DeviceAvailability::WaitForDevice,
                deadline: None,
                max_actions: 20,
                lease_id: "lease-1".to_owned(),
            },
        }
    }

    fn scheduler(s: Schedule) -> Scheduler {
        let mut sched = Scheduler::new(false);
        sched.add_schedule(s);
        sched
    }

    #[test]
    fn quiet_hours_suppress_non_critical_starts() {
        let mut sched = scheduler(schedule(
            CatchUpPolicy::RunOnce,
            Some(QuietHours {
                start_hour: 22,
                end_hour: 6,
            }),
        ));
        // 23:00 local → quiet.
        let err = sched
            .admit_firing(
                &ScheduleId::new("nightly-reconciliation"),
                Timestamp::from_epoch(100, 0).unwrap(),
                23,
                1,
                true,
            )
            .unwrap_err();
        assert!(matches!(err, SchedulerError::QuietHours { until_hour: 6 }));
        // 07:00 local → allowed.
        assert!(sched
            .admit_firing(
                &ScheduleId::new("nightly-reconciliation"),
                Timestamp::from_epoch(100, 0).unwrap(),
                7,
                1,
                true
            )
            .is_ok());
    }

    #[test]
    fn critical_schedules_ignore_quiet_hours() {
        let mut s = schedule(
            CatchUpPolicy::RunOnce,
            Some(QuietHours {
                start_hour: 22,
                end_hour: 6,
            }),
        );
        s.critical = true;
        let mut sched = scheduler(s);
        let admission = sched
            .admit_firing(
                &ScheduleId::new("nightly-reconciliation"),
                Timestamp::from_epoch(100, 0).unwrap(),
                23,
                1,
                true,
            )
            .unwrap();
        assert!(matches!(admission, ScheduleAdmission::Admit { .. }));
    }

    #[test]
    fn missed_window_policies_diverge() {
        // SKIP: missed window refused.
        let mut sched = scheduler(schedule(CatchUpPolicy::Skip, None));
        assert!(matches!(
            sched.admit_firing(
                &ScheduleId::new("nightly-reconciliation"),
                Timestamp::from_epoch(100, 0).unwrap(),
                7,
                3,
                true
            ),
            Err(SchedulerError::MissedWindow {
                policy: CatchUpPolicy::Skip,
                missed_windows: 3
            })
        ));

        // REQUIRE_USER: parks in the human queue (same refusal shape).
        let mut sched = scheduler(schedule(CatchUpPolicy::RequireUser, None));
        assert!(matches!(
            sched.admit_firing(
                &ScheduleId::new("nightly-reconciliation"),
                Timestamp::from_epoch(100, 0).unwrap(),
                7,
                3,
                true
            ),
            Err(SchedulerError::MissedWindow {
                policy: CatchUpPolicy::RequireUser,
                ..
            })
        ));

        // RUN_ONCE (default): one admission for any number of missed.
        let mut sched = scheduler(schedule(CatchUpPolicy::RunOnce, None));
        let admission = sched
            .admit_firing(
                &ScheduleId::new("nightly-reconciliation"),
                Timestamp::from_epoch(100, 0).unwrap(),
                7,
                5,
                true,
            )
            .unwrap();
        assert!(matches!(admission, ScheduleAdmission::Admit { .. }));
    }

    #[test]
    fn device_offline_wait_policy_refuses_and_local_only_never_reroutes() {
        // WaitForDevice: device offline ⇒ wait (refuse with explicit reason).
        let mut sched = scheduler(schedule(CatchUpPolicy::RunOnce, None));
        assert!(matches!(
            sched.admit_firing(
                &ScheduleId::new("nightly-reconciliation"),
                Timestamp::from_epoch(100, 0).unwrap(),
                7,
                1,
                false
            ),
            Err(SchedulerError::DeviceUnavailable)
        ));

        // MayRerouteIfPolicyPermits + local-only deployment: STILL refused
        // (§13.9: locality is never weakened).
        let mut s = schedule(CatchUpPolicy::RunOnce, None);
        s.trigger.device_availability = DeviceAvailability::MayRerouteIfPolicyPermits;
        let mut sched = Scheduler::new(true); // local_only deployment
        sched.add_schedule(s);
        assert!(matches!(
            sched.admit_firing(
                &ScheduleId::new("nightly-reconciliation"),
                Timestamp::from_epoch(100, 0).unwrap(),
                7,
                1,
                false
            ),
            Err(SchedulerError::DeviceUnavailable)
        ));
    }

    #[test]
    fn duplicate_nightly_runs_are_dropped_by_dedup() {
        let mut sched = scheduler(schedule(CatchUpPolicy::RunOnce, None));
        // First fire admits...
        assert!(sched
            .admit_firing(
                &ScheduleId::new("nightly-reconciliation"),
                Timestamp::from_epoch(100, 0).unwrap(),
                7,
                1,
                true
            )
            .is_ok());
        // ...an immediate re-fire with the same dedup key is dropped.
        let err = sched
            .admit_firing(
                &ScheduleId::new("nightly-reconciliation"),
                Timestamp::from_epoch(110, 0).unwrap(),
                7,
                1,
                true,
            )
            .unwrap_err();
        assert!(matches!(err, SchedulerError::Duplicate { .. }), "{err:?}");
        // ...and after the dedup window it admits again.
        assert!(sched
            .admit_firing(
                &ScheduleId::new("nightly-reconciliation"),
                Timestamp::from_epoch(500, 0).unwrap(),
                7,
                1,
                true
            )
            .is_ok());
    }

    #[test]
    fn disabled_schedule_refuses() {
        let mut sched = scheduler(schedule(CatchUpPolicy::RunOnce, None));
        sched.enable(&ScheduleId::new("nightly-reconciliation"), false);
        assert!(matches!(
            sched.admit_firing(
                &ScheduleId::new("nightly-reconciliation"),
                Timestamp::from_epoch(100, 0).unwrap(),
                7,
                1,
                true
            ),
            Err(SchedulerError::Disabled)
        ));
    }
}
