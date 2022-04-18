pub mod colors;
pub mod vec2;

#[macro_use]
extern crate log;
#[macro_use]
extern crate derivative;

use colors::*;
use glutin_window::GlutinWindow;
use image::{imageops, io::Reader as ImageReader, Rgba, RgbaImage};
use opengl_graphics::{GlGraphics, OpenGL, Texture, TextureSettings};
use piston::{
    event_loop::{EventSettings, Events},
    window::WindowSettings,
    Button, Key, PressEvent, ReleaseEvent, RenderEvent, Size, UpdateEvent,
};
use vec2::F64x2;

pub fn rectangle_by_points(c0: F64x2, c1: F64x2) -> [f64; 4] {
    graphics::rectangle::rectangle_by_corners(c0.x, c0.y, c1.x, c1.y)
}

pub const METERS_TO_POINTS: f64 = 100.0;
pub const POINTS_TO_METERS: f64 = 1.0 / METERS_TO_POINTS;

pub const GRAMS_TO_KG: f64 = 0.001;

pub const GRAVITY: F64x2 = F64x2::new(0.0, -9.80665);
pub const BOUNCE_COEFF: f64 = 0.1;
/// friciton coefficients
/// for this section, see https://en.wikipedia.org/wiki/Friction#Approximate_coefficients_of_friction

