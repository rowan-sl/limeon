pub mod colors;
pub mod constants;
pub mod player;
pub mod utils;
pub mod vec2;
pub mod world;

#[macro_use]
extern crate log;
#[macro_use]
extern crate derivative;


use anyhow::Result;
use glutin_window::GlutinWindow;
use opengl_graphics::{GlGraphics, OpenGL};
use piston::{
    event_loop::{EventSettings, Events},
    window::WindowSettings,
    Button, Key, PressEvent, ReleaseEvent, RenderEvent, Size, UpdateEvent,
};

use colors::*;
use constants::*;
use player::Player;
use vec2::F64x2;
use world::{WorldMap, TileEffect, TileEffectCondition};

fn main() -> Result<()> {
    pretty_env_logger::formatted_builder()
        .filter_level(log::LevelFilter::Debug)
        .init();

    info!("Initialized");

    // Change this to OpenGL::V2_1 if not working.
    let opengl = OpenGL::V4_5;

    // Create an Glutin window.
    let mut window: GlutinWindow = WindowSettings::new("limeon", [200, 200])
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

    let mut map = WorldMap::from_path("assets/maps/limeon_test_map_3_100x100.png", &player)?;

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
