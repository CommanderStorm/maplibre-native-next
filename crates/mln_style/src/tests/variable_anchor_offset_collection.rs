// Ports test/style/variable_anchor_offset_collection.test.cpp (the type is
// in mln_style_types, its converter in mln_style).

use super::util::parse_json;
use crate::conversion::{Converter, Error};
use mln_conversion::Convertible;
use mln_style_types::{AnchorOffsetPair, SymbolAnchorType, VariableAnchorOffsetCollection};

fn parse(src: &str, error: &mut Error) -> Option<VariableAnchorOffsetCollection> {
    let value = parse_json(src);
    <VariableAnchorOffsetCollection as Converter>::convert(&Convertible::from(&value), error)
}

#[test]
fn calculations() {
    // TEST(VariableAnchorOffsetCollection, Calculations)
    let mut error = Error::default();

    // valid value
    {
        let a = parse(r#"["top", [0, 0], "bottom", [2, 2]]"#, &mut error).expect("a");
        let b = parse(r#"["top", [1, 1], "bottom", [4, 4]]"#, &mut error).expect("b");
        let c = VariableAnchorOffsetCollection::interpolate(&a, &b, 0.5).unwrap();
        assert_eq!(
            VariableAnchorOffsetCollection::new(vec![
                AnchorOffsetPair::new(SymbolAnchorType::Top, [0.5, 0.5]),
                AnchorOffsetPair::new(SymbolAnchorType::Bottom, [3.0, 3.0]),
            ]),
            c
        );
    }

    // should throw with mismatched endpoints
    {
        let a = parse(r#"["top", [0, 0]]"#, &mut error).expect("a");
        let b = parse(r#"["bottom", [1, 1]]"#, &mut error).expect("b");
        assert!(VariableAnchorOffsetCollection::interpolate(&a, &b, 0.5).is_err());

        let a = parse(r#"["top", [0, 0]]"#, &mut error).expect("a");
        let b = parse(r#"["top", [1, 1], "bottom", [2, 2]]"#, &mut error).expect("b");
        assert!(VariableAnchorOffsetCollection::interpolate(&a, &b, 0.5).is_err());
    }
}

#[test]
fn style_conversion() {
    // TEST(VariableAnchorOffsetCollection, StyleConversion), without the
    // symbol-layer part (TODO(port) with the symbol layer).
    let mut error = Error::default();

    // valid value
    {
        let json_value = r#"["left",[1.0,2.0],"top",[0.0,0.0]]"#;
        let variable_anchor_offset = parse(json_value, &mut error).expect("valid");
        assert_eq!(variable_anchor_offset.len(), 2);
        assert_eq!(variable_anchor_offset[0].anchor_type, SymbolAnchorType::Left);
        assert_eq!(variable_anchor_offset[0].offset[0], 1.0);
        assert_eq!(variable_anchor_offset[0].offset[1], 2.0);

        assert_eq!(variable_anchor_offset.to_string(), json_value);
    }

    // Invalid values
    let cases = [
        ("{}", "value must be an array"),
        ("[]", "array must contain an even number of elements"),
        (r#"["test",  [1, 2]]"#, "anchor must be a valid anchor value"),
        (r#"["top", "left"]"#, "anchor offset must be an array"),
        (r#"["top",  [1, 2, 3]]"#, "anchor offset must have two elements"),
        (r#"["top",  ["test", "test"]]"#, "anchor offset must have two numbers"),
    ];
    for (json, expected_error) in cases {
        let variable_anchor_offset = parse(json, &mut error);
        assert!(variable_anchor_offset.is_none(), "{json}");
        assert_eq!(error.message, expected_error, "{json}");
    }
}
