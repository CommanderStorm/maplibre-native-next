// Ports include/mln/style/expression/is_constant.hpp and
// src/mln/style/expression/is_constant.cpp.

use crate::compound_expression::CompoundExpression;
use crate::expression::{Expression, Kind, downcast};

pub fn is_global_property_constant(expression: &dyn Expression, properties: &[&str]) -> bool {
    if expression.get_kind() == Kind::CompoundExpression {
        let e = downcast::<CompoundExpression>(expression).expect("CompoundExpression");
        for property in properties {
            if e.get_operator() == *property {
                return false;
            }
        }
    }

    let mut is_constant = true;
    expression.each_child(&mut |e| {
        if is_constant && !is_global_property_constant(e, properties) {
            is_constant = false;
        }
    });
    is_constant
}

const FILTER: &str = "filter-";

pub fn is_feature_constant(expression: &dyn Expression) -> bool {
    if expression.get_kind() == Kind::CompoundExpression {
        let e = downcast::<CompoundExpression>(expression).expect("CompoundExpression");
        let name = e.get_operator();
        let parameter_count = e.get_parameter_count();
        if name == "get" && parameter_count == Some(1) {
            return false;
        } else if name == "has" && parameter_count == Some(1) {
            return false;
        } else if name == "properties" || name == "geometry-type" || name == "id" || name == "feature-state" {
            return false;
        } else if name.starts_with(FILTER) {
            // Legacy filters begin with "filter-" and are never constant.
            return false;
        }
    }

    if expression.get_kind() == Kind::FormatSectionOverride {
        return false;
    }

    if expression.get_kind() == Kind::Within {
        return false;
    }

    if expression.get_kind() == Kind::Distance {
        return false;
    }

    if expression.get_kind() == Kind::CollatorExpression {
        // Although the results of a Collator expression with fixed arguments
        // generally shouldn't change between executions, we can't serialize
        // them as constant expressions because results change based on
        // environment.
        return false;
    }

    let mut feature_constant = true;
    expression.each_child(&mut |e| {
        if feature_constant && !is_feature_constant(e) {
            feature_constant = false;
        }
    });
    feature_constant
}

const ZOOM_PROPERTY: [&str; 1] = ["zoom"];

pub fn is_zoom_constant(e: &dyn Expression) -> bool {
    is_global_property_constant(e, &ZOOM_PROPERTY)
}

/// Returns true if expression does not depend on information provided by the
/// runtime.
pub fn is_runtime_constant(expression: &dyn Expression) -> bool {
    if expression.get_kind() == Kind::ImageExpression {
        return false;
    }

    if expression.get_kind() == Kind::CompoundExpression {
        let e = downcast::<CompoundExpression>(expression).expect("CompoundExpression");
        if e.get_operator() == "global-state" {
            // The result depends on the style's global state, which can be
            // changed at runtime, so the expression is not runtime constant.
            return false;
        }
    }

    let mut runtime_constant = true;
    expression.each_child(&mut |e| {
        if runtime_constant && !is_runtime_constant(e) {
            runtime_constant = false;
        }
    });
    runtime_constant
}
