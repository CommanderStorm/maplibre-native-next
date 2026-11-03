// Ports test/style/style_layer.test.cpp.

use super::util::parse_json;
use crate::conversion::convert;
use crate::expression::dsl::{format_str, literal_str, to_color};
use crate::expression::{Expression, Formatted, FormattedSection, Image, Type};
use crate::filter::Filter;
use crate::layer::Layer;
use crate::layers::background_layer::BackgroundLayer;
use crate::layers::symbol_layer_impl::SymbolLayerPaintPropertyOverrides as MockOverrides;
use crate::layers::symbol_layer_properties::{TextColor, TextField};
use crate::layers::{
    CircleLayer, FillLayer, LineLayer, RasterLayer, SymbolLayoutProperties, SymbolPaintProperties,
};
use crate::properties::Transitionable;
use crate::property_expression::PropertyExpression;
use crate::property_value::PropertyValue;
use crate::renderer::possibly_evaluated_property_value::PossiblyEvaluatedPropertyValue;
use crate::style_impl::Impl as StyleImpl;
use crate::transition_options::TransitionOptions;
use mln_conversion::{Convertible, Error};
use mln_expression::format_expression::{FormatExpression, FormatExpressionSection};
use mln_expression::match_::Match;
use mln_style_types::{LineCapType, LineJoinType, TranslateAnchorType, VisibilityType};
use mln_util::Color;
use std::cell::RefCell;
use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::Arc;

fn color() -> Color {
    Color::new(1.0, 0.0, 0.0, 1.0)
}
const OPACITY: f32 = 1.0;
const RADIUS: f32 = 1.0;
const BLUR: f32 = 1.0;
fn pattern() -> PropertyValue<Image> {
    PropertyValue::Constant(Image::new("foo"))
}
const ANTIALIAS: bool = false;
const TRANSLATE: [f32; 2] = [0.0, 0.0];
const TRANSLATE_ANCHOR: TranslateAnchorType = TranslateAnchorType::Map;
const LINE_CAP: LineCapType = LineCapType::Round;
const LINE_JOIN: LineJoinType = LineJoinType::Miter;
const MITER_LIMIT: f32 = 1.0;
const ROUND_LIMIT: f32 = 1.0;
const WIDTH: f32 = 1.0;
const GAP_WIDTH: f32 = 1.0;
const OFFSET: f32 = 1.0;
fn dash_array() -> Vec<f32> {
    Vec::new()
}
const HUE_ROTATE: f32 = 1.0;
const BRIGHTNESS: f32 = 1.0;
const SATURATION: f32 = 1.0;
const CONTRAST: f32 = 1.0;
const DURATION: f32 = 1.0;

/// The implicit `PropertyValue<T>(T)` conversion of the C++ setters.
fn c<T>(t: T) -> PropertyValue<T> {
    PropertyValue::Constant(t)
}

fn parse_filter(expression: &str) -> Filter {
    let mut error = Error::default();
    let json = parse_json(expression);
    convert::<Filter>(&Convertible::from(&json), &mut error).expect("filter")
}

// gtest: Layer.BackgroundProperties
#[test]
fn background_properties() {
    let mut layer = BackgroundLayer::new("background");
    assert_eq!("background", layer.get_type_info().type_);

    // Paint properties

    layer.set_background_color(&c(color()));
    assert_eq!(*layer.get_background_color(), c(color()));

    layer.set_background_opacity(&c(OPACITY));
    assert_eq!(*layer.get_background_opacity(), c(OPACITY));

    layer.set_background_pattern(&pattern());
    assert_eq!(*layer.get_background_pattern(), pattern());
}

// gtest: Layer.CircleProperties
#[test]
fn circle_properties() {
    let mut layer = CircleLayer::new("circle", "source");
    assert_eq!("circle", layer.get_type_info().type_);

    // Paint properties

    layer.set_circle_color(&c(color()));
    assert_eq!(*layer.get_circle_color(), c(color()));

    layer.set_circle_opacity(&c(OPACITY));
    assert_eq!(*layer.get_circle_opacity(), c(OPACITY));

    layer.set_circle_radius(&c(RADIUS));
    assert_eq!(*layer.get_circle_radius(), c(RADIUS));

    layer.set_circle_blur(&c(BLUR));
    assert_eq!(*layer.get_circle_blur(), c(BLUR));

    layer.set_circle_translate(&c(TRANSLATE));
    assert_eq!(*layer.get_circle_translate(), c(TRANSLATE));

    layer.set_circle_translate_anchor(&c(TRANSLATE_ANCHOR));
    assert_eq!(*layer.get_circle_translate_anchor(), c(TRANSLATE_ANCHOR));
}

