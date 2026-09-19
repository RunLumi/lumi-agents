//! Canonical UTC RFC 3339 timestamps (spec 01 §1.14).
//!
//! Persisted timestamps MUST be UTC RFC 3339. This module implements a
//! dependency-free `Timestamp` that stores seconds + nanoseconds since the
//! Unix epoch, serializes as `YYYY-MM-DDTHH:MM:SS[.fff…]Z`, and parses the
//! full RFC 3339 grammar (including numeric offsets, which are normalized to
//! UTC). Ordering is by instant, not by string.

use core::cmp::Ordering;
use core::time::Duration;
use serde::{de, Deserialize, Deserializer, Serialize, Serializer};

/// Maximum nanosecond component of a timestamp.
const NANOS_PER_SECOND: u32 = 1_000_000_000;
/// Unix epoch day for 1970-01-01 (civil conversion base).
const SECONDS_IN_DAY: i64 = 86_400;

/// An instant on the UTC timeline.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Timestamp {
    /// Whole seconds since 1970-01-01T00:00:00Z (may be negative).
    seconds: i64,
    /// Sub-second component, always in `0..1_000_000_000`.
    nanos: u32,
}

impl Timestamp {
    /// Unix epoch (1970-01-01T00:00:00Z).
    pub const UNIX_EPOCH: Self = Self {
        seconds: 0,
        nanos: 0,
    };

    /// Builds a timestamp from epoch components.
    ///
    /// # Errors
    /// Returns `ProtocolError::Malformed`-style message via `String` when
    /// `nanos >= 1_000_000_000`. Exposed as `new` for constructor
    /// ergonomics; callers handling untrusted input should use
    /// [`Timestamp::parse`] instead.
    #[must_use]
    pub fn from_epoch(seconds: i64, nanos: u32) -> Option<Self> {
        if nanos >= NANOS_PER_SECOND {
            return None;
        }
        // Normalize negative-second carry so `nanos` is always non-negative.
        if seconds < 0 && nanos > 0 {
            Some(Self {
                seconds: seconds + 1,
                nanos: nanos - NANOS_PER_SECOND,
            })
        } else {
            Some(Self { seconds, nanos })
        }
    }

    /// Current wall-clock time.
    #[must_use]
    pub fn now() -> Self {
        let now = std::time::SystemTime::now();
        match now.duration_since(std::time::UNIX_EPOCH) {
            Ok(d) => Self {
                seconds: d.as_secs() as i64,
                nanos: d.subsec_nanos(),
            },
            Err(e) => {
                let d = e.duration();
                Self {
                    seconds: -(d.as_secs() as i64),
                    nanos: 0,
                }
            }
        }
    }

