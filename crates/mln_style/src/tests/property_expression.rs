// Ports test/style/property_expression.test.cpp.

use super::util::{StubGeometryTileFeature, property_map};
use crate::expression::dsl::{get, get_str, image, interpolate, linear, literal, literal_str, number, step, zoom};
use crate::expression::{Dependency, Image, Value};
use crate::property_expression::PropertyExpression;
use crate::property_value::PropertyValue;
use crate::renderer::property_evaluation_parameters::PropertyEvaluationParameters;
use crate::renderer::property_evaluator::PropertyEvaluator;
use maplibre_native_base::Value as MbglValue;
use std::collections::BTreeSet;

fn one_integer() -> StubGeometryTileFeature {
    StubGeometryTileFeature::new(property_map(&[("property", MbglValue::Uint(1))]))
}

fn one_double() -> StubGeometryTileFeature {
    StubGeometryTileFeature::new(property_map(&[("property", MbglValue::Double(1.0))]))
}

fn one_string() -> StubGeometryTileFeature {
    StubGeometryTileFeature::new(property_map(&[("property", MbglValue::String("1".into()))]))
}

fn one_image() -> StubGeometryTileFeature {
    StubGeometryTileFeature::new(property_map(&[("image_name", MbglValue::String("maki-11".into()))]))
}

fn empty_tile_feature() -> StubGeometryTileFeature {
    StubGeometryTileFeature::new(property_map(&[]))
}

fn evaluate(value: PropertyValue<f32>, zoom: f32) -> f32 {
    let parameters = PropertyEvaluationParameters::new(zoom);
    value.evaluate(&PropertyEvaluator::new(&parameters, 0.0))
}

fn num(v: f64) -> Box<dyn crate::expression::Expression> {
    literal(Value::Number(v))
}

#[test]
fn constant() {
    // TEST(PropertyExpression, Constant)
    assert_eq!(2.0_f32, evaluate(PropertyValue::Constant(2.0), 0.0));
    assert_eq!(3.8_f32, evaluate(PropertyValue::Constant(3.8), 0.0));
    assert_eq!(22.0_f32, evaluate(PropertyValue::Constant(22.0), 0.0));
    assert_eq!(2.0_f32, evaluate(PropertyValue::Constant(2.0), 4.0));
    assert_eq!(3.8_f32, evaluate(PropertyValue::Constant(3.8), 4.0));
    assert_eq!(22.0_f32, evaluate(PropertyValue::Constant(22.0), 4.0));
    assert_eq!(2.0_f32, evaluate(PropertyValue::Constant(2.0), 22.0));
    assert_eq!(3.8_f32, evaluate(PropertyValue::Constant(3.8), 22.0));
    assert_eq!(22.0_f32, evaluate(PropertyValue::Constant(22.0), 22.0));

    assert_eq!(Dependency::NONE, PropertyValue::Constant(2.0_f32).get_dependencies());
}

#[test]
fn expression() {
    // TEST(PropertyExpression, Expression)
    let expression = PropertyExpression::<f32>::from_box(
        interpolate(linear(), zoom(), vec![(0.0, num(0.0)), (1.0, num(1.0))]),
        None,
    );
    assert_eq!(0.0, evaluate(PropertyValue::Expression(expression.clone()), 0.0));
    assert_eq!(0.5, evaluate(PropertyValue::Expression(expression.clone()), 0.5));
    assert_eq!(Dependency::ZOOM, expression.get_dependencies());
}

#[test]
fn defaults() {
    // TEST(PropertyExpression, Defaults)
    let no_default = PropertyExpression::<f32>::from_box(number(get_str("property"), None), None);
    let with_default = PropertyExpression::<f32>::from_box(number(get_str("property"), None), Some(0.0));
    assert_eq!(1.0_f32, with_default.evaluate_feature(&one_integer(), 2.0));
    assert_eq!(1.0_f32, with_default.evaluate_feature(&one_double(), 2.0));
    assert_eq!(0.0_f32, with_default.evaluate_feature(&one_string(), 2.0));
    assert_eq!(2.0_f32, no_default.evaluate_feature(&one_string(), 2.0));

    assert_eq!(Dependency::FEATURE, no_default.get_dependencies());
}

