mod phys;

use image::{imageops, io::Reader as ImageReader};
use opengl_graphics::{GlGraphics, Texture, TextureSettings};

use crate::colors::*;
use crate::constants::*;
use crate::utils::rectangle_by_points;
use crate::vec2::F64x2;

use phys::{HorizontalDirection, PlayerPhys};

#[derive(Derivative)]
#[derivative(Debug)]
pub struct Player {
    pub phys: PlayerPhys,
    #[derivative(Debug = "ignore")]
    pub sprites: (Texture, Texture),
    // cfg values
    /// force added to y velocity on jumping
    pub jump_force: f64,
    pub move_force: f64,
    pub debug_phys: bool,
}

impl Player {
    pub fn new(loc: F64x2, mass: f64, jump_force: f64, move_force: f64) -> Self {
        let player_image = ImageReader::open("assets/player/cursd_le_mon_smol.png")
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
            debug_phys: false,
        }
    }

    pub fn draw(
        &mut self,
        c: &graphics::Context,
        gl: &mut GlGraphics,
        win_height: f64,
        map: &crate::WorldMap,
    ) {
        let map_px_to_meter = map.map_px_to_meter;
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
                c.transform.trans(
                    -map.cam_loc.x * METERS_TO_POINTS,
                    map.cam_loc.y * METERS_TO_POINTS,
                ),
                gl,
            );

        if self.debug_phys {
            Rectangle::new(rgba(0, 243, 223, 0.3)).draw(
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
                c.transform.trans(
                    -map.cam_loc.x * METERS_TO_POINTS,
                    map.cam_loc.y * METERS_TO_POINTS,
                ),
                gl,
            );

            //y min
            let mut render_y_min = |value: f64, offset: f64| {
                line_from_to(
                    rgba(0, 255, 0, 0.2),
                    5.0,
                    globalize_physics_cord(
                        (self.phys.loc + F64x2::new(offset, 0.0) * map_px_to_meter) * METERS_TO_POINTS,
                    ),
                    globalize_physics_cord(
                        (F64x2::new(self.phys.loc.x, value)
                            + F64x2::new(offset, 0.1 /* so it shows a small gap */) * map_px_to_meter)
                            * METERS_TO_POINTS,
                    ),
                    c.transform.trans(
                        -map.cam_loc.x * METERS_TO_POINTS,
                        map.cam_loc.y * METERS_TO_POINTS,
                    ),
                    gl,
                );
            };
            render_y_min(self.phys.y0_min, 0.0);
            render_y_min(self.phys.y1_min, 1.0);
            render_y_min(self.phys.y2_min, 2.0);
            // y max
            let mut render_y_max = |value: f64, offset: f64| {
                line_from_to(
                    rgba(0, 255, 0, 0.2),
                    5.0,
                    globalize_physics_cord(
                        (self.phys.loc + F64x2::new(offset, 3.0) * map_px_to_meter) * METERS_TO_POINTS,
                    ),
                    globalize_physics_cord(
                        (F64x2::new(self.phys.loc.x, value)
                            + F64x2::new(offset, -1.1 /* so it shows a small gap */) * map_px_to_meter)
                            * METERS_TO_POINTS,
                    ),
                    c.transform.trans(
                        -map.cam_loc.x * METERS_TO_POINTS,
                        map.cam_loc.y * METERS_TO_POINTS,
                    ),
                    gl,
                );
            };
            render_y_max(self.phys.y0_max, 0.0);
            render_y_max(self.phys.y1_max, 1.0);
            render_y_max(self.phys.y2_max, 2.0);
            // x min
            let mut render_x_min = |value: f64, offset: f64| {
                line_from_to(
                    rgba(0, 255, 0, 0.2),
                    5.0,
                    globalize_physics_cord(
                        (self.phys.loc + F64x2::new(0.0, offset) * map_px_to_meter) * METERS_TO_POINTS,
                    ),
                    globalize_physics_cord(
                        (F64x2::new(value, self.phys.loc.y)
                            + F64x2::new(0.1, offset) * map_px_to_meter)
                            * METERS_TO_POINTS,
                    ),
                    c.transform.trans(
                        -map.cam_loc.x * METERS_TO_POINTS,
                        map.cam_loc.y * METERS_TO_POINTS,
                    ),
                    gl,
                );
            };
            render_x_min(self.phys.x0_min, 0.0);
            render_x_min(self.phys.x1_min, 1.0);
            render_x_min(self.phys.x2_min, 2.0);
            render_x_min(self.phys.x3_min, 3.0);
            // x max
            let mut render_x_max = |value: f64, offset: f64| {
                line_from_to(
                    rgba(0, 255, 0, 0.2),
                    5.0,
                    globalize_physics_cord(
                        (self.phys.loc + F64x2::new(2.0, offset) * map_px_to_meter) * METERS_TO_POINTS,
                    ),
                    globalize_physics_cord(
                        (F64x2::new(value, self.phys.loc.y)
                            + F64x2::new(-0.1, offset) * map_px_to_meter)
                            * METERS_TO_POINTS,
                    ),
                    c.transform.trans(
                        -map.cam_loc.x * METERS_TO_POINTS,
                        map.cam_loc.y * METERS_TO_POINTS,
                    ),
                    gl,
                );
            };
            render_x_max(self.phys.x0_max, 0.0);
            render_x_max(self.phys.x1_max, 1.0);
            render_x_max(self.phys.x2_max, 2.0);
            render_x_max(self.phys.x3_max, 3.0);
        }
    }

    pub fn update_phys(&mut self, dt: f64, map: &crate::WorldMap) {
        self.phys.update(dt, map)
    }

    pub fn jump(&mut self) {
        self.phys.vel.y += self.jump_force;
    }
}