    /// Parses an RFC 3339 timestamp and normalizes it to UTC.
    ///
    /// # Errors
    /// Returns a human-readable error for any input outside RFC 3339.
    pub fn parse(input: &str) -> Result<Self, String> {
        let bytes = input.as_bytes();
        if bytes.len() < 20 {
            return Err(format!("timestamp too short: {input:?}"));
        }
        // YYYY-MM-DD
        let year: i64 = input
            .get(0..4)
            .ok_or("missing year")?
            .parse()
            .map_err(|_| "bad year")?;
        if input.as_bytes()[4] != b'-' {
            return Err("expected '-' after year".into());
        }
        let month: u32 = input
            .get(5..7)
            .ok_or("missing month")?
            .parse()
            .map_err(|_| "bad month")?;
        if input.as_bytes()[7] != b'-' {
            return Err("expected '-' after month".into());
        }
        let day: u32 = input
            .get(8..10)
            .ok_or("missing day")?
            .parse()
            .map_err(|_| "bad day")?;
        // Separator: 'T' or 't' per RFC 3339.
        let sep = bytes[10];
        if sep != b'T' && sep != b't' && sep != b' ' {
            return Err("expected 'T' date/time separator".into());
        }
        // HH:MM:SS
        let hour: u32 = input
            .get(11..13)
            .ok_or("missing hour")?
            .parse()
            .map_err(|_| "bad hour")?;
        if input.as_bytes()[13] != b':' {
            return Err("expected ':' after hour".into());
        }
        let minute: u32 = input
            .get(14..16)
            .ok_or("missing minute")?
            .parse()
            .map_err(|_| "bad minute")?;
        if input.as_bytes()[16] != b':' {
            return Err("expected ':' after minute".into());
        }
        let second: u32 = input
            .get(17..19)
            .ok_or("missing second")?
            .parse()
            .map_err(|_| "bad second")?;
        if hour > 23 {
            return Err(format!("hour out of range: {hour}"));
        }
        if minute > 59 {
            return Err(format!("minute out of range: {minute}"));
        }
        if second > 59 {
            // RFC 3339 permits second == 60 only for leap seconds; we reject
            // because correct epoch mapping requires a leap-second table.
            return Err(format!(
                "second out of range (leap seconds unsupported): {second}"
            ));
        }
        let mut rest = &input[19..];
        let mut nanos: u32 = 0;
        let mut carry_second: i64 = 0;
        if let Some(frac) = rest.strip_prefix('.') {
            let digits_end = frac
                .find(|c: char| !c.is_ascii_digit())
                .unwrap_or(frac.len());
            let digits = &frac[..digits_end];
            if digits.is_empty() {
                return Err("empty fractional seconds".into());
            }
            rest = &frac[digits_end..];
            // Take at most 9 digits, rounding (not truncating) the remainder.
            let mut value: u64 = 0;
            let mut scale: u64 = 100_000_000;
            for (i, ch) in digits.bytes().enumerate() {
                let digit = u64::from(ch - b'0');
                if i < 9 {
                    value += digit * scale;
                    scale /= 10;
                } else if i == 9 && digit >= 5 {
                    value += 1;
                    break;
                }
            }
            if value >= u64::from(NANOS_PER_SECOND) {
                value -= u64::from(NANOS_PER_SECOND);
                carry_second = 1;
            }
            nanos = value as u32;
        }
        // Offset: Z / z / +HH:MM / -HH:MM
        let offset_seconds: i64 = match rest {
            "Z" | "z" => 0,
            _ => {
                let offset_bytes = rest.as_bytes();
                if offset_bytes.len() != 6
                    || (offset_bytes[0] != b'+' && offset_bytes[0] != b'-')
                    || offset_bytes[3] != b':'
                {
                    return Err(format!("invalid UTC offset: {rest:?}"));
                }
                let oh: u32 = rest
                    .get(1..3)
                    .ok_or("bad offset hour")?
                    .parse()
                    .map_err(|_| "bad offset hour")?;
                let om: u32 = rest
                    .get(4..6)
                    .ok_or("bad offset minute")?
                    .parse()
                    .map_err(|_| "bad offset minute")?;
                if oh > 23 || om > 59 {
                    return Err("offset out of range".into());
                }
                let magnitude = i64::from(oh) * 3600 + i64::from(om) * 60;
                if offset_bytes[0] == b'-' {
                    -magnitude
                } else {
                    magnitude
                }
            }
        };

        let days = days_from_civil(year, month, day)?;
        let mut seconds = days * SECONDS_IN_DAY
            + i64::from(hour) * 3600
            + i64::from(minute) * 60
            + i64::from(second);
        seconds -= offset_seconds;
        seconds += carry_second;
        Ok(Self { seconds, nanos })
    }

    /// Formats as RFC 3339 UTC with `Z` designator. Fractional digits are
    /// emitted only when nonzero, with trailing zeros trimmed.
    #[must_use]
    pub fn to_rfc3339(&self) -> String {
        let days = self.seconds.div_euclid(SECONDS_IN_DAY);
        let rem = self.seconds.rem_euclid(SECONDS_IN_DAY);
        let (year, month, day) = civil_from_days(days);
        let hour = rem / 3600;
        let minute = (rem % 3600) / 60;
        let second = rem % 60;
        let mut out = format!("{year:04}-{month:02}-{day:02}T{hour:02}:{minute:02}:{second:02}");
        if self.nanos > 0 {
            let mut frac = format!("{:09}", self.nanos);
            while frac.ends_with('0') {
                frac.pop();
            }
            out.push('.');
            out.push_str(&frac);
        }
        out.push('Z');
        out
    }