// gtest: Layer.FillProperties
#[test]
fn fill_properties() {
    let mut layer = FillLayer::new("fill", "source");
    assert_eq!("fill", layer.get_type_info().type_);

    // Paint properties

    layer.set_fill_color(&c(color()));
    assert_eq!(*layer.get_fill_color(), c(color()));

    layer.set_fill_outline_color(&c(color()));
    assert_eq!(*layer.get_fill_outline_color(), c(color()));

    layer.set_fill_opacity(&c(OPACITY));
    assert_eq!(*layer.get_fill_opacity(), c(OPACITY));

    layer.set_fill_pattern(&pattern());
    assert_eq!(*layer.get_fill_pattern(), pattern());

    layer.set_fill_antialias(&c(ANTIALIAS));
    assert_eq!(*layer.get_fill_antialias(), c(ANTIALIAS));

    layer.set_fill_translate(&c(TRANSLATE));
    assert_eq!(*layer.get_fill_translate(), c(TRANSLATE));

    layer.set_fill_translate_anchor(&c(TRANSLATE_ANCHOR));
    assert_eq!(*layer.get_fill_translate_anchor(), c(TRANSLATE_ANCHOR));
}

// gtest: Layer.LineProperties
#[test]
fn line_properties() {
    let mut layer = LineLayer::new("line", "source");
    assert_eq!("line", layer.get_type_info().type_);

    // Layout properties

    layer.set_line_cap(&c(LINE_CAP));
    assert_eq!(*layer.get_line_cap(), c(LINE_CAP));

    layer.set_line_join(&c(LINE_JOIN));
    assert_eq!(*layer.get_line_join(), c(LINE_JOIN));

    layer.set_line_miter_limit(&c(MITER_LIMIT));
    assert_eq!(*layer.get_line_miter_limit(), c(MITER_LIMIT));

    layer.set_line_round_limit(&c(ROUND_LIMIT));
    assert_eq!(*layer.get_line_round_limit(), c(ROUND_LIMIT));

    // Paint properties

    layer.set_line_color(&c(color()));
    assert_eq!(*layer.get_line_color(), c(color()));

    layer.set_line_opacity(&c(OPACITY));
    assert_eq!(*layer.get_line_opacity(), c(OPACITY));

    layer.set_line_translate(&c(TRANSLATE));
    assert_eq!(*layer.get_line_translate(), c(TRANSLATE));

    layer.set_line_translate_anchor(&c(TRANSLATE_ANCHOR));
    assert_eq!(*layer.get_line_translate_anchor(), c(TRANSLATE_ANCHOR));

    layer.set_line_width(&c(WIDTH));
    assert_eq!(*layer.get_line_width(), c(WIDTH));

    layer.set_line_gap_width(&c(GAP_WIDTH));
    assert_eq!(*layer.get_line_gap_width(), c(GAP_WIDTH));

    layer.set_line_offset(&c(OFFSET));
    assert_eq!(*layer.get_line_offset(), c(OFFSET));

    layer.set_line_blur(&c(BLUR));
    assert_eq!(*layer.get_line_blur(), c(BLUR));

    layer.set_line_dasharray(&c(dash_array()));
    assert_eq!(*layer.get_line_dasharray(), c(dash_array()));

    layer.set_line_pattern(&pattern());
    assert_eq!(*layer.get_line_pattern(), pattern());
}

// gtest: Layer.RasterProperties
#[test]
fn raster_properties() {
    let mut layer = RasterLayer::new("raster", "source");
    assert_eq!("raster", layer.get_type_info().type_);

    // Paint properties

    layer.set_raster_opacity(&c(OPACITY));
    assert_eq!(*layer.get_raster_opacity(), c(OPACITY));

    layer.set_raster_hue_rotate(&c(HUE_ROTATE));
    assert_eq!(*layer.get_raster_hue_rotate(), c(HUE_ROTATE));

    layer.set_raster_brightness_min(&c(BRIGHTNESS));
    assert_eq!(*layer.get_raster_brightness_min(), c(BRIGHTNESS));

    layer.set_raster_brightness_max(&c(BRIGHTNESS));
    assert_eq!(*layer.get_raster_brightness_max(), c(BRIGHTNESS));

    layer.set_raster_saturation(&c(SATURATION));
    assert_eq!(*layer.get_raster_saturation(), c(SATURATION));

    layer.set_raster_contrast(&c(CONTRAST));
    assert_eq!(*layer.get_raster_contrast(), c(CONTRAST));

    layer.set_raster_fade_duration(&c(DURATION));
    assert_eq!(*layer.get_raster_fade_duration(), c(DURATION));
}

