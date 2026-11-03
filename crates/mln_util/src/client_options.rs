// Ports include/mln/util/client_options.hpp and src/mln/util/client_options.cpp.

/// `ClientOptions`. `Clone` is `clone()`.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ClientOptions {
    name: String,
    version: String,
}

impl ClientOptions {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = name.into();
        self
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn with_version(mut self, version: impl Into<String>) -> Self {
        self.version = version.into();
        self
    }

    pub fn version(&self) -> &str {
        &self.version
    }
}