    /// Instant elapsed since this timestamp, saturating at zero for future
    /// instants.
    #[must_use]
    pub fn duration_since(&self, earlier: &Self) -> Duration {
        let mut secs = self.seconds.saturating_sub(earlier.seconds);
        let mut nanos = i64::from(self.nanos) - i64::from(earlier.nanos);
        if nanos < 0 {
            secs = secs.saturating_sub(1);
            nanos += i64::from(NANOS_PER_SECOND);
        }
        if secs <= 0 {
            return Duration::ZERO;
        }
        Duration::new(secs as u64, nanos as u32)
    }

    /// Adds a duration, saturating at the representable bounds.
    #[must_use]
    pub fn checked_add(&self, d: Duration) -> Option<Self> {
        let seconds = self.seconds.checked_add(i64::try_from(d.as_secs()).ok()?)?;
        let nanos = self.nanos + d.subsec_nanos();
        if nanos >= NANOS_PER_SECOND {
            Some(Self {
                seconds: seconds.checked_add(1)?,
                nanos: nanos - NANOS_PER_SECOND,
            })
        } else {
            Some(Self { seconds, nanos })
        }
    }

    /// Epoch seconds (may be negative).
    #[must_use]
    pub const fn epoch_seconds(&self) -> i64 {
        self.seconds
    }

    /// Sub-second nanoseconds (0..1_000_000_000).
    #[must_use]
    pub const fn nanoseconds(&self) -> u32 {
        self.nanos
    }
}

impl PartialOrd for Timestamp {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Timestamp {
    fn cmp(&self, other: &Self) -> Ordering {
        self.seconds
            .cmp(&other.seconds)
            .then(self.nanos.cmp(&other.nanos))
    }
}

impl Serialize for Timestamp {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.to_rfc3339())
    }
}

impl<'de> Deserialize<'de> for Timestamp {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let s = String::deserialize(deserializer)?;
        Self::parse(&s).map_err(de::Error::custom)
    }
}

impl std::fmt::Display for Timestamp {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.to_rfc3339())
    }
}

/// Days since 1970-01-01 for a proleptic-Gregorian civil date.
fn days_from_civil(year: i64, month: u32, day: u32) -> Result<i64, String> {
    if !(1..=12).contains(&month) {
        return Err(format!("month out of range: {month}"));
    }
    if day == 0 || day > days_in_month(year, month) {
        return Err(format!("day out of range: {day} for {year:04}-{month:02}"));
    }
    let y = if month <= 2 { year - 1 } else { year };
    let era = y.div_euclid(400);
    let yoe = y - era * 400; // [0, 399]
    let m = i64::from(month);
    let mp = (m + 9) % 12; // Mar=0 … Feb=11
    let doy = (153 * mp + 2) / 5 + i64::from(day) - 1; // [0, 365]
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy; // [0, 146096]
    Ok(era * 146_097 + doe - 719_468)
}

/// Inverse of [`days_from_civil`].
fn civil_from_days(days: i64) -> (i64, u32, u32) {
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097; // [0, 146096]
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365; // [0, 399]
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100); // [0, 365]
    let mp = (5 * doy + 2) / 153; // [0, 11]
    let d = doy - (153 * mp + 2) / 5 + 1; // [1, 31]
    let m = if mp < 10 { mp + 3 } else { mp - 9 }; // [1, 12]
    let year = if m <= 2 { y + 1 } else { y };
    (year, m as u32, d as u32)
}

fn is_leap_year(year: i64) -> bool {
    (year % 4 == 0 && year % 100 != 0) || year % 400 == 0
}