// gtest: Layer.Observer
#[test]
fn observer() {
    // TEST(Layer, Observer). C++ counts `StubLayerObserver::layerChanged`
    // calls; in the port a change reaches `Style::Impl::onLayerChanged` when
    // the layer's impl `Arc` is replaced (see layer.rs), so each "notifies"
    // check is "the impl pointer changed".
    let mut layer = LineLayer::new("line", "source");
    let notified = |layer: &mut LineLayer, f: &mut dyn FnMut(&mut LineLayer)| {
        let before = layer.base_impl();
        f(layer);
        !Arc::ptr_eq(&before, &layer.base_impl())
    };

    // Notifies observer on filter change.
    assert!(notified(&mut layer, &mut |l| l.set_filter(&parse_filter(r#"["==", "foo", "bar"]"#))));

    // Notifies observer on visibility change.
    assert!(notified(&mut layer, &mut |l| l.set_visibility(VisibilityType::None)));

    // Notifies observer on paint property change.
    assert!(notified(&mut layer, &mut |l| l.set_line_color(&c(color()))));

    // Notifies observer on layout property change.
    assert!(notified(&mut layer, &mut |l| l.set_line_cap(&c(LINE_CAP))));

    // Does not notify observer on no-op visibility change.
    assert!(!notified(&mut layer, &mut |l| l.set_visibility(VisibilityType::None)));

    // Does not notify observer on no-op paint property change.
    assert!(!notified(&mut layer, &mut |l| l.set_line_color(&c(color()))));

    // Does not notify observer on no-op layout property change.
    assert!(!notified(&mut layer, &mut |l| l.set_line_cap(&c(LINE_CAP))));

    // Notifies observer on source-layer change.
    assert!(notified(&mut layer, &mut |l| l.set_source_layer("roads")));

    // Does not notify observer on no-op source-layer change.
    assert!(!notified(&mut layer, &mut |l| l.set_source_layer("roads")));

    // Notifies observer on source ID change.
    assert!(notified(&mut layer, &mut |l| l.set_source_id("other-source")));

    // Does not notify observer on no-op source ID change.
    assert!(!notified(&mut layer, &mut |l| l.set_source_id("other-source")));
}

fn fixture(path: &str) -> String {
    let base = std::env::var("RUNFILES_DIR")
        .or_else(|_| std::env::var("TEST_SRCDIR"))
        .expect("RUNFILES_DIR or TEST_SRCDIR");
    let path = PathBuf::from(base).join("maplibre+").join(path);
    std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()))
}

// gtest: Layer.DuplicateLayer
#[test]
fn duplicate_layer() {
    // Setup style
    let mut style = StyleImpl::new(1.0);
    style.load_json(&fixture("test/fixtures/resources/style-unused-sources.json"));

    // Add initial layer
    style
        .add_layer(Box::new(LineLayer::new("line", "unusedsource")), None)
        .expect("first add");

    // Try to add duplicate
    match style.add_layer(Box::new(LineLayer::new("line", "unusedsource")), None) {
        Ok(_) => panic!("Should not have been allowed to add a duplicate layer id"),
        // Expected
        Err(e) => assert_eq!("Layer line already exists", e.to_string()),
    }
}

// gtest: Layer.IncompatibleLayer
#[test]
fn incompatible_layer() {
    // Setup style
    let mut style = StyleImpl::new(1.0);
    style.load_json(&fixture("test/fixtures/resources/style-unused-sources.json"));

    // Try to add duplicate
    match style.add_layer(Box::new(RasterLayer::new("raster", "unusedsource")), None) {
        Ok(_) => panic!("Should not have been allowed to add an incompatible layer to the source"),
        // Expected
        Err(e) => assert_eq!("Layer 'raster' is not compatible with source 'unusedsource'", e.to_string()),
    }
}

/// The text-field values `testHasOverrides` sets, in its order.
#[derive(Clone, Copy, Debug)]
enum FormattedCase {
    Undefined,
    ConstantNoOverride,
    ConstantOverride,
    ExpressionNoOverride,
    ExpressionOverride,
    NestedOverride,
}

const FORMATTED_CASES: [FormattedCase; 6] = [
    FormattedCase::Undefined,
    FormattedCase::ConstantNoOverride,
    FormattedCase::ConstantOverride,
    FormattedCase::ExpressionNoOverride,
    FormattedCase::ExpressionOverride,
    FormattedCase::NestedOverride,
];

impl FormattedCase {
    fn has_override(self) -> bool {
        matches!(
            self,
            FormattedCase::ConstantOverride | FormattedCase::ExpressionOverride | FormattedCase::NestedOverride
        )
    }

    fn constant(self) -> Option<Formatted> {
        match self {
            // Constant, no overrides.
            FormattedCase::ConstantNoOverride => Some(Formatted::from_plain("")),
            // Constant, overridden text-color.
            FormattedCase::ConstantOverride => {
                let mut formatted = Formatted::from_plain("");
                formatted.sections.push(FormattedSection::new(
                    "section text".to_owned(),
                    None,
                    None,
                    Some(Color::green()),
                ));
                Some(formatted)
            }
            _ => None,
        }
    }

    fn expression(self) -> Option<PropertyExpression<Formatted>> {
        let expression: Box<dyn Expression> = match self {
            // Expression, no overrides.
            FormattedCase::ExpressionNoOverride => Box::new(FormatExpression::new(Vec::new())),
            // Expression, overridden text-color.
            FormattedCase::ExpressionOverride => {
                let mut section = FormatExpressionSection::new(literal_str(""));
                section.set_text_section_options(None, None, Some(to_color(literal_str("red"), None)));
                Box::new(FormatExpression::new(vec![section]))
            }
            // Nested expressions, overridden text-color.
            FormattedCase::NestedOverride => {
                let formatted_expr1 = format_str("first paragraph");
                let mut second_paragraph = FormatExpressionSection::new(literal_str("second paragraph"));
                second_paragraph.set_text_section_options(None, None, Some(to_color(literal_str("blue"), None)));
                let formatted_expr2: Box<dyn Expression> = Box::new(FormatExpression::new(vec![second_paragraph]));
                let mut branches: BTreeMap<String, Arc<dyn Expression>> = BTreeMap::new();
                branches.insert("1st".to_owned(), Arc::from(formatted_expr1));
                branches.insert("2nd".to_owned(), Arc::from(formatted_expr2));
                Box::new(Match::<String>::new(
                    Type::Formatted,
                    literal_str("input"),
                    branches,
                    format_str("otherwise"),
                ))
            }
            _ => return None,
        };
        Some(PropertyExpression::from_box(expression, None))
    }
}

// gtest: Layer.SymbolLayerOverrides
#[test]
fn symbol_layer_overrides() {
    // TEST(Layer, SymbolLayerOverrides). The C++ Mock*Properties are
    // `Properties<TextField>` / `Properties<TextColor>`; the symbol layer's
    // sets hold the same tags.

    // Unevaluated / transitionable.
    {
        let layout = RefCell::new(SymbolLayoutProperties::Unevaluated::default());
        // testHasOverrides<PropertyValue>(layout)
        for case in FORMATTED_CASES {
            let value = match case {
                FormattedCase::Undefined => PropertyValue::Undefined,
                _ => case
                    .constant()
                    .map(PropertyValue::Constant)
                    .or_else(|| case.expression().map(PropertyValue::Expression))
                    .unwrap(),
            };
            *layout.borrow_mut().get_mut::<TextField>() = value;
            assert_eq!(
                case.has_override(),
                MockOverrides::has_overrides(layout.borrow().get::<TextField>()),
                "{case:?}"
            );
        }

        let mut current = SymbolPaintProperties::Transitionable::default();
        let mut updated = SymbolPaintProperties::Transitionable::default();
        *current.get_mut::<TextColor>() = Transitionable {
            value: c(Color::green()),
            options: TransitionOptions::default(),
        };
        *updated.get_mut::<TextColor>() = Transitionable {
            value: c(Color::green()),
            options: TransitionOptions::default(),
        };
        assert!(!MockOverrides::has_paint_property_difference(&current, &updated));

        *current.get_mut::<TextColor>() = Transitionable {
            value: c(Color::red()),
            options: TransitionOptions::default(),
        };
        assert!(MockOverrides::has_paint_property_difference(&current, &updated));
    }

    // Possibly evaluated.
    {
        let mut layout = SymbolLayoutProperties::PossiblyEvaluated::default();
        // testHasOverrides<PossiblyEvaluatedPropertyValue>(layout); C++'s
        // `PossiblyEvaluatedPropertyValue<Formatted>()` is a default-constructed
        // (empty) constant, so its "Undefined" step has no override.
        for case in FORMATTED_CASES {
            let value = match case {
                FormattedCase::Undefined => PossiblyEvaluatedPropertyValue::default(),
                _ => case
                    .constant()
                    .map(PossiblyEvaluatedPropertyValue::Constant)
                    .or_else(|| case.expression().map(PossiblyEvaluatedPropertyValue::Expression))
                    .unwrap(),
            };
            *layout.get_mut::<TextField>() = value;
            assert_eq!(
                case.has_override(),
                MockOverrides::has_overrides(layout.get::<TextField>()),
                "{case:?}"
            );
        }
        // TODO(port): the setOverrides / updateOverrides half of this case
        // and the global-state case build `expression::FormatSectionOverride`
        // (render side, with RenderSymbolLayer; M5).
    }
}
