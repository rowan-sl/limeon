pub mod colors;
pub mod constants;
pub mod player;
pub mod utils;
pub mod vec2;

#[macro_use]
extern crate log;
#[macro_use]
extern crate derivative;

use std::{collections::HashMap, path::Path};

use anyhow::Result;
use glutin_window::GlutinWindow;
use image::{io::Reader as ImageReader, ImageBuffer, Rgba};
use opengl_graphics::{GlGraphics, OpenGL};
use piston::{
    event_loop::{EventSettings, Events},
    window::WindowSettings,
    Button, Key, PressEvent, ReleaseEvent, RenderEvent, Size, UpdateEvent,
};

use colors::*;
use constants::*;
use player::Player;
use utils::*;
use vec2::F64x2;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TileEffectCondition {
    StandingOn,
    InsideOf,
}

#[derive(Debug, Clone, PartialEq)]
pub enum TileEffect {
    /// base collision type. this does not require any [`TileEffectCondition`] to take effect,
    /// it just signals that this is a collision block
    ///
    /// ( bounce factor, friction coeff )
    Collision(f64, F64x2),
    /// faster horizontal speed ( multiplier )
    HorizontalSpeedBoost(f64),
    /// enables the launch action ( launch strength )
    LaunchEnable(f64),
    /// constant force ( force )
    Wind(F64x2),
}

/// relationship between effects and conditions is as folows:
///
/// **any** condition being true means **all** effects apply
type TileEffectMap = HashMap<Rgba<u8>, (Vec<TileEffect>, Vec<TileEffectCondition>)>;

#[derive(Debug)]
pub struct WorldMap {
    map: ImageBuffer<Rgba<u8>, Vec<u8>>,
    effect_map: TileEffectMap,
    map_px_to_meter: f64,
    cam_loc: F64x2,
}

impl WorldMap {
    pub fn from_path<P: AsRef<Path>>(path: P, player: &Player) -> Result<Self> {
        let cam_loc = player.phys.loc.clone();

        let map = ImageReader::open(path)?.decode()?.to_rgba8();

        let map_px_to_meter = 1.0 / 5.0;

        let mut effect_map = HashMap::new();
        //TODO add a real way to load this
        effect_map.insert(
            Rgba([230, 180, 50, 255]),
            (
                vec![
                    TileEffect::Collision(0.0, F64x2::new(0.15, 0.0)),
                    TileEffect::HorizontalSpeedBoost(2.0),
                ],
                vec![TileEffectCondition::StandingOn],
            ),
        );
        effect_map.insert(
            Rgba([50, 222, 250, 255]),
            (
                vec![TileEffect::LaunchEnable(1.0)],
                vec![TileEffectCondition::InsideOf],
            ),
        );
        // basic void, must be here to prevent warnings
        effect_map.insert(Rgba([0; 4]), (vec![], vec![]));
        effect_map.insert(
            Rgba([255, 75, 125, 255]),
            (
                vec![TileEffect::Wind(F64x2::new(3.0, 7.0))],
                vec![TileEffectCondition::InsideOf],
            ),
        );
        // basic collision tile
        effect_map.insert(
            Rgba([255; 4]),
            (
                vec![TileEffect::Collision(0.15, F64x2::new(0.5, 0.0))],
                vec![],
            ),
        );
        effect_map.insert(
            Rgba([0, 255, 20, 255]),
            (
                vec![TileEffect::Collision(0.9, F64x2::new(0.8, 0.0))],
                vec![],
            ),
        );

        Ok(Self {
            map,
            effect_map,
            map_px_to_meter,
            cam_loc,
        })
    }

    pub fn render(&mut self, c: &graphics::Context, gl: &mut GlGraphics, win_size: [f64; 2]) {
        use graphics::*;

        let globalize_physics_cord = move |coord: F64x2| -> F64x2 {
            F64x2 {
                x: coord.x,
                y: win_size[1] * POINTS_TO_METERS - coord.y,
            }
        };

        for (raw_x, raw_y, px) in self.map.enumerate_pixels() {
            let x_pts = raw_x as f64 * self.map_px_to_meter;
            let y_pts = (self.map.height() - raw_y - 1) as f64 * self.map_px_to_meter;

            Rectangle::new(rgba(px.0[0], px.0[1], px.0[2], px.0[3] as f32 / 255.0)).draw(
                rectangle_by_points(
                    globalize_physics_cord(F64x2::new(x_pts, y_pts)) * METERS_TO_POINTS,
                    globalize_physics_cord(F64x2::new(
                        x_pts + self.map_px_to_meter,
                        y_pts + self.map_px_to_meter,
                    )) * METERS_TO_POINTS,
                ),
                &DrawState::default(),
                c.transform.trans(
                    -self.cam_loc.x * METERS_TO_POINTS,
                    self.cam_loc.y * METERS_TO_POINTS,
                ),
                gl,
            );
        }
    }
}

fn main() -> Result<()> {
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

    let mut player = Player::new(
        F64x2::splat(1.0),
        113.0 * GRAMS_TO_KG, /* about how much a large lemon weighs */
        5.0,
        2.0,
    );

    let mut map = WorldMap::from_path("assets/maps/limeon_test_map_2_100x100.png", &player)?;

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
                clear(rgba(128, 204, 204, 1.0), gl);

                map.render(&c, gl, win_size);
                player.draw(&c, gl, win_size[1], &map);
            });
        }

        if let Some(args) = e.update_args() {
            player.update_phys(args.dt, &map);
            map.cam_loc = F64x2 {
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
                    Key::Y => {
                        player.debug_phys = !player.debug_phys;
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

    Ok(())
}