/// when it is close enough to the ground, this is applied as velocity -= FLOOR_FRICTION_COEFF * WEIGHT * GRAVITY
pub const FLOOR_FRICTION_COEFF: F64x2 = F64x2::new(0.9, 0.0);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum HorizontalDirection {
    Left,
    Right,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Direction {
    N,
    NE,
    E,
    SE,
    S,
    SW,
    W,
    NW,
}

#[derive(Debug, Clone)]
pub struct PlayerPhys {
    /// m
    ///
    /// loc += vel * dt
    loc: F64x2,
    /// newtons or 1 kg * m/s^2
    force: F64x2,
    /// m/s^2
    ///
    /// f=ma so accel = f/m
    accel: F64x2,
    /// m/s
    ///
    /// vel += accel * dt
    vel: F64x2,
    /// forces, but only from l/r movement
    movement_forces: F64x2,
    /// kg
    mass: f64,
    /// if it is 'touching' the ground
    down_to_earth: bool,
    /// last direction of movement. by default, this is right
    last_direction: HorizontalDirection,
    /// width, height from the bottom left corner
    size: F64x2,
    // min and max vals
    x_min: f64,
    y_min: f64,
    x_max: f64,
    y_max: f64,
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

    pub fn update(&mut self, dt: f64, win_size: [f64; 2], map: &RgbaImage, map_px_to_meter: f64) {
        let meter_to_map_px = 1.0 / map_px_to_meter;
        // TODO make friction apply in the y axis
        // let friction = if self.down_to_earth {
        //     FLOOR_FRICTION_COEFF.x * self.mass * GRAVITY.y
        // } else {
        //     0.0
        // };
        // let forces_after_friction = forces - if forces.x.is_sign_negative() { -friction } else { friction };
        // println!("{:?} - {:?} = {:?}", forces, friction, forces_after_friction);
        // // make sure that the old force is less than the new one, and that the sign stays the same
        // if forces.x.abs() > forces_after_friction.x.abs() && forces.x.is_sign_negative() == forces_after_friction.x.is_sign_negative() {
        //     forces = forces_after_friction;
        // }
        // TODO implement a proper system for storing forces

        let forces = self.force + self.movement_forces;
        self.accel = forces / self.mass;
        // self.vel += GRAVITY * dt;
        self.vel += self.accel * dt;

        //TODO make better friction
        if self.down_to_earth {
            let mut friction = FLOOR_FRICTION_COEFF.x * GRAVITY.y * dt;
            if !self.vel.x.is_sign_negative() {
                friction = -friction;
            }
            // println!(
            //     "{} - {} = {}",
            //     self.vel.x,
            //     friction,
            //     self.vel.x - friction
            // );
            self.vel.x = if ((self.vel.x - friction).abs() < self.vel.x.abs())
                && ((self.vel.x - friction).is_sign_negative() == self.vel.x.is_sign_negative())
            {
                // println!("friction applies");
                self.vel.x - friction
            } else {
                0.0
            };

            // TODO remove this stupid y axis friction
            let mut friction = FLOOR_FRICTION_COEFF.x * GRAVITY.y * dt;
            if !self.vel.y.is_sign_negative() {
                friction = -friction;
            }
            // println!(
            //     "{} - {} = {}",
            //     self.vel.x,
            //     friction,
            //     self.vel.x - friction
            // );
            self.vel.y = if ((self.vel.y - friction).abs() < self.vel.y.abs())
                && ((self.vel.y - friction).is_sign_negative() == self.vel.y.is_sign_negative())
            {
                // println!("friction applies");
                self.vel.y - friction
            } else {
                0.0
            };
        }
        // let last_loc = self.loc;
        let mut new_loc = self.loc + self.vel * dt;

        if new_loc.x < 0.0 || new_loc.y < 0.0 {
            // new_loc = F64x2::new(1.0, 3.0);
            // self.vel = F64x2::zero();
            println!("out of base bounds!");
        }

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
        self.x_min = get_limit(current_pixelspace_loc + F64x2::new(0.0, 1.0), 3);
        println!("x min val: {}", self.x_min);
        self.x_max = get_limit(current_pixelspace_loc + F64x2::new(2.0, 1.0), 4);
        println!("x max val: {}", self.x_max);

        // println!("old location:\n{:#?}\nnew location:\n{:#?}", current_pixelspace_loc, new_pixelspace_loc);

        // if let Some(pixel) =
        //     map.get_pixel_checked(new_pixelspace_loc.0.x as u32, new_pixelspace_loc.0.y as u32)
        // {
        //     println!("{:?} @ {} {}", pixel, new_pixelspace_loc.0.x as u32, new_pixelspace_loc.0.y as u32);
        //     if *pixel != Rgba([0; 4]) {
        //         let mut acceptable = 0;
        //         loop {
        //             if let Some(pixel) = map.get_pixel_checked(
        //                 new_pixelspace_loc.0.x as u32,
        //                 new_pixelspace_loc.0.y as u32 + acceptable,
        //             ) {
        //                 if *pixel == Rgba([0; 4]) {
        //                     break;
        //                 } else {
        //                     acceptable += 1;
        //                 }
        //             } else {
        //                 break;
        //             }
        //         }
        //         new_pixelspace_loc.0.y = (new_pixelspace_loc.0.y as u32 + acceptable) as f64;
        //     }
        //     // self.loc = new_loc;
        //     // self.vel.y = self.vel.y.abs() * BOUNCE_COEFF;
        // } else {
        //     // self.loc = new_loc;
        // }
        self.loc = new_loc;

        // // corner, collision direction, distance of collision, resulting direction
        // let mut collisions: Vec<(Direction, Direction, f64, Direction)> = vec![];

        // for (i, corner) in [
        //     (1, self.loc),
        //     (
        //         2,
        //         F64x2 {
        //             x: self.loc.x + self.size.x,
        //             y: self.loc.y,
        //         },
        //     ),
        //     (
        //         3,
        //         F64x2 {
        //             x: self.loc.x,
        //             y: self.loc.y + self.size.y,
        //         },
        //     ),
        //     (4, self.loc + self.size),
        // ] {
        //     let nearby_x = [
        //         corner.x + 1.0 * map_px_to_meter,
        //         corner.x - 1.0 * map_px_to_meter,
        //         corner.x,
        //     ];
        //     let nearby_y = [
        //         corner.y + 1.0 * map_px_to_meter,
        //         corner.y - 1.0 * map_px_to_meter,
        //         corner.y,
        //     ];
        //     for x in nearby_x {
        //         for y in nearby_y {
        //             let distance = corner.dist_between_circles(
        //                 F64x2::new(x.floor(), y.floor()),
        //                 0.0,
        //                 map_px_to_meter * 1.5,
        //             ); // if this distance is < 0, then it is within the collision zone
        //             let pxl_x = (x * meter_to_map_px).floor() as i64;
        //             let pxl_y = (y * meter_to_map_px).floor() as i64;
        //             if pxl_x.is_negative() || pxl_y.is_negative() {
        //                 continue;
        //             } else {
        //                 if let Some(pxl) = map.get_pixel_checked(x as u32, y as u32) {
        //                     if pxl.0[3] != 0 && distance < 0.0 {
        //                         // collision!
        //                         let base_direction = match i {
        //                             1 => Direction::SW,
        //                             2 => Direction::SE,
        //                             3 => Direction::NW,
        //                             4 => Direction::NE,
        //                             _ => unreachable!(),
        //                         };
        //                         let direction = match (
        //                             ((corner.x * meter_to_map_px).floor()
        //                                 - (x * meter_to_map_px).floor())
        //                             .is_sign_negative(), /* pos = s neg = n */
        //                             (
        //                                 (((corner.x * meter_to_map_px).floor()
        //                                 - (x * meter_to_map_px).floor()) as i64) == 0
        //                             ), /* is it zero */
        //                             ((corner.y * meter_to_map_px).floor()
        //                                 - (y * meter_to_map_px).floor())
        //                             .is_sign_negative(), /* pos = w neg = e */
        //                             (
        //                                 (((corner.y * meter_to_map_px).floor()
        //                                 - (y * meter_to_map_px).floor()) as i64) == 0
        //                             ), /* is it zero */
        //                         ) {
        //                             //* FIXME right now the NE, NW, SE, and SW collisions are removed as they kinda take place anyway farther
        //                             //* on thanks to the stacking of collisions, and it would be rather bad if they were in place at that point
        //                             // north and its variations
        //                             // (true, false, true, false) => Direction::NE,
        //                             // (true, false, false, false) => Direction::NW,
        //                             (true, false, _, true) => Direction::N,
        //                             // south and its variations
        //                             // (false, false, true, false) => Direction::SE,
        //                             // (false, false, false, false) => Direction::SW,
        //                             (false, false, _, true) => Direction::S,
        //                             // just east and west
        //                             (_, true, true, false) => Direction::E,
        //                             (_, true, false, false) => Direction::W,
        //                             (_, true, _, true) => continue,
        //                             _ => continue,
        //                         };
        //                         let resulting_direction = match base_direction {
        //                             Direction::NE => {
        //                                 match direction {
        //                                     Direction::NE => direction,
        //                                     Direction::N => direction,
        //                                     Direction::E => direction,
        //                                     _ => Direction::NE,
        //                                 }
        //                             }
        //                             Direction::SE => {
        //                                 match direction {
        //                                     Direction::S => direction,
        //                                     Direction::E => direction,
        //                                     Direction::SE => direction,
        //                                     _ => Direction::SE
        //                                 }
        //                             }
        //                             Direction::SW => {
        //                                 match direction {
        //                                     Direction::W => direction,
        //                                     Direction::S => direction,
        //                                     Direction::SW => direction,
        //                                     _ => Direction::SW
        //                                 }
        //                             }
        //                             Direction::NW => {
        //                                 match direction {
        //                                     Direction::N => direction,
        //                                     Direction::W => direction,
        //                                     Direction::NW => direction,
        //                                     _ => Direction::NW
        //                                 }
        //                             }
        //                             _ => unreachable!(),
        //                         };
        //                         collisions.push((base_direction, direction, distance, resulting_direction));
        //                     }
        //                 }
        //             }
        //         }
        //     }
        // }

        // if !collisions.is_empty() {
        //     println!("{:#?}", collisions);
        //     for coll in collisions {
        //         match coll.3 {
        //             Direction::N => {
        //                 self.vel.y = -self.vel.y.abs() * BOUNCE_COEFF;
        //             }
        //             Direction::NE => {
        //                 self.vel.y = -self.vel.y.abs() * BOUNCE_COEFF;
        //                 self.vel.x = -self.vel.x.abs() * BOUNCE_COEFF;
        //             }
        //             Direction::E => {
        //                 self.vel.x = -self.vel.x.abs() * BOUNCE_COEFF;
        //             }
        //             Direction::SE => {
        //                 self.vel.y = self.vel.y.abs() * BOUNCE_COEFF;
        //                 self.vel.x = -self.vel.x.abs() * BOUNCE_COEFF;
        //             }
        //             Direction::S => {
        //                 self.vel.y = self.vel.y.abs() * BOUNCE_COEFF;
        //             }
        //             Direction::SW => {
        //                 self.vel.y = self.vel.y.abs() * BOUNCE_COEFF;
        //                 self.vel.x = self.vel.x.abs() * BOUNCE_COEFF;
        //             }
        //             Direction::W => {
        //                 self.vel.x = self.vel.x.abs() * BOUNCE_COEFF;
        //             }
        //             Direction::NW => {
        //                 self.vel.y = -self.vel.y.abs() * BOUNCE_COEFF;
        //                 self.vel.x = self.vel.x.abs() * BOUNCE_COEFF;
        //             }
        //         }
        //     }
        //     self.loc = last_loc;
        // }

        //TODO implement proper bouncing
        // bouncing off walls (no longer needed, as a proper map is being made)
        if self.loc.x < 0.0 {
            self.vel.x = -self.vel.x * BOUNCE_COEFF;
            self.loc.x = 0.0;
        }
        if self.loc.x + self.size.x > win_size[0] * POINTS_TO_METERS {
            self.vel.x = -self.vel.x * BOUNCE_COEFF;
            self.loc.x = win_size[0] * POINTS_TO_METERS - self.size.x;
        }
        if self.loc.y < 0.0 {
            self.vel.y = -self.vel.y * BOUNCE_COEFF;
            self.loc.y = 0.0;
        }
        if self.loc.y + self.size.y > win_size[1] * POINTS_TO_METERS {
            self.vel.y = -self.vel.y * BOUNCE_COEFF;
            self.loc.y = win_size[1] * POINTS_TO_METERS - self.size.y;
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

#[derive(Derivative)]
#[derivative(Debug)]
pub struct Player {
    phys: PlayerPhys,
    #[derivative(Debug = "ignore")]
    sprites: (Texture, Texture),
    // cfg values
    /// force added to y velocity on jumping
    jump_force: f64,
    move_force: f64,
}

impl Player {
    pub fn new(loc: F64x2, mass: f64, jump_force: f64, move_force: f64) -> Self {
        let player_image = ImageReader::open("assets/cursd_le_mon_smol.png")
            .unwrap()
            .decode()
            .unwrap()
            .to_rgba8();

        let scale_factor = 4;

        let player_image_upscaled = imageops::resize(
            &player_image,
            player_image.width() * scale_factor,
            player_image.height() * scale_factor,
            imageops::Nearest,
        );

        let sprites = (
            Texture::from_image(
                &imageops::flip_horizontal(&player_image_upscaled),
                &TextureSettings::new(),
            ),
            Texture::from_image(&player_image_upscaled, &TextureSettings::new()),
        );

        Self {
            phys: PlayerPhys::new(
                loc,
                mass,
                F64x2 {
                    x: player_image.width() as f64 * scale_factor as f64 * POINTS_TO_METERS,
                    y: player_image.height() as f64 * scale_factor as f64 * POINTS_TO_METERS,
                },
            ),
            sprites,
            jump_force,
            move_force,
        }
    }

    pub fn draw(
        &mut self,
        c: &graphics::Context,
        gl: &mut GlGraphics,
        win_height: f64,
        cam_loc: F64x2,
        map_px_to_meter: f64,
    ) {
        let meter_to_map_px = 1.0 / map_px_to_meter;
        let globalize_physics_cord = move |coord: F64x2| -> F64x2 {
            F64x2 {
                x: coord.x,
                y: win_height - coord.y,
            }
        };

        use graphics::*;

        Image::new()
            .rect(rectangle_by_points(
                globalize_physics_cord(self.phys.loc * METERS_TO_POINTS),
                globalize_physics_cord((self.phys.loc + self.phys.size) * METERS_TO_POINTS),
            ))
            .draw(
                match self.phys.last_direction {
                    HorizontalDirection::Left => &self.sprites.0,
                    HorizontalDirection::Right => &self.sprites.1,
                },
                &graphics::DrawState::default(),
                c.transform
                    .trans(-cam_loc.x * METERS_TO_POINTS, cam_loc.y * METERS_TO_POINTS),
                gl,
            );

        // let circle = rectangle::centered_square(
        //     (self.phys.loc.x + self.phys.size.x / 2.0) * METERS_TO_POINTS,
        //     (self.phys.loc.y + self.phys.size.y / 2.0) * METERS_TO_POINTS,
        //     30.0,
        // );
        // Ellipse::new(rgba(50, 100, 255, 0.4)).draw(
        //     circle,
        //     &DrawState::default(),
        //     c.transform
        //         .trans(-cam_loc.x * METERS_TO_POINTS, -cam_loc.y * METERS_TO_POINTS),
        //     gl,
        // );

        Rectangle::new(rgba(0, 243, 223, 0.42)).draw(
            rectangle_by_points(
                globalize_physics_cord(
                    F64x2::new(
                        (self.phys.loc.x * meter_to_map_px).floor(),
                        (self.phys.loc.y * meter_to_map_px).floor(),
                    ) * map_px_to_meter
                        * METERS_TO_POINTS,
                ),
                globalize_physics_cord(
                    F64x2::new(
                        (self.phys.loc.x * meter_to_map_px).floor() + 3.0,
                        (self.phys.loc.y * meter_to_map_px).floor() + 4.0,
                    ) * map_px_to_meter
                        * METERS_TO_POINTS,
                ),
            ),
            &DrawState::default(),
            c.transform
                .trans(-cam_loc.x * METERS_TO_POINTS, cam_loc.y * METERS_TO_POINTS),
            gl,
        );

        // for i in [
        //     F64x2 {
        //         x: self.phys.loc.x + self.phys.size.x / 2.0,
        //         y: self.phys.loc.y,
        //     },
        //     F64x2 {
        //         x: self.phys.loc.x,
        //         y: self.phys.loc.y + self.phys.size.y / 2.0,
        //     },
        //     F64x2 {
        //         x: self.phys.loc.x + self.phys.size.x,
        //         y: self.phys.loc.y + self.phys.size.y / 2.0,
        //     },
        //     F64x2 {
        //         x: self.phys.loc.x + self.phys.size.x / 2.0,
        //         y: self.phys.loc.y + self.phys.size.y,
        //     },
        // ] {
        //     let circle =
        //         rectangle::centered_square(i.x * METERS_TO_POINTS, i.y * METERS_TO_POINTS, 5.0);
        //     Ellipse::new(rgba(0, 100, 110, 0.4)).draw(
        //         circle,
        //         &DrawState::default(),
        //         c.transform
        //             .trans(-cam_loc.x * METERS_TO_POINTS, -cam_loc.y * METERS_TO_POINTS),
        //         gl,
        //     );
        // }
    }

    pub fn update_phys(
        &mut self,
        dt: f64,
        win_size: [f64; 2],
        map: &RgbaImage,
        map_px_to_meter: f64,
    ) {
        self.phys.update(dt, win_size, map, map_px_to_meter)
    }

    pub fn jump(&mut self) {
        self.phys.vel.y += self.jump_force;
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct RectangularObject {
    c0: F64x2,
    c1: F64x2,
    color: Color,
}

impl RectangularObject {
    pub fn new(c0: F64x2, c1: F64x2, color: Color) -> Self {
        Self { c0, c1, color }
    }

    pub fn draw(&self, c: &graphics::Context, gl: &mut GlGraphics, win_height: f64) {
        let globalize_physics_cord = move |coord: F64x2| -> F64x2 {
            F64x2 {
                x: coord.x,
                y: win_height - coord.y,
            }
        };

        use graphics::*;

        rectangle::Rectangle::new(self.color).draw(
            rectangle_by_points(
                globalize_physics_cord(self.c0),
                globalize_physics_cord(self.c1),
            ),
            &DrawState::default(),
            c.transform,
            gl,
        );
    }
}

fn main() {
    pretty_env_logger::formatted_builder()
        .filter_level(log::LevelFilter::Debug)
        .init();

    info!("Initialized");

    // Change this to OpenGL::V2_1 if not working.
    let opengl = OpenGL::V4_5;

    // Create an Glutin window.
    let mut window: GlutinWindow = WindowSettings::new("platformR", [200, 200])
        .graphics_api(opengl)
        .exit_on_esc(true)
        .size(Size {
            width: 1_000.0,
            height: 700.0,
        })
        .vsync(true)
        .controllers(true)
        .build()
        .unwrap();

    let mut gl = GlGraphics::new(opengl);
    let mut win_size = [0f64; 2];

    // unit is METERS
    let mut cam_loc = F64x2::new(1.0, 1.0);

    let map = ImageReader::open("assets/sample_map_100x100.png")
        .unwrap()
        .decode()
        .unwrap()
        .to_rgba8();
    let map_px_to_meter = 1.0 / 5.0;

    let mut player = Player::new(
        F64x2::splat(1.0),
        113.0 * GRAMS_TO_KG, /* about how much a large lemon weighs */
        5.0,
        2.0,
    );

    let mut events = Events::new({
        let mut es = EventSettings::new();
        // rendering only when receiving input
        es.lazy = false;
        // agressivly skips updates to catch up
        es.ups_reset = 0;
        // 100 updates per second
        es.ups = 100;
        es
    });

    while let Some(e) = events.next(&mut window) {
        if let Some(args) = e.render_args() {
            use graphics::*;

            win_size = args.window_size;

            gl.draw(args.viewport(), |c, gl| {
                clear(DARK_GREY, gl);

                player.draw(&c, gl, win_size[1], cam_loc, map_px_to_meter);

                let globalize_physics_cord = move |coord: F64x2| -> F64x2 {
                    F64x2 {
                        x: coord.x,
                        y: win_size[1] * POINTS_TO_METERS - coord.y,
                    }
                };

                for (raw_x, raw_y, px) in map.enumerate_pixels() {
                    let x_pts = raw_x as f64 * map_px_to_meter;
                    let y_pts = (map.height() - raw_y - 1) as f64 * map_px_to_meter;

                    Rectangle::new([
                        px.0[0] as f32,
                        px.0[1] as f32,
                        px.0[2] as f32,
                        px.0[3] as f32,
                    ])
                    .draw(
                        rectangle_by_points(
                            globalize_physics_cord(F64x2::new(x_pts, y_pts)) * METERS_TO_POINTS,
                            globalize_physics_cord(F64x2::new(
                                x_pts + map_px_to_meter,
                                y_pts + map_px_to_meter,
                            )) * METERS_TO_POINTS,
                        ),
                        &DrawState::default(),
                        c.transform
                            .trans(-cam_loc.x * METERS_TO_POINTS, cam_loc.y * METERS_TO_POINTS),
                        gl,
                    );
                }
            });
        }

        if let Some(args) = e.update_args() {
            player.update_phys(args.dt, win_size, &map, map_px_to_meter);
            cam_loc = F64x2 {
                x: player.phys.loc.x - win_size[0] * POINTS_TO_METERS / 2.0
                    + player.phys.size.x / 2.0,
                y: player.phys.loc.y - win_size[1] * POINTS_TO_METERS / 2.0
                    + player.phys.size.y / 2.0,
            };
        }

        if let Some(args) = e.press_args() {
            match args {
                Button::Mouse(mouse_btn) => match mouse_btn {
                    _ => {}
                },
                Button::Keyboard(keyboard_btn) => match keyboard_btn {
                    Key::A => {
                        player.phys.movement_forces += F64x2::new(-player.move_force, 0.0);
                    }
                    Key::D => {
                        player.phys.movement_forces += F64x2::new(player.move_force, 0.0);
                    }
                    Key::W => {
                        player.phys.movement_forces += F64x2::new(0.0, player.move_force);
                    }
                    Key::S => {
                        player.phys.movement_forces += F64x2::new(0.0, -player.move_force);
                    }
                    Key::Space => {
                        // boing
                        // if player.phys.down_to_earth {
                        player.jump();
                        // }
                    }
                    _ => {}
                },
                _ => {}
            }
        }

        if let Some(args) = e.release_args() {
            match args {
                Button::Mouse(mouse_btn) => match mouse_btn {
                    _ => {}
                },
                Button::Keyboard(keyboard_btn) => match keyboard_btn {
                    Key::A => {
                        player.phys.movement_forces -= F64x2::new(-player.move_force, 0.0);
                    }
                    Key::D => {
                        player.phys.movement_forces -= F64x2::new(player.move_force, 0.0);
                    }
                    Key::W => {
                        player.phys.movement_forces -= F64x2::new(0.0, player.move_force);
                    }
                    Key::S => {
                        player.phys.movement_forces -= F64x2::new(0.0, -player.move_force);
                    }
                    _ => {}
                },
                _ => {}
            }
        }
    }
}
