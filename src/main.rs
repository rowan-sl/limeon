pub mod colors;
pub mod vec2;

#[macro_use]
extern crate log;
#[macro_use]
extern crate derivative;


use colors::*;
use glutin_window::GlutinWindow;
use image::{imageops, io::Reader as ImageReader};
use opengl_graphics::{GlGraphics, OpenGL, Texture, TextureSettings};
use piston::{
    event_loop::{EventSettings, Events},
    window::WindowSettings,
    Button, Key, PressEvent, ReleaseEvent, RenderEvent, UpdateEvent,
};
use vec2::F64x2;

pub fn rectangle_by_points(c0: F64x2, c1: F64x2) -> [f64; 4] {
    graphics::rectangle::rectangle_by_corners(c0.x, c0.y, c1.x, c1.y)
}

pub const METERS_TO_POINTS: f64 = 100.0;
pub const POINTS_TO_METERS: f64 = 1.0 / METERS_TO_POINTS;

pub const GRAVITY: F64x2 = F64x2::new(0.0, -9.80665);
pub const BOUNCE_COEFF: f64 = 0.1;
/// friciton coefficients
/// for this section, see https://en.wikipedia.org/wiki/Friction#Approximate_coefficients_of_friction

/// when it is close enough to the ground, this is applied as velocity -= FLOOR_FRICTION_COEFF * WEIGHT * GRAVITY
pub const FLOOR_FRICTION_COEFF: F64x2 = F64x2::new(0.6, 0.0);

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
}

impl PlayerPhys {
    pub fn new(loc: F64x2, mass: f64) -> Self {
        Self {
            loc,
            force: F64x2::zero(),
            accel: F64x2::zero(),
            vel: F64x2::zero(),
            movement_forces: F64x2::zero(),
            mass,
            down_to_earth: false,
            last_direction: HorizontalDirection::Right,
        }
    }

    pub fn update(&mut self, dt: f64, win_size: [f64; 2]) {
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
            self.vel += GRAVITY * dt;
            self.vel += self.accel * dt;
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
                    && ((self.vel.x - friction).is_sign_negative()
                        == self.vel.x.is_sign_negative())
                {
                    // println!("friction applies");
                    self.vel.x - friction
                } else {
                    0.0
                }
            }
            self.loc += self.vel * dt;

            //TODO implement friction and proper bouncing
            if self.loc.x < 0.0 {
                self.vel.x = -self.vel.x * BOUNCE_COEFF;
                self.loc.x = 0.0;
            }
            if self.loc.x > win_size[0] * POINTS_TO_METERS {
                self.vel.x = -self.vel.x * BOUNCE_COEFF;
                self.loc.x = win_size[0] * POINTS_TO_METERS;
            }
            if self.loc.y < 0.0 {
                self.vel.y = -self.vel.y * BOUNCE_COEFF;
                self.loc.y = 0.0;
            }
            if self.loc.y > win_size[1] * POINTS_TO_METERS {
                self.vel.y = -self.vel.y * BOUNCE_COEFF;
                self.loc.y = win_size[1] * POINTS_TO_METERS;
            }
            self.down_to_earth = self.loc.y < 0.1;
            if self.movement_forces.x > 0.0 {
                self.last_direction = HorizontalDirection::Right;
            } else if self.movement_forces.x < 0.0 {
                self.last_direction = HorizontalDirection::Left;
            }
            println!("{:#?}", self);
    }
}

#[derive(Derivative)]
#[derivative(Debug)]
pub struct Player {
    phys: PlayerPhys,
    #[derivative(Debug="ignore")]
    sprites: (Texture, Texture),
}

impl Player {
    pub fn new(loc: F64x2, mass: f64) -> Self {

        let player_image = ImageReader::open("assets/cursd_le_mon_smol.png")
            .unwrap()
            .decode()
            .unwrap()
            .to_rgba8();
        let player_image_upscaled = imageops::resize(
            &player_image,
            player_image.width() * 4,
            player_image.height() * 4,
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
            phys: PlayerPhys::new(loc, mass),
            sprites,
        }
    }

    pub fn draw(&mut self, c: &graphics::Context, gl: &mut GlGraphics, win_height: f64) {
        let globalize_physics_coord = move |coord: F64x2| -> F64x2 {
            F64x2 {
                x: coord.x,
                y: win_height - coord.y,
            }
        };

        use graphics::*;

        Image::new()
            .rect(rectangle_by_points(
                globalize_physics_coord(self.phys.loc * METERS_TO_POINTS)
                    - F64x2::new(12.0 * 4.0, 16.0 * 4.0),
                globalize_physics_coord(self.phys.loc * METERS_TO_POINTS),
            ))
            .draw(
                match self.phys.last_direction {
                    HorizontalDirection::Left => &self.sprites.0,
                    HorizontalDirection::Right => &self.sprites.1,
                },
                &graphics::DrawState::default(),
                c.transform,
                gl,
            );
    }

    pub fn update_phys(&mut self, dt: f64, win_size: [f64; 2]) {
        self.phys.update(dt, win_size);
    }
}

fn main() {
    pretty_env_logger::formatted_builder()
        .filter_level(log::LevelFilter::Debug)
        .init();

    info!("Initialized");

    // Change this to OpenGL::V2_1 if not working.
    let opengl = OpenGL::V4_5;

    const WIDTH: f64 = 1000.0;
    const HEIGHT: f64 = 700.0;

    // Create an Glutin window.
    let mut window: GlutinWindow = WindowSettings::new("platformR", [200, 200])
        .graphics_api(opengl)
        .exit_on_esc(true)
        .vsync(true)
        .resizable(false)
        .size(piston::Size {
            width: WIDTH,
            height: HEIGHT,
        })
        .controllers(true)
        .build()
        .unwrap();

    let mut gl = GlGraphics::new(opengl);

    let mut win_size = [0f64; 2];
    let mut player = Player::new(F64x2::splat(1.0), 1.0);


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

                player.draw(&c, gl, HEIGHT);
            });
        }
        if let Some(args) = e.update_args() {
            player.update_phys(args.dt, win_size);
        }
        const MOVE_FORCE: f64 = 10.0;
        if let Some(args) = e.press_args() {
            match args {
                Button::Mouse(mouse_btn) => match mouse_btn {
                    _ => {}
                },
                Button::Keyboard(keyboard_btn) => match keyboard_btn {
                    Key::A => {
                        player.phys.movement_forces += F64x2::new(-MOVE_FORCE, 0.0);
                    }
                    Key::D => {
                        player.phys.movement_forces += F64x2::new(MOVE_FORCE, 0.0);
                    }
                    Key::Space => {
                        // boing
                        if player.phys.down_to_earth {
                            player.phys.vel.y += 7.0;
                        }
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
                        player.phys.movement_forces -= F64x2::new(-MOVE_FORCE, 0.0);
                    }
                    Key::D => {
                        player.phys.movement_forces -= F64x2::new(MOVE_FORCE, 0.0);
                    }
                    _ => {}
                },
                _ => {}
            }
        }
    }
}
