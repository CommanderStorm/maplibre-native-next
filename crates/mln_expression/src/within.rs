// Ports include/mln/style/expression/within.hpp and
// src/mln/style/expression/within.cpp.
//
// TODO(port): the "within" expression (GeoJSON polygon parsing and the
// point/line-in-polygon tests on tile geometry) arrives with GeoJSON sources
// (M3). Until then it fails to parse with an explicit error.

use crate::parsing_context::{ParseResult, ParsingContext};
use mln_conversion::Convertible;

pub struct Within;

impl Within {
    pub fn parse(_value: &Convertible<'_>, ctx: &mut ParsingContext) -> ParseResult {
        ctx.error("TODO(port): the 'within' expression is not ported yet.".to_owned());
        None
    }
}