fn days_in_month(year: i64, month: u32) -> u32 {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if is_leap_year(year) => 29,
        2 => 28,
        _ => 0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_and_formats_utc() {
        let t = Timestamp::parse("2026-09-18T12:34:56Z").unwrap();
        assert_eq!(t.to_rfc3339(), "2026-09-18T12:34:56Z");
    }

    #[test]
    fn normalizes_offsets_to_utc() {
        let t = Timestamp::parse("2026-09-18T15:34:56+03:00").unwrap();
        assert_eq!(t.to_rfc3339(), "2026-09-18T12:34:56Z");
        let t2 = Timestamp::parse("2026-09-18T06:34:56-06:00").unwrap();
        assert_eq!(t2.to_rfc3339(), "2026-09-18T12:34:56Z");
    }

    #[test]
    fn parses_fractional_seconds() {
        let t = Timestamp::parse("2026-09-18T12:34:56.123456789Z").unwrap();
        assert_eq!(t.nanoseconds(), 123_456_789);
        assert_eq!(t.to_rfc3339(), "2026-09-18T12:34:56.123456789Z");
        let trimmed = Timestamp::parse("2026-09-18T12:34:56.100Z").unwrap();
        assert_eq!(trimmed.to_rfc3339(), "2026-09-18T12:34:56.1Z");
    }

    #[test]
    fn rejects_leap_second_and_out_of_range() {
        assert!(Timestamp::parse("2026-06-30T23:59:60Z").is_err());
        assert!(Timestamp::parse("2026-02-30T00:00:00Z").is_err());
        assert!(Timestamp::parse("2026-13-01T00:00:00Z").is_err());
        assert!(Timestamp::parse("2026-09-18 12:34:56").is_err());
    }

    #[test]
    fn ordering_is_instant_based() {
        let a = Timestamp::parse("2026-09-18T12:34:56.5Z").unwrap();
        // Same instant expressed with a +01:00 offset.
        let b = Timestamp::parse("2026-09-18T13:34:56.5+01:00").unwrap();
        assert_eq!(a, b);
        let earlier = Timestamp::parse("2026-09-18T12:34:56Z").unwrap();
        assert!(earlier < a);
    }

    #[test]
    fn duration_and_checked_add() {
        let a = Timestamp::parse("2026-09-18T12:00:00Z").unwrap();
        let b = Timestamp::parse("2026-09-18T12:01:30.5Z").unwrap();
        assert_eq!(b.duration_since(&a), Duration::new(90, 500_000_000));
        assert_eq!(a.duration_since(&b), Duration::ZERO);
        let plus = a.checked_add(Duration::new(90, 500_000_000)).unwrap();
        assert_eq!(plus, b);
        assert_eq!(Timestamp::UNIX_EPOCH.to_rfc3339(), "1970-01-01T00:00:00Z");
    }

    #[test]
    fn epoch_boundaries() {
        assert_eq!(
            Timestamp::parse("1970-01-01T00:00:00Z")
                .unwrap()
                .epoch_seconds(),
            0
        );
        assert_eq!(
            Timestamp::parse("1969-12-31T23:59:59Z")
                .unwrap()
                .epoch_seconds(),
            -1
        );
        assert_eq!(
            Timestamp::from_epoch(0, 0).unwrap().to_rfc3339(),
            "1970-01-01T00:00:00Z"
        );
        assert!(Timestamp::from_epoch(0, NANOS_PER_SECOND).is_none());
    }

    #[test]
    fn leap_year_handling() {
        assert!(Timestamp::parse("2024-02-29T00:00:00Z").is_ok());
        assert!(Timestamp::parse("2026-02-29T00:00:00Z").is_err());
        assert!(Timestamp::parse("2000-02-29T00:00:00Z").is_ok());
        assert!(Timestamp::parse("1900-02-29T00:00:00Z").is_err());
    }

    #[test]
    fn serde_round_trip() {
        let t = Timestamp::parse("2026-09-18T12:34:56.5Z").unwrap();
        let json = serde_json::to_string(&t).unwrap();
        assert_eq!(json, "\"2026-09-18T12:34:56.5Z\"");
        let back: Timestamp = serde_json::from_str(&json).unwrap();
        assert_eq!(back, t);
    }
}
