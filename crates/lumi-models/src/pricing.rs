//! Model pricing and cost accounting in micro-USD.
//!
//! Canonical integer cost unit for all Lumi economics (spec 16): prices
//! are micro-USD per 1K tokens so providers' per-million pricing converts
//! exactly.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

use crate::response::Usage;

/// Price for one model, in micro-USD per 1K tokens.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModelPrice {
    pub input_per_1k: u64,
    pub output_per_1k: u64,
}

/// Known price table; unknown models cost nothing recorded (surfaced as
/// None so callers cannot silently under-count).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct PriceTable {
    prices: BTreeMap<String, ModelPrice>,
}

impl PriceTable {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    pub fn set(&mut self, model: &str, price: ModelPrice) {
        self.prices.insert(model.to_owned(), price);
    }

    /// Cost of `usage` in micro-USD, when the model is priced.
    #[must_use]
    pub fn cost_micro_usd(&self, model: &str, usage: &Usage) -> Option<u64> {
        let price = self.prices.get(model)?;
        let input = u128::from(usage.input_tokens) * u128::from(price.input_per_1k) / 1_000;
        let output = u128::from(usage.output_tokens) * u128::from(price.output_per_1k) / 1_000;
        u64::try_from(input + output).ok()
    }

    #[must_use]
    pub fn is_known(&self, model: &str) -> bool {
        self.prices.contains_key(model)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn micro_usd_math() {
        let mut table = PriceTable::new();
        // $3 / 1M input = 3000 micro-USD per 1K.
        table.set(
            "gpt-4o-mini",
            ModelPrice {
                input_per_1k: 150,
                output_per_1k: 600,
            },
        );
        let usage = Usage {
            input_tokens: 2_000,
            output_tokens: 500,
        };
        // 2 * 150 + 0.5 * 600 = 600 micro-USD.
        assert_eq!(table.cost_micro_usd("gpt-4o-mini", &usage), Some(600));
        assert_eq!(table.cost_micro_usd("unknown", &usage), None);
    }
}
