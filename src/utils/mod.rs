use crate::vec2::F64x2;

pub fn rectangle_by_points(c0: F64x2, c1: F64x2) -> [f64; 4] {
    graphics::rectangle::rectangle_by_corners(c0.x, c0.y, c1.x, c1.y)
}