/// interpolate(linear(), number(get("property")), 1.0, literal(v))
fn inner(v: f64) -> Box<dyn crate::expression::Expression> {
    interpolate(linear(), number(get_str("property"), None), vec![(1.0, num(v))])
}

#[test]
fn zoom_interpolation() {
    // TEST(PropertyExpression, ZoomInterpolation)
    assert_eq!(
        40.0_f32,
        PropertyExpression::<f32>::from_box(
            interpolate(linear(), zoom(), vec![(0.0, inner(24.0)), (1.5, inner(36.0)), (3.0, inner(48.0))]),
            Some(0.0),
        )
        .evaluate_zoom_feature(2.0, &one_integer(), -1.0),
        "Should interpolate between stops"
    );

    assert_eq!(
        33.0_f32,
        PropertyExpression::<f32>::from_box(
            interpolate(linear(), zoom(), vec![(5.0, inner(33.0)), (10.0, inner(66.0))]),
            Some(0.0),
        )
        .evaluate_zoom_feature(0.0, &one_integer(), -1.0),
        "Use first stop output for input values from -inf to first stop"
    );

    assert_eq!(
        66.0_f32,
        PropertyExpression::<f32>::from_box(
            interpolate(linear(), zoom(), vec![(0.0, inner(33.0)), (10.0, inner(66.0))]),
            Some(0.0),
        )
        .evaluate_zoom_feature(20.0, &one_integer(), -1.0),
        "Use last stop output for input values from last stop to +inf"
    );

    assert_eq!(
        66.0_f32,
        PropertyExpression::<f32>::from_box(
            interpolate(linear(), zoom(), vec![(0.0, inner(33.0)), (10.0, inner(66.0))]),
            Some(0.0),
        )
        .evaluate_zoom_feature(10.0, &one_integer(), -1.0),
        "Should interpolate TO the last stop."
    );

    assert_eq!(
        33.0_f32,
        PropertyExpression::<f32>::from_box(
            interpolate(linear(), zoom(), vec![(0.0, inner(33.0)), (10.0, inner(66.0))]),
            Some(0.0),
        )
        .evaluate_zoom_feature(0.0, &one_integer(), -1.0),
        "Should interpolate TO the first stop"
    );
}

fn assert_near(expected: f32, actual: f32, tolerance: f32) {
    assert!((expected - actual).abs() <= tolerance, "expected {expected} ± {tolerance}, got {actual}");
}

#[test]
fn issue8460() {
    // TEST(PropertyExpression, Issue8460)
    let fn1 = PropertyExpression::<f32>::from_box(
        interpolate(linear(), zoom(), vec![(15.0, inner(0.0)), (15.2, inner(600.0))]),
        Some(0.0),
    );

    assert_near(0.0, fn1.evaluate_zoom_feature(15.0, &one_integer(), -1.0), 0.00);
    assert_near(300.0, fn1.evaluate_zoom_feature(15.1, &one_integer(), -1.0), 0.01);
    assert_near(600.0, fn1.evaluate_zoom_feature(15.2, &one_integer(), -1.0), 0.00);
    assert_near(600.0, fn1.evaluate_zoom_feature(16.0, &one_integer(), -1.0), 0.00);

    let fn2 = PropertyExpression::<f32>::from_box(
        interpolate(
            linear(),
            zoom(),
            vec![(15.0, inner(0.0)), (15.2, inner(300.0)), (18.0, inner(600.0))],
        ),
        Some(0.0),
    );

    assert_near(0.0, fn2.evaluate_zoom_feature(15.0, &one_integer(), -1.0), 0.00);
    assert_near(150.0, fn2.evaluate_zoom_feature(15.1, &one_integer(), -1.0), 0.01);
    assert_near(300.0, fn2.evaluate_zoom_feature(15.2, &one_integer(), -1.0), 0.00);
    assert_near(385.71, fn2.evaluate_zoom_feature(16.0, &one_integer(), -1.0), 0.01);
    assert_near(600.0, fn2.evaluate_zoom_feature(18.0, &one_integer(), -1.0), 0.00);
    assert_near(600.0, fn2.evaluate_zoom_feature(19.0, &one_integer(), -1.0), 0.00);
}

