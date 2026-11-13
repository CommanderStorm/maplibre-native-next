//! Style expressions: their types and values, parsing, and evaluation.
//!
//! Each expression has its own module. `type`, `match`, `let` and `in` are
//! Rust keywords, so those modules are `type_`, `match_`, `let_` and `in_`.

pub(crate) mod assertion;
pub(crate) mod at;
pub mod boolean_operator;
pub mod case;
pub(crate) mod check_subtype;
pub(crate) mod coalesce;
pub(crate) mod coercion;
pub(crate) mod collator;
pub(crate) mod collator_expression;
pub(crate) mod comparison;
pub mod compound_expression;
pub(crate) mod distance;
pub mod dsl;
pub(crate) mod error;
pub mod expression;
pub(crate) mod find_zoom_curve;
pub mod format_expression;
pub mod formatted;
pub mod image;
pub mod image_expression;
pub(crate) mod in_;
pub(crate) mod index_of;
pub(crate) mod interpolate;
pub(crate) mod interpolator;
pub(crate) mod is_constant;
pub mod is_expression;
pub(crate) mod length;
pub(crate) mod let_;
pub(crate) mod literal;
mod locale;
pub mod match_;
pub(crate) mod number_format;
pub mod parsing_context;
pub mod range;
pub(crate) mod semiliteral;
pub(crate) mod slice;
pub(crate) mod step;
pub(crate) mod stops;
pub mod type_;
pub(crate) mod utf8_op_helpers;
pub(crate) mod util;
pub mod value;
pub(crate) mod within;

pub use expression::{
    Dependency, EvaluationContext, EvaluationResult, Expression, ExpressionBase, Kind,
    downcast,
};
pub use find_zoom_curve::{ZoomCurvePtr, find_zoom_curve_checked};
pub use formatted::{Formatted, FormattedSection};
pub use image::Image;
pub use interpolate::{Interpolate, create_interpolate};
pub use interpolator::Interpolator;
pub use is_constant::{feature_property_keys, is_feature_constant, is_runtime_constant, is_zoom_constant};
pub use is_expression::is_expression;
pub use literal::Literal;
pub use parsing_context::{ParseResult, ParsingContext, TypeAnnotationOption};
pub use step::Step;
pub use stops::Stops;
pub use type_::Type;
pub use value::{Value, ValueConverter, from_expression_value};

#[cfg(test)]
mod tests;
