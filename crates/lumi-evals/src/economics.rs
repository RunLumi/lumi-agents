//! Workflow economics (spec 16 §16.7): decision metrics, not marketing.
//!
//! All money values are integer micro-USD; minutes are integer; derived
//! values use u128 intermediates so real-world magnitudes never overflow.

use serde::{Deserialize, Serialize};

pub const YEAR_MONTHS: u32 = 12;

/// The measured/declared economic inputs for one workflow (§16.7).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkflowEconomics {
    pub runs_per_month: u32,
    /// Measured manual minutes per run before automation.
    pub baseline_manual_minutes_per_run: u32,
    /// Measured human minutes still required per automated run
    /// (approvals, exception handling).
    pub residual_human_minutes_per_run: u32,
    /// Fully-loaded labor cost per hour, micro-USD (e.g. $60/h =
    /// 60_000_000).
    pub loaded_labor_cost_per_hour_micro_usd: u64,
    /// Measured variable runtime cost per run, micro-USD.
    pub runtime_variable_cost_per_run_micro_usd: u64,
    /// Monthly support burden, micro-USD.
    pub support_cost_monthly_micro_usd: u64,
    /// One-off implementation cost, micro-USD.
    pub implementation_cost_micro_usd: u64,
    /// Observed end-to-end cycle times.
    pub cycle_time_before_minutes: u32,
    pub cycle_time_after_minutes: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct DerivedEconomics {
    /// Gross labor value released per month, micro-USD.
    pub gross_labor_value_monthly_micro_usd: u64,
    /// Net of runtime + support costs, micro-USD.
    pub net_value_monthly_micro_usd: u64,
    /// implementation_cost / net monthly value (rounded up; saturates at
    /// u32::MAX when net value is non-positive).
    pub simple_payback_months: u32,
}

impl WorkflowEconomics {
    /// Computes the derived economics per spec 16 §16.7 formulas.
    ///
    /// These are decision metrics (§16.7): garbage inputs produce garbage
    /// outputs, which is why every input must be measured, not invented.
    #[must_use]
    pub fn derived(&self) -> DerivedEconomics {
        // gross = runs * (baseline - residual) / 60 * hourly
        let minutes_released = i64::from(self.baseline_manual_minutes_per_run)
            - i64::from(self.residual_human_minutes_per_run);
        let gross: i128 = if minutes_released <= 0 {
            0
        } else {
            i128::from(self.runs_per_month)
                * i128::from(minutes_released as u32)
                * i128::from(self.loaded_labor_cost_per_hour_micro_usd)
                / 60
        };
        let runtime_cost_monthly = i128::from(self.runtime_variable_cost_per_run_micro_usd)
            * i128::from(self.runs_per_month);
        let net = gross - runtime_cost_monthly - i128::from(self.support_cost_monthly_micro_usd);
        let payback = if net <= 0 {
            u32::MAX
        } else {
            u32::try_from((i128::from(self.implementation_cost_micro_usd) + net - 1) / net)
                .unwrap_or(u32::MAX)
        };
        DerivedEconomics {
            gross_labor_value_monthly_micro_usd: u64::try_from(gross).unwrap_or(u64::MAX),
            net_value_monthly_micro_usd: u64::try_from(i128::max(net, 0)).unwrap_or(u64::MAX),
            simple_payback_months: payback,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn economics() -> WorkflowEconomics {
        WorkflowEconomics {
            runs_per_month: 1_000,
            baseline_manual_minutes_per_run: 12,
            residual_human_minutes_per_run: 1,
            loaded_labor_cost_per_hour_micro_usd: 60_000_000, // $60/h
            runtime_variable_cost_per_run_micro_usd: 20_000,  // $0.02
            support_cost_monthly_micro_usd: 50_000_000,       // $50
            implementation_cost_micro_usd: 180_000_000_000,   // $180k
            cycle_time_before_minutes: 12,
            cycle_time_after_minutes: 3,
        }
    }

    #[test]
    fn payback_math_matches_spec_formulas() {
        let derived = economics().derived();
        // gross = 1000 * 11 / 60 * 60_000_000 = 11_000_000_000 ($11k)
        assert_eq!(derived.gross_labor_value_monthly_micro_usd, 11_000_000_000);
        // net = 11_000_000_000 - 20_000_000 - 50_000_000 = 10_930_000_000
        assert_eq!(derived.net_value_monthly_micro_usd, 10_930_000_000);
        // payback = ceil(180_000_000_000 / 10_930_000_000) = 17
        assert_eq!(derived.simple_payback_months, 17);
    }

    #[test]
    fn negative_value_saturates_payback() {
        let mut e = economics();
        e.residual_human_minutes_per_run = e.baseline_manual_minutes_per_run;
        e.support_cost_monthly_micro_usd = 1_000_000_000;
        let derived = e.derived();
        assert_eq!(derived.gross_labor_value_monthly_micro_usd, 0);
        assert_eq!(derived.net_value_monthly_micro_usd, 0);
        assert_eq!(derived.simple_payback_months, u32::MAX);
    }

    #[test]
    fn no_released_minutes_means_no_value() {
        let mut e = economics();
        e.residual_human_minutes_per_run = 15; // more than baseline
        let derived = e.derived();
        assert_eq!(derived.gross_labor_value_monthly_micro_usd, 0);
    }
}
