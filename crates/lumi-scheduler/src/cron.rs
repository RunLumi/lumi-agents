//! Minimal 5-field cron matching (Spec 13 §13.11) — the scheduler owns
//! the POLICY; this module only resolves expressions into wall-clock
//! field matches. Supports `*`, lists `a,b`, ranges `a-b`, steps
//! `*/n` / `a-b/n`, and names nothing (numbers only). Fields: minute
//! (0–59) hour (0–23) day-of-month (1–31) month (1–12) day-of-week
//! (0–6, Sunday = 0). Standard cron DOM/DOW OR-rule applies when both
//! are restricted (vixie cron semantics).
//!
//! Timezones: callers convert their wall clock to fields themselves —
//! the expression is matched against the fields of the schedule's
//! timezone (§13.11), never silently against UTC.

/// Wall-clock fields for one candidate minute.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CronFields {
    pub minute: u8,
    pub hour: u8,
    pub day_of_month: u8,
    pub month: u8,
    pub day_of_week: u8,
}

/// One parsed cron field: the set of matching values it accepts.
#[derive(Debug, Clone, PartialEq, Eq)]
struct CronField {
    values: Vec<u8>,
}

impl CronField {
    fn parse(spec: &str, min: u8, max: u8) -> Result<CronField, String> {
        let mut values = Vec::new();
        for part in spec.split(',') {
            let (range, step) = match part.split_once('/') {
                Some((r, s)) => {
                    let step: u32 = s.parse().map_err(|_| format!("bad step {s:?}"))?;
                    if step == 0 {
                        return Err(format!("step must be >= 1 in {part:?}"));
                    }
                    (r, step as usize)
                }
                None => (part, 1),
            };
            let (lo, hi) = if range == "*" {
                (min as usize, max as usize)
            } else if let Some((a, b)) = range.split_once('-') {
                let a: usize = a.parse().map_err(|_| format!("bad range {range:?}"))?;
                let b: usize = b.parse().map_err(|_| format!("bad range {range:?}"))?;
                (a, b)
            } else {
                let v: usize = range.parse().map_err(|_| format!("bad value {range:?}"))?;
                (v, v)
            };
            if lo < min as usize || hi > max as usize || lo > hi {
                return Err(format!("range {lo}-{hi} outside {min}-{max}"));
            }
            let mut v = lo;
            while v <= hi {
                values.push(v as u8);
                v += step;
            }
        }
        values.sort_unstable();
        values.dedup();
        Ok(CronField { values })
    }

    fn matches(&self, value: u8) -> bool {
        self.values.binary_search(&value).is_ok()
    }
}

/// A parsed 5-field cron expression.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CronExpr {
    minute: CronField,
    hour: CronField,
    day_of_month: CronField,
    month: CronField,
    day_of_week: CronField,
    /// True when both DOM and DOW are restricted (`*` counts as
    /// unrestricted) — standard cron then matches on EITHER.
    dom_dow_or: bool,
}

impl CronExpr {
    /// Parses a 5-field cron expression.
    ///
    /// # Errors
    /// Wrong field count or malformed values.
    pub fn parse(expr: &str) -> Result<CronExpr, String> {
        let fields: Vec<&str> = expr.split_whitespace().collect();
        if fields.len() != 5 {
            return Err(format!(
                "cron expects 5 fields, got {}: {expr:?}",
                fields.len()
            ));
        }
        let minute = CronField::parse(fields[0], 0, 59)?;
        let hour = CronField::parse(fields[1], 0, 23)?;
        let day_of_month = CronField::parse(fields[2], 1, 31)?;
        let month = CronField::parse(fields[3], 1, 12)?;
        let day_of_week = CronField::parse(fields[4], 0, 6)?;
        let dom_restricted = fields[2] != "*";
        let dow_restricted = fields[4] != "*";
        Ok(CronExpr {
            minute,
            hour,
            day_of_month,
            month,
            day_of_week,
            dom_dow_or: dom_restricted && dow_restricted,
        })
    }

