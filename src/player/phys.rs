use crate::{constants::*, vec2::F64x2, TileEffect};


#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum HorizontalDirection {
    Left,
    Right,
}

pub fn min(a: f64, b: f64) -> f64 {
    if a > b {
        b
    } else {
        a
    }
}

pub fn max(a: f64, b: f64) -> f64 {
    if a > b {
        a
    } else {
        b
    }
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
    pub x0_min: f64,
    pub x1_min: f64,
    pub x2_min: f64,
    pub x3_min: f64,

    pub x0_max: f64,
    pub x1_max: f64,
    pub x2_max: f64,
    pub x3_max: f64,

    pub y0_min: f64,
    pub y1_min: f64,
    pub y2_min: f64,

    pub y0_max: f64,
    pub y1_max: f64,
    pub y2_max: f64,
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
            x0_min: 0.0,
            x1_min: 0.0,
            x2_min: 0.0,
            x3_min: 0.0,
            x0_max: 0.0,
            x1_max: 0.0,
            x2_max: 0.0,
            x3_max: 0.0,
            y0_min: 0.0,
            y1_min: 0.0,
            y2_min: 0.0,
            y0_max: 0.0,
            y1_max: 0.0,
            y2_max: 0.0,
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

        // left -> right
        self.y0_min = get_limit(current_pixelspace_loc + F64x2::new(0.0, 0.0), 1);
        self.y1_min = get_limit(current_pixelspace_loc + F64x2::new(1.0, 0.0), 1);// unchanged
        self.y2_min = get_limit(current_pixelspace_loc + F64x2::new(2.0, 0.0), 1);

        self.y0_max = get_limit(current_pixelspace_loc + F64x2::new(0.0, 4.0), 2) - map_px_to_meter;
        self.y1_max = get_limit(current_pixelspace_loc + F64x2::new(1.0, 4.0), 2) - map_px_to_meter;//unchanged
        self.y2_max = get_limit(current_pixelspace_loc + F64x2::new(2.0, 4.0), 2) - map_px_to_meter;

        // bottom -> top
        self.x0_min = get_limit(current_pixelspace_loc + F64x2::new(0.0, 1.0), 3) + map_px_to_meter;
        self.x1_min = get_limit(current_pixelspace_loc + F64x2::new(0.0, 2.0), 3) + map_px_to_meter;//unchanged
        self.x2_min = get_limit(current_pixelspace_loc + F64x2::new(0.0, 3.0), 3) + map_px_to_meter;
        self.x3_min = get_limit(current_pixelspace_loc + F64x2::new(0.0, 4.0), 3) + map_px_to_meter;

        self.x0_max = get_limit(current_pixelspace_loc + F64x2::new(2.0, 1.0), 4);
        self.x1_max = get_limit(current_pixelspace_loc + F64x2::new(2.0, 2.0), 4);//unchanged
        self.x2_max = get_limit(current_pixelspace_loc + F64x2::new(2.0, 3.0), 4);
        self.x3_max = get_limit(current_pixelspace_loc + F64x2::new(2.0, 4.0), 4);

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
        let x_min = max(max(self.x0_min, self.x1_min), max(self.x2_min, self.x3_min));
        let y_min = max(max(self.y0_min, self.y1_min), self.y2_min);
        let x_max = min(min(self.x0_max, self.x1_max), min(self.x2_max, self.x3_max));
        let y_max = min(min(self.y0_max, self.y1_max), self.y2_max);
        if let Some((bounce_coeff, _)) = collision_information {
            // x min
            if self.loc.x < x_min {
                self.vel.x = -self.vel.x * bounce_coeff;
                self.loc.x = x_min;
            }
            // x max
            if self.loc.x + self.size.x > x_max {
                self.vel.x = -self.vel.x * bounce_coeff;
                self.loc.x = x_max - self.size.x;
            }
            // y min
            if self.loc.y < y_min {
                self.vel.y = -self.vel.y * bounce_coeff;
                self.loc.y = y_min;
            }
            // y max
            if self.loc.y + self.size.y > y_max {
                self.vel.y = -self.vel.y * bounce_coeff;
                self.loc.y = y_max - self.size.y;
            }
        } else if self.loc.x < x_min || self.loc.x + self.size.x > x_max || self.loc.y < y_min || self.loc.y + self.size.y > y_max {
            error!("a collision occured, but no collision information was present!");
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
