// Ports src/mln/layout/symbol_projection.hpp and
// src/mln/layout/symbol_projection.cpp: the label-plane matrices (see the
// C++ file's overview of coordinate spaces).
//
// TODO(port): the CPU line-label projection (`project`,
// `evaluateSizeForFeature`, `placeFirstAndLastGlyph`, `placeGlyphsAlongLine`,
// `reprojectLineLabels`, `hideGlyphs`): it arrives with placement.

use mln_map_types::TransformState;
use mln_util::mat4::{Mat4, matrix};

/// `getTileSkewVectors(state, vecEast, vecSouth)`.
pub fn get_tile_skew_vectors(state: &TransformState) -> ([f64; 2], [f64; 2]) {
    let cos_roll = state.get_roll().cos();
    let sin_roll = state.get_roll().sin();
    let cos_pitch = state.get_pitch().cos();
    let cos_bearing = (-state.get_bearing()).cos();
    let sin_bearing = (-state.get_bearing()).sin();
    let mut vec_south = [
        -cos_bearing * cos_pitch * sin_roll - sin_bearing * cos_roll,
        -sin_bearing * cos_pitch * sin_roll + cos_bearing * cos_roll,
    ];
    let vec_south_len = vec_south[0].hypot(vec_south[1]);
    if vec_south_len < 1.0e-9 {
        vec_south = [0.0, 0.0];
    } else {
        vec_south[0] /= vec_south_len;
        vec_south[1] /= vec_south_len;
    }
    let mut vec_east = [
        cos_bearing * cos_pitch * cos_roll - sin_bearing * sin_roll,
        sin_bearing * cos_pitch * cos_roll + cos_bearing * sin_roll,
    ];
    let vec_east_len = vec_east[0].hypot(vec_east[1]);
    if vec_east_len < 1.0e-9 {
        vec_east = [0.0, 0.0];
    } else {
        vec_east[0] /= vec_east_len;
        vec_east[1] /= vec_east_len;
    }
    (vec_east, vec_south)
}

/// `getTileSkewMatrix(state)`.
pub fn get_tile_skew_matrix(state: &TransformState) -> Mat4 {
    let (vec_east, vec_south) = get_tile_skew_vectors(state);
    let mut m: Mat4 = [0.0; 16];
    matrix::identity(&mut m);
    m[0] = vec_east[0];
    m[1] = vec_east[1];
    m[4] = vec_south[0];
    m[5] = vec_south[1];
    m
}

/// `getLabelPlaneMatrix(posMatrix, pitchWithMap, rotateWithMap, state,
/// pixelsToTileUnits)`: a matrix for converting from tile units to the
/// correct label coordinate space.
pub fn get_label_plane_matrix(
    pos_matrix: &Mat4,
    pitch_with_map: bool,
    rotate_with_map: bool,
    state: &TransformState,
    pixels_to_tile_units: f32,
) -> Mat4 {
    let mut m: Mat4 = [0.0; 16];
    matrix::identity(&mut m);
    if pitch_with_map {
        let s = f64::from(1.0 / pixels_to_tile_units);
        let a = m;
        matrix::scale(&mut m, &a, s, s, 1.0);
        if !rotate_with_map {
            let skew = get_tile_skew_matrix(state);
            let mut inverted = skew;
            matrix::invert(&mut inverted, &skew);
            let a = m;
            matrix::multiply(&mut m, &a, &inverted);
        }
    } else {
        let size = state.get_size();
        let a = m;
        matrix::scale(&mut m, &a, f64::from(size.width) / 2.0, -(f64::from(size.height) / 2.0), 1.0);
        let a = m;
        matrix::translate(&mut m, &a, 1.0, -1.0, 0.0);
        let a = m;
        matrix::multiply(&mut m, &a, pos_matrix);
    }
    m
}

/// `getGlCoordMatrix(posMatrix, pitchWithMap, rotateWithMap, state,
/// pixelsToTileUnits)`: a matrix for converting from the correct label
/// coordinate space to gl coords.
pub fn get_gl_coord_matrix(
    pos_matrix: &Mat4,
    pitch_with_map: bool,
    rotate_with_map: bool,
    state: &TransformState,
    pixels_to_tile_units: f32,
) -> Mat4 {
    let mut m: Mat4 = [0.0; 16];
    matrix::identity(&mut m);
    if pitch_with_map {
        let a = m;
        matrix::multiply(&mut m, &a, pos_matrix);
        let s = f64::from(pixels_to_tile_units);
        let a = m;
        matrix::scale(&mut m, &a, s, s, 1.0);
        if !rotate_with_map {
            let a = m;
            matrix::multiply(&mut m, &a, &get_tile_skew_matrix(state));
        }
    } else {
        let size = state.get_size();
        let a = m;
        matrix::scale(&mut m, &a, 1.0, -1.0, 1.0);
        let a = m;
        matrix::translate(&mut m, &a, -1.0, -1.0, 0.0);
        let a = m;
        matrix::scale(&mut m, &a, 2.0 / f64::from(size.width), 2.0 / f64::from(size.height), 1.0);
    }
    m
}
