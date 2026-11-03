// Ports include/mln/renderer/query.hpp.

use mln_style::filter::Filter;

/// Options for query rendered features.
#[derive(Clone, Debug, Default)]
pub struct RenderedQueryOptions {
    /// layerIDs to include in the query
    pub layer_ids: Option<Vec<String>>,

    pub filter: Option<Filter>,
}

impl RenderedQueryOptions {
    pub fn new(layer_ids: Option<Vec<String>>, filter: Option<Filter>) -> Self {
        RenderedQueryOptions { layer_ids, filter }
    }
}

/// Options for query source features
#[derive(Clone, Debug, Default)]
pub struct SourceQueryOptions {
    /// Required for VectorSource, ignored for GeoJSONSource
    pub source_layers: Option<Vec<String>>,

    pub filter: Option<Filter>,
}

impl SourceQueryOptions {
    pub fn new(source_layers: Option<Vec<String>>, filter: Option<Filter>) -> Self {
        SourceQueryOptions { source_layers, filter }
    }
}