    /// True when the fields match this expression.
    #[must_use]
    pub fn matches(&self, f: &CronFields) -> bool {
        if !self.minute.matches(f.minute)
            || !self.hour.matches(f.hour)
            || !self.month.matches(f.month)
        {
            return false;
        }
        // vixie-cron DOM/DOW rule: when BOTH are restricted, the field
        // matches if EITHER does; otherwise both must match.
        let dom_ok = self.day_of_month.matches(f.day_of_month);
        let dow_ok = self.day_of_week.matches(f.day_of_week);
        if self.dom_dow_or {
            dom_ok || dow_ok
        } else {
            dom_ok && dow_ok
        }
    }
}

/// Civil wall-clock fields from epoch seconds plus a UTC offset
/// (dependency-free; standard days-from-civil conversion).
#[must_use]
pub fn civil_from_epoch(epoch_seconds: i64, utc_offset_seconds: i32) -> CronFields {
    let local = epoch_seconds + i64::from(utc_offset_seconds);
    let days = local.div_euclid(86_400);
    let secs_of_day = local.rem_euclid(86_400);
    let minute = (secs_of_day / 60) % 60;
    let hour = secs_of_day / 3600;
    // Howard Hinnant's civil_from_days.
    let z = days + 719_468;
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    // 1970-01-01 was a Thursday (dow 4 in the 0=Sunday convention).
    let dow = (days.rem_euclid(7) + 4) % 7;
    CronFields {
        minute: minute as u8,
        hour: hour as u8,
        day_of_month: d as u8,
        month: m as u8,
        day_of_week: dow as u8,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_and_matches_basic_patterns() {
        let e = CronExpr::parse("0 2 * * *").unwrap();
        assert!(e.matches(&CronFields {
            minute: 0,
            hour: 2,
            day_of_month: 15,
            month: 7,
            day_of_week: 3
        }));
        assert!(!e.matches(&CronFields {
            minute: 1,
            hour: 2,
            day_of_month: 15,
            month: 7,
            day_of_week: 3
        }));
    }

    #[test]
    fn steps_and_ranges() {
        let e = CronExpr::parse("*/15 8-10 * * 1-5").unwrap();
        assert!(e.matches(&CronFields {
            minute: 30,
            hour: 9,
            day_of_month: 15,
            month: 7,
            day_of_week: 3
        }));
        assert!(!e.matches(&CronFields {
            minute: 10,
            hour: 9,
            day_of_month: 15,
            month: 7,
            day_of_week: 3
        }));
        assert!(!e.matches(&CronFields {
            minute: 30,
            hour: 9,
            day_of_month: 15,
            month: 7,
            day_of_week: 0
        }));
    }

    #[test]
    fn lists_and_single_values() {
        let e = CronExpr::parse("0 9 1,15 * *").unwrap();
        assert!(e.matches(&CronFields {
            minute: 0,
            hour: 9,
            day_of_month: 1,
            month: 3,
            day_of_week: 4
        }));
        assert!(e.matches(&CronFields {
            minute: 0,
            hour: 9,
            day_of_month: 15,
            month: 3,
            day_of_week: 4
        }));
        assert!(!e.matches(&CronFields {
            minute: 0,
            hour: 9,
            day_of_month: 2,
            month: 3,
            day_of_week: 4
        }));
    }

    #[test]
    fn rejects_malformed() {
        assert!(CronExpr::parse("* * * *").is_err());
        assert!(CronExpr::parse("61 * * * *").is_err());
        assert!(CronExpr::parse("* */0 * * *").is_err());
    }

    #[test]
    fn civil_conversion_known_instant() {
        // 2026-09-20T12:00:00Z is a Sunday.
        let f = civil_from_epoch(1_789_905_600, 0);
        assert_eq!(
            (f.minute, f.hour, f.day_of_month, f.month, f.day_of_week),
            (0, 12, 20, 9, 0)
        );
    }

    #[test]
    fn offset_shifts_local_fields() {
        // 2026-09-20T23:30:00Z at UTC+7 is 2026-09-21T06:30 local.
        let f = civil_from_epoch(1_789_947_000, 7 * 3600);
        assert_eq!((f.day_of_month, f.hour, f.minute), (21, 6, 30));
    }
}
