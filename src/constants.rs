use crate::vec2::F64x2;

pub const METERS_TO_POINTS: f64 = 100.0;
pub const POINTS_TO_METERS: f64 = 1.0 / METERS_TO_POINTS;

pub const GRAMS_TO_KG: f64 = 0.001;

pub const GRAVITY: F64x2 = F64x2::new(0.0, -9.80665);
pub const BOUNCE_COEFF: f64 = 0.1;
/// friciton coefficients
/// for this section, see https://en.wikipedia.org/wiki/Friction#Approximate_coefficients_of_friction

/// when it is close enough to the ground, this is applied as velocity -= FLOOR_FRICTION_COEFF * WEIGHT * GRAVITY
pub const FLOOR_FRICTION_COEFF: F64x2 = F64x2::new(0.2, 0.0);
