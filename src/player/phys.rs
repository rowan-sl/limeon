use crate::{constants::*, vec2::F64x2, TileEffect, TileEffectCondition};
use image::Rgba;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum HorizontalDirection {
    Left,
    Right,
}

#[derive(Debug, Clone)]
pub struct PlayerPhys {
    /// m
    ///
    /// loc += vel * dt
    pub loc: F64x2,
    /// newtons or 1 kg * m/s^2
    pub force: F64x2,
    /// m/s^2
    ///
    /// f=ma so accel = f/m
    pub accel: F64x2,
    /// m/s
    ///
    /// vel += accel * dt
    pub vel: F64x2,
    /// forces, but only from l/r movement
    pub movement_forces: F64x2,
    /// kg
    pub mass: f64,
    /// last direction of movement. by default, this is right
    pub last_direction: HorizontalDirection,
    /// width, height from the bottom left corner
    pub size: F64x2,
    // min and max vals
    pub x_min: f64,
    pub y_min: f64,
    pub x_max: f64,
    pub y_max: f64,
}

impl PlayerPhys {
    pub fn new(loc: F64x2, mass: f64, size: F64x2) -> Self {
        Self {
            loc,
            force: F64x2::zero(),
            accel: F64x2::zero(),
            vel: F64x2::zero(),
            movement_forces: F64x2::zero(),
            mass,
            last_direction: HorizontalDirection::Right,
            size,
            x_min: 0.0,
            y_min: 0.0,
            x_max: 0.0,
            y_max: 0.0,
        }
    }

    pub fn update(&mut self, dt: f64, map: &crate::WorldMap) {
        let map_px_to_meter = map.map_px_to_meter;
        let meter_to_map_px = 1.0 / map_px_to_meter;
        let forces = self.force + self.movement_forces;
        self.accel = forces / self.mass;
        self.vel += GRAVITY * dt;
        self.vel += self.accel * dt;

        let new_loc = self.loc + self.vel * dt;

        let pixel_space_self_coords = |loc: F64x2| {
            F64x2::new(
                (loc.x * meter_to_map_px).floor(),
                (loc.y * meter_to_map_px).floor(),
            )
        };

        // let mut new_pixelspace_loc = pixel_space_self_coords(new_loc);
        let current_pixelspace_loc = pixel_space_self_coords(self.loc);

        // information about collisions occuring below the player. (bounce fac, friction)
        let mut collision_information: Option<(f64, F64x2)> = None;

        let mut get_limit =
            |start: F64x2, /* starting coord in pixel space */
             mode: u8      /* 1 = y min 2 = y max 3 = x min 4 = x max */| {
                let mut lim = match mode {
                    1 | 2 => start.y,
                    3 | 4 => start.x,
                    _ => unreachable!(),
                };
                loop {
                    if let Some(pixel) = map.map.get_pixel_checked(
                        match mode {
                            1 | 2 => start.x,
                            3 | 4 => lim,
                            _ => unreachable!(),
                        } as u32,
                        map.map.height()
                            - match mode {
                                1 | 2 => lim,
                                3 | 4 => start.y,
                                _ => unreachable!(),
                            } as u32,
                    ) {
                        let mut collision: bool = false;
                        if let Some(eff) = map.effect_map.get(pixel) {
                            for effect in &eff.0 {
                                match effect {
                                    //TODO implement the rest of the effects
                                    TileEffect::Collision(bounce_factor, friction) => {
                                        collision = true;
                                        if mode == 1 {
                                            // y axis min distance (collision on players feet)
                                            trace!(
                                                "x min: {:?} = {:?} @ {:?}",
                                                pixel,
                                                eff,
                                                (lim, start.y)
                                            );
                                            collision_information =
                                                Some((*bounce_factor, *friction));
                                        }
                                    }
                                    _ => {}
                                }
                            }
                        }
                        if collision {
                            break;
                        } else {
                            match mode {
                                1 | 3 => {
                                    // y or x bottom lim
                                    if let Some(new) = (lim as u32).checked_sub(1) {
                                        lim = new as f64;
                                    } else {
                                        break;
                                    }
                                }
                                2 | 4 => {
                                    // y or x top lim
                                    lim = (lim as u32 + 1) as f64;
                                }
                                _ => unreachable!(),
                            }
                        }
                    } else {
                        break;
                    }
                }

                let lim_meters = lim * map_px_to_meter;
                lim_meters
            };

        self.y_min = get_limit(current_pixelspace_loc + F64x2::new(1.0, 0.0), 1);
        trace!("y min val: {}", self.y_min);
        self.y_max = get_limit(current_pixelspace_loc + F64x2::new(1.0, 4.0), 2) - map_px_to_meter;
        trace!("y max val: {}", self.y_max);
        self.x_min = get_limit(current_pixelspace_loc + F64x2::new(0.0, 2.0), 3) + map_px_to_meter;
        trace!("x min val: {}", self.x_min);
        self.x_max = get_limit(current_pixelspace_loc + F64x2::new(2.0, 2.0), 4);
        trace!("x max val: {}", self.x_max);

        self.loc = new_loc;

        if let Some((_, friction_coeff)) = collision_information {
            let mut friction = friction_coeff.x * GRAVITY.y * dt;
            if !self.vel.x.is_sign_negative() {
                friction = -friction;
            }
            self.vel.x = if ((self.vel.x - friction).abs() < self.vel.x.abs())
                && ((self.vel.x - friction).is_sign_negative() == self.vel.x.is_sign_negative())
            {
                self.vel.x - friction
            } else {
                0.0
            };
        }

        //TODO implement proper bouncing
        // x min
        if self.loc.x < self.x_min {
            self.vel.x = -self.vel.x * collision_information.unwrap().0;
            self.loc.x = self.x_min;
        }
        // x max
        if self.loc.x + self.size.x > self.x_max {
            self.vel.x = -self.vel.x * collision_information.unwrap().0;
            self.loc.x = self.x_max - self.size.x;
        }
        // y min
        if self.loc.y < self.y_min {
            self.vel.y = -self.vel.y * collision_information.unwrap().0;
            self.loc.y = self.y_min;
        }
        // y max
        if self.loc.y + self.size.y > self.y_max {
            self.vel.y = -self.vel.y * collision_information.unwrap().0;
            self.loc.y = self.y_max - self.size.y;
        }

        if self.movement_forces.x > 0.0 {
            self.last_direction = HorizontalDirection::Right;
        } else if self.movement_forces.x < 0.0 {
            self.last_direction = HorizontalDirection::Left;
        }
        trace!("{:#?}", self);
        trace!("tick");
    }
}
