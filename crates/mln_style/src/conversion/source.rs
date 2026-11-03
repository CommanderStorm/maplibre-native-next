// Ports include/mln/style/conversion/source.hpp and
// src/mln/style/conversion/source.cpp.

use super::{Converter, Convertible, Error};
use crate::source::Source;
use crate::sources::geojson_source::{GeoJSONOptions, GeoJSONSource};
use geojson::GeoJSON;
use std::sync::Arc;

/// `convertGeoJSONSource(id, value, error)`.
fn convert_geojson_source(id: &str, value: &Convertible<'_>, error: &mut Error) -> Option<Box<dyn Source>> {
    let Some(data_value) = value.object_member("data") else {
        error.message = "GeoJSON source must have a data value".to_owned();
        return None;
    };

    let mut options = GeoJSONOptions::default_options();
    if let Some(converted) = GeoJSONOptions::convert(value, error) {
        options = Arc::new(converted);
    }

    let result = GeoJSONSource::new(id, options);

    if data_value.is_object() {
        let geojson = GeoJSON::convert(&data_value, error)?;
        if let Err(e) = result.set_geojson(&geojson) {
            // TODO(port): C++ lets the Supercluster exception (a clustered
            // non-point feature) propagate out of the style parse.
            error.message = e;
            return None;
        }
    } else if let Some(url) = data_value.to_string() {
        result.set_url(&url);
    } else {
        error.message = "GeoJSON data must be a URL or an object".to_owned();
        return None;
    }

    Some(Box::new(result))
}

/// `convert<std::unique_ptr<Source>>(value, error, id)`.
pub fn convert_source(value: &Convertible<'_>, error: &mut Error, id: &str) -> Option<Box<dyn Source>> {
    if !value.is_object() {
        error.message = "source must be an object".to_owned();
        return None;
    }

    let Some(type_value) = value.object_member("type") else {
        error.message = "source must have a type".to_owned();
        return None;
    };

    let Some(type_) = type_value.to_string() else {
        error.message = "source type must be a string".to_owned();
        return None;
    };
    match type_.as_str() {
        "geojson" => convert_geojson_source(id, value, error),
        "raster" | "raster-dem" | "vector" | "image" => {
            // TODO(port): convertRasterSource, convertRasterDEMSource,
            // convertVectorSource, convertImageSource with the tile and
            // image sources (M4). Until then the parser logs this and skips
            // the source, as it does for an invalid one.
            error.message = format!("source type '{type_}' is not ported yet");
            None
        }
        _ => {
            error.message = "invalid source type".to_owned();
            None
        }
    }
}
