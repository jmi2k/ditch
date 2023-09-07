#![feature(isqrt)]
#![feature(iter_collect_into)]
#![feature(iterator_try_collect)]
#![feature(new_uninit)]
#![feature(slice_flatten)]
#![feature(variant_count)]

// jmi2k: rendering seems slow even though there are very few tris... too many VBOs?
// jmi2k: coordinate system seems backwards (culling, default direction camera points to)

mod assets;
mod chunk;
mod graphics;
mod input;
mod types;
mod world;

use std::{time::{Duration, Instant}, f32::consts::PI};

use chunk::Chunk;
use glam::{Quat, Vec3, ivec3, IVec3, ivec2};
use graphics::{Camera, GraphicsContext, Pov, Projection, Vertex, WorldRenderer};
use input::{Action, Direction3, Input, InputHandler};
use rand_xoshiro::rand_core::{SeedableRng, RngCore};
use winit::{
    event::*,
    event_loop::{ControlFlow, EventLoop},
    window::{CursorGrabMode, WindowBuilder, Fullscreen}, platform::run_return::EventLoopExtRunReturn,
};
use world::World;

use crate::chunk::{MESHING_TIMES, MESHING_DURATION, Mesher};

#[derive(Debug, Default)]
pub struct CameraController {
    pub camera: Camera,
    direction: Vec3,
}

impl CameraController {
    pub fn turn(&mut self, delta: (f32, f32)) {
        let (yaw, pitch) = delta;
        self.camera.turn(yaw / 2., pitch / 2.);
    }

    pub fn walk(&mut self, direction: Direction3) {
        self.direction += Vec3::from_array(direction.into());
    }

    pub fn stop(&mut self, direction: Direction3) {
        self.direction -= Vec3::from_array(direction.into());
    }

    pub fn tick(&mut self, delta: Duration) {
        let true_direction =
            Quat::from_rotation_z(-self.camera.pov.yaw) * self.direction.normalize_or_zero();

        self.camera.walk(true_direction * delta.as_secs_f32() * 6.);
    }
}

impl From<Camera> for CameraController {
    fn from(camera: Camera) -> Self {
        Self {
            camera,
            ..Self::default()
        }
    }
}

#[repr(i16)]
#[derive(Default)]
pub enum BlockId {
    #[default]
    Air,

    Bedrock,
    Stone,
    Dirt,
    Grass,
}

#[derive(Default)]
pub struct BlockData {
    pub id: BlockId,
}

