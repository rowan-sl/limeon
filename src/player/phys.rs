use crate::{constants::*, vec2::F64x2};
use image::{Rgba, RgbaImage};

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
    /// if it is 'touching' the ground
    pub down_to_earth: bool,
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
            down_to_earth: false,
            last_direction: HorizontalDirection::Right,
            size,
            x_min: 0.0,
            y_min: 0.0,
            x_max: 0.0,
            y_max: 0.0,
        }
    }

    pub fn update(&mut self, dt: f64, map: &RgbaImage, map_px_to_meter: f64) {
        let meter_to_map_px = 1.0 / map_px_to_meter;
        let forces = self.force + self.movement_forces;
        self.accel = forces / self.mass;
        self.vel += GRAVITY * dt;
        self.vel += self.accel * dt;

        //TODO make better friction
        if self.down_to_earth {
            // let mut friction = FLOOR_FRICTION_COEFF.x * GRAVITY.y * dt;
            // if !self.vel.x.is_sign_negative() {
            //     friction = -friction;
            // }
            // self.vel.x = if ((self.vel.x - friction).abs() < self.vel.x.abs())
            //     && ((self.vel.x - friction).is_sign_negative() == self.vel.x.is_sign_negative())
            // {
            //     // println!("friction applies");
            //     self.vel.x - friction
            // } else {
            //     0.0
            // };
        }

        let new_loc = self.loc + self.vel * dt;

        let pixel_space_self_coords = |loc: F64x2| {
            F64x2::new(
                (loc.x * meter_to_map_px).floor(),
                (loc.y * meter_to_map_px).floor(),
            )
        };

        // let mut new_pixelspace_loc = pixel_space_self_coords(new_loc);
        let current_pixelspace_loc = pixel_space_self_coords(self.loc);

        let get_limit =
            |start: F64x2, /* starting coord in pixel space */
             mode: u8      /* 1 = y min 2 = y max 3 = x min 4 = x max */| {
                let mut lim = match mode {
                    1 | 2 => start.y,
                    3 | 4 => start.x,
                    _ => unreachable!(),
                };
                loop {
                    if let Some(pixel) = map.get_pixel_checked(
                        match mode {
                            1 | 2 => start.x,
                            3 | 4 => lim,
                            _ => unreachable!(),
                        } as u32,
                        map.height()
                            - match mode {
                                1 | 2 => lim,
                                3 | 4 => start.y,
                                _ => unreachable!(),
                            } as u32,
                    ) {
                        if *pixel != Rgba([0; 4]) {
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
        println!("y min val: {}", self.y_min);
        self.y_max = get_limit(current_pixelspace_loc + F64x2::new(1.0, 4.0), 2);
        println!("y max val: {}", self.y_max);
        self.x_min = get_limit(current_pixelspace_loc + F64x2::new(0.0, 2.0), 3);
        println!("x min val: {}", self.x_min);
        self.x_max = get_limit(current_pixelspace_loc + F64x2::new(2.0, 2.0), 4);
        println!("x max val: {}", self.x_max);

        self.loc = new_loc;

        //TODO implement proper bouncing
        // bouncing off walls (no longer needed, as a proper map is being made)
        // x min
        if self.loc.x < self.x_min {
            self.vel.x = -self.vel.x * BOUNCE_COEFF;
            self.loc.x = self.x_min;
        }
        // x max
        if self.loc.x + self.size.x > self.x_max {
            self.vel.x = -self.vel.x * BOUNCE_COEFF;
            self.loc.x = self.x_max - self.size.x;
        }
        // y min
        if self.loc.y < self.y_min {
            self.vel.y = -self.vel.y * BOUNCE_COEFF;
            self.loc.y = self.y_min;
        }
        // y max
        if self.loc.y + self.size.y > self.y_max {
            self.vel.y = -self.vel.y * BOUNCE_COEFF;
            self.loc.y = self.y_max - self.size.y;
        }
        // self.down_to_earth = self.loc.y < 0.1;
        //TODO fix this
        self.down_to_earth = true;
        if self.movement_forces.x > 0.0 {
            self.last_direction = HorizontalDirection::Right;
        } else if self.movement_forces.x < 0.0 {
            self.last_direction = HorizontalDirection::Left;
        }
        println!("{:#?}", self);
        println!("tick");
    }
}
