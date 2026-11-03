// Ports include/mln/util/tileset.hpp (the `Scheme` that Resource::tile
// takes).
//
// TODO(port): `Tileset` itself (tiles, zoom range, attribution, scheme,
// encodings, bounds) with the tile sources (M4).

/// `Tileset::Scheme`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum Scheme {
    #[default]
    XYZ,
    TMS,
}
