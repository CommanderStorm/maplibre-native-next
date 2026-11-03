// Ports the helpers of include/mln/util/feature.hpp (`numericValue`,
// `featureIDtoString`). `mln::Feature` arrives with queries.

use crate::string::{to_string_double, to_string_i64, to_string_u64};
use maplibre_native_base::{Identifier, Value};

/// `numericValue<double>(const Value&)`.
pub fn numeric_value_f64(value: &Value) -> Option<f64> {
    match value {
        Value::Uint(t) => Some(*t as f64),
        Value::Int(t) => Some(*t as f64),
        Value::Double(t) => Some(*t),
        _ => None,
    }
}

/// `featureIDtoString(const FeatureIdentifier&)`.
pub fn feature_id_to_string(id: &Identifier) -> Option<String> {
    match id {
        Identifier::Null => None,
        Identifier::String(value) => Some(value.clone()),
        Identifier::Uint(value) => Some(to_string_u64(*value)),
        Identifier::Int(value) => Some(to_string_i64(*value)),
        Identifier::Double(value) => Some(to_string_double(*value)),
    }
}