#[pollster::main]
async fn main() {
    let mut event_loop = EventLoop::new();
    let window = WindowBuilder::new().build(&event_loop).unwrap();
    let pack = assets::open("pack").unwrap();
    let mut graphics_context = GraphicsContext::new(&window).await;
    let mut world_renderer = WorldRenderer::new(&graphics_context, &pack.atlases);

    #[rustfmt::skip]
    let mut input_handler = {
        use Input::*;
        use Action::*;

        InputHandler::from([
            (Scroll,                          Select),
            (Button(MouseButton::Right),      Place),
            (Press(VirtualKeyCode::Tab),      Fullscreen),
            (Press(VirtualKeyCode::Escape),   Pause),
            (Press(VirtualKeyCode::Q),        ExitGame),
            (Press(VirtualKeyCode::W),        Walk(Direction3::Forward)),
            (Press(VirtualKeyCode::S),        Walk(Direction3::Backward)),
            (Press(VirtualKeyCode::A),        Walk(Direction3::Left)),
            (Press(VirtualKeyCode::D),        Walk(Direction3::Right)),
            (Press(VirtualKeyCode::Space),    Walk(Direction3::Up)),
            (Press(VirtualKeyCode::LShift),   Walk(Direction3::Down)),
            (Release(VirtualKeyCode::W),      Stop(Direction3::Forward)),
            (Release(VirtualKeyCode::S),      Stop(Direction3::Backward)),
            (Release(VirtualKeyCode::A),      Stop(Direction3::Left)),
            (Release(VirtualKeyCode::D),      Stop(Direction3::Right)),
            (Release(VirtualKeyCode::Space),  Stop(Direction3::Up)),
            (Release(VirtualKeyCode::LShift), Stop(Direction3::Down)),
            (Button(MouseButton::Left),       Focus),
            (Motion,                          Turn),
        ])
    };

    let mut start = Instant::now();
    let mut world = World::default();

    let then = Instant::now();

    for k in -8..8 {
        for j in -16..16 {
            for i in -16..16 {
                world.loaded_chunks.insert([i, j, k], Chunk::generate(ivec3(i, j, k), &pack));
            }
        }
    }

    let ba = pack.blocks.binary_search_by(|(n, _)| n.as_str().cmp("air")).unwrap() as i16;
    let bw = pack.blocks.binary_search_by(|(n, _)| n.as_str().cmp("wood.toml")).unwrap() as i16;
    let bl = pack.blocks.binary_search_by(|(n, _)| n.as_str().cmp("leaves.toml")).unwrap() as i16;
    let b_grass = pack.blocks.binary_search_by(|(n, _)| n.as_str().cmp("grass.toml")).unwrap() as i16;

    let tree_model = [
        [
            [ba, ba, ba, ba, ba, ba, ba],
            [ba, ba, ba, ba, ba, ba, ba],
            [ba, ba, ba, bw, ba, ba, ba],
            [ba, ba, ba, ba, ba, ba, ba],
            [ba, ba, ba, ba, ba, ba, ba],
        ],
        [
            [ba, ba, ba, ba, ba, ba, ba],
            [ba, ba, ba, ba, ba, ba, ba],
            [ba, ba, bl, bw, ba, ba, ba],
            [ba, ba, ba, ba, ba, ba, ba],
            [ba, ba, ba, ba, ba, ba, ba],
        ],
        [
            [ba, ba, ba, bl, ba, ba, ba],
            [ba, bl, bl, bl, bl, ba, ba],
            [bl, bl, bw, bw, ba, bl, ba],
            [ba, bl, bl, bl, bl, ba, ba],
            [ba, ba, ba, ba, ba, ba, ba],
        ],
        [
            [ba, ba, bl, bl, bl, ba, ba],
            [bl, bl, bl, bw, bl, bl, ba],
            [bl, bw, bl, bw, bw, bl, ba],
            [bl, bl, bl, bl, bl, bl, ba],
            [ba, ba, ba, ba, ba, ba, ba],
        ],
        [
            [ba, ba, bl, bl, bl, ba, ba],
            [ba, bl, bl, bw, bl, bl, ba],
            [bl, bl, bl, bw, bl, bw, bl],
            [ba, bl, bl, bl, bl, bl, ba],
            [ba, ba, ba, bl, ba, ba, ba],
        ],
        [
            [ba, ba, ba, bl, ba, ba, ba],
            [ba, bl, bl, bl, bl, bl, ba],
            [ba, bl, bw, bw, bl, bw, bl],
            [ba, bl, bl, bw, bl, bl, ba],
            [ba, ba, bl, bl, bl, ba, ba],
        ],
        [
            [ba, ba, ba, ba, ba, ba, ba],
            [ba, bl, bl, bl, bl, bl, ba],
            [ba, bl, bw, bw, bl, bl, ba],
            [ba, bl, bl, bl, bl, bl, ba],
            [ba, ba, ba, bl, ba, ba, ba],
        ],
        [
            [ba, ba, ba, ba, ba, ba, ba],
            [ba, ba, bl, bl, ba, ba, ba],
            [ba, bl, bl, bl, bl, ba, ba],
            [ba, ba, bl, bl, ba, ba, ba],
            [ba, ba, ba, ba, ba, ba, ba],
        ],
    ];

    println!("terraforming {:?} average", then.elapsed() / (16*32*32));

    let mut initial_h = 255;
    loop {
        let loc = ivec3(0, 0, initial_h);
        let loc2: IVec3 = loc >> 5;
        let loc2 = loc2.to_array();
        if world.loaded_chunks.get(&loc2).unwrap()[loc & 31] != 0 {
            initial_h += 1;
            break;
        }
        initial_h -= 1;
    }
    let mut camera_controller = CameraController {
        direction: Default::default(),
        camera: Camera {
            // pov: Pov {
            //     position: (0., 0., initial_h as f32 + 2.).into(),
            //     yaw: 0.,
            //     pitch: 0.,
            // },
            pov: Pov {
                position: Vec3::new(
                    0.0,
                    0.0,
                    5.104397,
                ),
                yaw: 1.5799987,
                pitch: 0.33499983,
            },
            projection: Projection::Perspective {
                aspect: window.inner_size().width as f32 / window.inner_size().height as f32,
                fov: 90f32.to_radians(),
            },
        },
    };

    let mut micros = 0u128;
    let mut frames = 0;

    let mut mesher = Mesher::new();
    let mut selected_item = 0;

    event_loop.run_return(move |event, _, control_flow| {
        let mut action = Action::Nop;
        *control_flow = ControlFlow::Poll;
        let distance = 8;

        match event {
            Event::RedrawRequested(_) => {
                let then = Instant::now();

                for (location, mesh) in world.build_meshes(&mut mesher, camera_controller.camera.pov.position.as_ivec3(), &pack, distance) {
                    world_renderer.add_vertices(&graphics_context, location, &mesh);
                }

                world_renderer.remove_vertices(camera_controller.camera.pov.position.as_ivec3(), distance);

                //println!("meshing {:?} average", unsafe { MESHING_DURATION / MESHING_TIMES as u32 });
                let then = Instant::now();

                world_renderer
                    .render(&graphics_context, camera_controller.camera)
                    .unwrap();

                micros += then.elapsed().as_micros();
                frames += 1;
                //println!("render {:?}", then.elapsed());
            },

            Event::MainEventsCleared | Event::NewEvents(StartCause::Poll) => {
                let delta = start.elapsed();
                start = Instant::now();

                camera_controller.tick(delta);
                window.request_redraw()
            }

            Event::DeviceEvent { event, .. } => action = input_handler.handle_device(event),
            Event::WindowEvent { event, .. } => action = input_handler.handle_window(event),
            _ => {}
        }

        match action {
            Action::Focus => {
                window.set_cursor_grab(CursorGrabMode::Confined).unwrap();
                window.set_cursor_visible(false)
            }

            Action::Place => {
                let poss = camera_controller.camera.reach_ray();
                println!("{:?}", poss);
                let mut pos0 = poss[0];
                for pos in &poss[1..] {
                    let loc = pos.as_ivec3();
                    let x: IVec3 = loc >> 5;
                    let x = x.to_array();
                    let block = world.loaded_chunks.get(&x).unwrap()[loc];
                    if block != 0 {
                        let location = loc + IVec3::Z;
                        world.loaded_chunks.get_mut(&x).unwrap().place(location, selected_item as i16);
                        break;
                    }
                    pos0 = *pos;
                }
            }

            Action::Pause => {
                window.set_cursor_grab(CursorGrabMode::None).unwrap();
                window.set_cursor_visible(true)
            }

            Action::Select => {
                let (_, dy) = input_handler.scroll_delta();
                let count = dy as isize;
                selected_item = (selected_item + count).rem_euclid(pack.blocks.len() as isize);
                println!("selected item is {}", pack.blocks[selected_item as usize].0);
            }

            Action::Fullscreen if window.fullscreen().is_none() => {
                let mode = Fullscreen::Borderless(None);
                window.set_fullscreen(Some(mode));
            }

            Action::Fullscreen if window.fullscreen().is_some() => {
                window.set_fullscreen(None);
            }

            Action::Resize { width, height } => {
                graphics_context.resize_viewport(width, height);
                camera_controller.camera.projection = Projection::Perspective {
                    aspect: width as f32 / height as f32,
                    fov: 90f32.to_radians(),
                };
            }
            Action::ExitGame => {
                println!("{} fps average", 1_000_000. / (micros / frames) as f32);
                println!("{:#?}", camera_controller.camera.pov);
                *control_flow = ControlFlow::Exit
            },

            Action::Turn => camera_controller.turn(input_handler.cursor_delta()),
            Action::Walk(direction) => camera_controller.walk(direction),
            Action::Stop(direction) => camera_controller.stop(direction),
            _ => {}
        }
    });
}
