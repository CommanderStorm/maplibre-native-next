// Ports test/util/text_conversions.test.cpp.
//
// `platform::uppercase` / `platform::lowercase` (platform/default
// string_stdlib.cpp, nunicode) are `nunicode::{uppercase, lowercase}`.

use nunicode::{lowercase, uppercase};

// gtest: TextConversions.to_upper
#[test]
fn to_upper() {
    assert_eq!("STREET", uppercase("strEEt")); // EN
    assert_eq!("ROAD", uppercase("rOAd")); // EN

    assert_eq!("STRASSE", uppercase("straße")); // DE
    assert_eq!("MASSE", uppercase("maße")); // DE
    assert_eq!("WEISSKOPFSEEADLER", uppercase("weißkopfseeadler")); // DE

    assert_eq!("BÊNÇÃO", uppercase("bênção")); // PT
    assert_eq!("AZƏRBAYCAN", uppercase("Azərbaycan")); // AZ
    assert_eq!("ὈΔΥΣΣΕΎΣ", uppercase("Ὀδυσσεύς")); // GR
}

// gtest: TextConversions.to_lower
#[test]
fn to_lower() {
    assert_eq!("street", lowercase("strEEt")); // EN
    assert_eq!("road", lowercase("rOAd")); // EN

    assert_eq!("straße", lowercase("Straße")); // DE
    assert_eq!("strasse", lowercase("STRASSE")); // DE
    assert_eq!("masse", lowercase("MASSE")); // DE
    assert_eq!("weisskopfseeadler", lowercase("weiSSkopfseeadler")); // DE

    assert_eq!("bênção", lowercase("BÊNÇÃO")); // PT
    assert_eq!("azərbaycan", lowercase("AZƏRBAYCAN")); // AZ
    assert_eq!("ὀδυσσεύς", lowercase("ὈΔΥΣΣΕΎΣ")); // GR (not __QT__)
}
