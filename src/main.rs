pub mod colors;
pub mod constants;
pub mod player;
pub mod utils;
pub mod vec2;

#[macro_use]
extern crate log;
#[macro_use]
extern crate derivative;

use glutin_window::GlutinWindow;
use image::io::Reader as ImageReader;
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

    let map = ImageReader::open("assets/maps/sample_map_100x100.png")
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
            player.update_phys(args.dt, &map, map_px_to_meter);
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
