// Ports include/mln/util/tile_server_options.hpp, src/mln/util/tile_server_options.cpp
// and include/mln/util/default_style.hpp (+ default_style.cpp).
//
// TODO(port): MapboxConfiguration, MapTilerConfiguration.

/// `util::DefaultStyle`.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct DefaultStyle {
    url: String,
    name: String,
    current_version: i32,
}

impl DefaultStyle {
    pub fn new(url: impl Into<String>, name: impl Into<String>, current_version: i32) -> Self {
        DefaultStyle {
            url: url.into(),
            name: name.into(),
            current_version,
        }
    }

    pub fn get_url(&self) -> &str {
        &self.url
    }

    pub fn get_name(&self) -> &str {
        &self.name
    }

    pub fn get_current_version(&self) -> i32 {
        self.current_version
    }
}

/// `TileServerOptions`. `Clone` is `clone()`.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct TileServerOptions {
    base_url: String,
    version_prefix: Option<String>,
    uri_scheme_alias: String,

    source_template: String,
    source_domain_name: String,
    source_version_prefix: Option<String>,

    style_template: String,
    style_domain_name: String,
    style_version_prefix: Option<String>,

    sprites_template: String,
    sprites_domain_name: String,
    sprites_version_prefix: Option<String>,

    glyphs_template: String,
    glyphs_domain_name: String,
    glyphs_version_prefix: Option<String>,

    tile_template: String,
    tile_domain_name: String,
    tile_version_prefix: Option<String>,

    api_key_parameter_name: String,
    // C++ leaves `apiKeyRequired` uninitialized in a default-constructed
    // object; every configuration sets it.
    api_key_required: bool,

    default_styles: Vec<DefaultStyle>,
    default_style: String,
}

macro_rules! template_accessors {
    ($with:ident, $template:ident, $domain:ident, $prefix:ident) => {
        pub fn $with(mut self, template: impl Into<String>, domain_name: impl Into<String>, version_prefix: Option<String>) -> Self {
            self.$template = template.into();
            self.$domain = domain_name.into();
            self.$prefix = version_prefix;
            self
        }
        pub fn $template(&self) -> &str {
            &self.$template
        }
        pub fn $domain(&self) -> &str {
            &self.$domain
        }
        pub fn $prefix(&self) -> &Option<String> {
            &self.$prefix
        }
    };
}

impl TileServerOptions {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_base_url(mut self, url: impl Into<String>) -> Self {
        self.base_url = url.into();
        self
    }

    pub fn base_url(&self) -> &str {
        &self.base_url
    }

    pub fn with_version_prefix(mut self, version_prefix: Option<String>) -> Self {
        self.version_prefix = version_prefix;
        self
    }

    pub fn version_prefix(&self) -> &Option<String> {
        &self.version_prefix
    }

    pub fn with_uri_scheme_alias(mut self, alias: impl Into<String>) -> Self {
        self.uri_scheme_alias = alias.into();
        self
    }

    pub fn uri_scheme_alias(&self) -> &str {
        &self.uri_scheme_alias
    }

    template_accessors!(with_source_template, source_template, source_domain_name, source_version_prefix);
    template_accessors!(with_style_template, style_template, style_domain_name, style_version_prefix);
    template_accessors!(with_sprites_template, sprites_template, sprites_domain_name, sprites_version_prefix);
    template_accessors!(with_glyphs_template, glyphs_template, glyphs_domain_name, glyphs_version_prefix);
    template_accessors!(with_tile_template, tile_template, tile_domain_name, tile_version_prefix);

    pub fn with_api_key_parameter_name(mut self, name: impl Into<String>) -> Self {
        self.api_key_parameter_name = name.into();
        self
    }

    pub fn api_key_parameter_name(&self) -> &str {
        &self.api_key_parameter_name
    }

    pub fn set_requires_api_key(mut self, api_key_required: bool) -> Self {
        self.api_key_required = api_key_required;
        self
    }

    pub fn requires_api_key(&self) -> bool {
        self.api_key_required
    }

    pub fn default_styles(&self) -> Vec<DefaultStyle> {
        self.default_styles.clone()
    }

    pub fn with_default_styles(mut self, styles: Vec<DefaultStyle>) -> Self {
        self.default_styles = styles;
        self
    }

    pub fn with_default_style(mut self, default_style: impl Into<String>) -> Self {
        self.default_style = default_style.into();
        self
    }

    pub fn default_style(&self) -> &str {
        &self.default_style
    }

    /// `DefaultConfiguration()`.
    pub fn default_configuration() -> Self {
        Self::maplibre_configuration()
    }

    /// `MapLibreConfiguration()`.
    pub fn maplibre_configuration() -> Self {
        // https://demotiles.maplibre.org/style.json
        let styles = vec![DefaultStyle::new("maplibre://maps/style", "Basic", 1)];

        TileServerOptions::new()
            .with_base_url("https://demotiles.maplibre.org")
            .with_uri_scheme_alias("maplibre")
            .with_api_key_parameter_name("")
            .with_source_template("/tiles/{domain}.json", "", None)
            .with_style_template("{path}.json", "maps", None)
            .with_sprites_template("/{path}/sprite{scale}.{format}", "", None)
            .with_glyphs_template("/font/{fontstack}/{start}-{end}.pbf", "fonts", None)
            .with_tile_template("/{path}", "tiles", None)
            .with_default_styles(styles)
            .with_default_style("Basic")
            .set_requires_api_key(false)
    }
}
