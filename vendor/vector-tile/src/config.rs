// Ports vendor/vector-tile/include/mapbox/vector_tile/vector_tile_config.hpp.
//
// The C++ unscoped enums become modules of constants (`TileType::LAYERS` →
// `tile_type::LAYERS`), so they compare with `PbfReader::tag()` as in C++.
// `GeomType` is a newtype because the reader `static_cast`s any decoded
// enum value to it (values other than 0..=3 exist).

/// `enum TileType : std::uint32_t`.
#[allow(non_snake_case)]
pub mod TileType {
    pub const LAYERS: u32 = 3;
}

/// `enum LayerType : std::uint32_t`.
#[allow(non_snake_case)]
pub mod LayerType {
    pub const VERSION: u32 = 15;
    pub const NAME: u32 = 1;
    pub const FEATURES: u32 = 2;
    pub const KEYS: u32 = 3;
    pub const VALUES: u32 = 4;
    pub const EXTENT: u32 = 5;
}

/// `enum FeatureType : std::uint32_t`.
#[allow(non_snake_case)]
pub mod FeatureType {
    pub const ID: u32 = 1;
    pub const TAGS: u32 = 2;
    pub const TYPE: u32 = 3;
    pub const GEOMETRY: u32 = 4;
    pub const RASTER: u32 = 5;
}

/// `enum ValueType : std::uint32_t`.
#[allow(non_snake_case)]
pub mod ValueType {
    pub const STRING: u32 = 1;
    pub const FLOAT: u32 = 2;
    pub const DOUBLE: u32 = 3;
    pub const INT: u32 = 4;
    pub const UINT: u32 = 5;
    pub const SINT: u32 = 6;
    pub const BOOL: u32 = 7;
}

/// `enum GeomType : std::uint8_t`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct GeomType(pub u8);

impl GeomType {
    pub const UNKNOWN: GeomType = GeomType(0);
    pub const POINT: GeomType = GeomType(1);
    pub const LINESTRING: GeomType = GeomType(2);
    pub const POLYGON: GeomType = GeomType(3);
}

/// `enum CommandType : std::uint8_t`.
#[allow(non_snake_case)]
pub mod CommandType {
    pub const END: u8 = 0;
    pub const MOVE_TO: u8 = 1;
    pub const LINE_TO: u8 = 2;
    pub const CLOSE: u8 = 7;
}