// TODO(port): TEST(PropertyExpression, FormatSectionOverride) needs
// FormatSectionOverride (renderer, M5).

#[test]
fn image_expression() {
    // TEST(PropertyExpression, ImageExpression)
    let empty_set = BTreeSet::new();
    let available_images: BTreeSet<String> =
        ["maki-11", "airport-11", "bicycle-15"].iter().map(|s| s.to_string()).collect();

    // evaluation test without available images
    {
        let prop_expr = PropertyExpression::<Image>::from_box(image(literal_str("airport-11")), None);
        let evaluated_image = prop_expr.evaluate_feature_images(&empty_tile_feature(), &empty_set, Image::default());
        assert!(!evaluated_image.is_available());
        assert_eq!(evaluated_image.id(), "airport-11");

        assert_eq!(Dependency::IMAGE, prop_expr.get_dependencies());

        let dd_prop_expr = PropertyExpression::<Image>::from_box(image(get(literal_str("image_name"))), None);
        let evaluated_image = dd_prop_expr.evaluate_feature_images(&one_image(), &empty_set, Image::default());
        assert!(!evaluated_image.is_available());
        assert_eq!(evaluated_image.id(), "maki-11");

        let evaluated_image = dd_prop_expr.evaluate_feature_images(&empty_tile_feature(), &empty_set, Image::default());
        assert!(!evaluated_image.is_available());
        assert_eq!(evaluated_image.id(), "");
    }
    // evaluation test with available images
    {
        let prop_expr = PropertyExpression::<Image>::from_box(image(literal_str("airport-11")), None);
        let evaluated_image =
            prop_expr.evaluate_feature_images(&empty_tile_feature(), &available_images, Image::default());
        assert!(evaluated_image.is_available());
        assert_eq!(evaluated_image.id(), "airport-11");
        assert_eq!(Dependency::IMAGE, prop_expr.get_dependencies());
        let dd_prop_expr = PropertyExpression::<Image>::from_box(image(get(literal_str("image_name"))), None);
        let evaluated_image = dd_prop_expr.evaluate_feature_images(&one_image(), &available_images, Image::default());
        assert!(evaluated_image.is_available());
        assert_eq!(evaluated_image.id(), "maki-11");
        assert_eq!(Dependency::IMAGE | Dependency::FEATURE, dd_prop_expr.get_dependencies());
        let evaluated_image =
            dd_prop_expr.evaluate_feature_images(&empty_tile_feature(), &available_images, Image::default());
        assert!(!evaluated_image.is_available());
        assert_eq!(evaluated_image.id(), "");
    }
    // evaluation with zoom
    {
        let expr = step(zoom(), image(literal_str("airport-11")), 18.0, image(literal_str("bicycle-15")));
        let prop_expr = PropertyExpression::<Image>::from_box(expr, None);
        let evaluated_image =
            prop_expr.evaluate_zoom_feature_images(0.0, &empty_tile_feature(), &available_images, Image::default());
        assert!(evaluated_image.is_available());
        assert_eq!(evaluated_image.id(), "airport-11");
        let evaluated_image =
            prop_expr.evaluate_zoom_feature_images(18.0, &empty_tile_feature(), &available_images, Image::default());
        assert!(evaluated_image.is_available());
        assert_eq!(evaluated_image.id(), "bicycle-15");
        assert_eq!(Dependency::IMAGE | Dependency::ZOOM, prop_expr.get_dependencies());
    }
}

// TODO(port): WithinExpression, WithinExpressionAntiMeridian,
// DistanceExpression (need GeoJSON geometry conversion; within/distance are
// expression's).
