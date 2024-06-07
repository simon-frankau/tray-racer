//
// Basic display-some-GL-with-egui-on-top code, created by ripping off
// my curved-spaces code and removing as much as I easily could.
//

use std::path::Path;

use anyhow::*;
use clap::Parser;
use glow::{Context, *};

use tray_racer_lib::{CanvasConfig, EnvMap, Tracer};

////////////////////////////////////////////////////////////////////////
// Command-line args

// TODO: Still need to finalise and source-control these.
const DEFAULT_ENV_MAP_POS: &str = "skyboxes/beach-skyboxes/HeartInTheSand";
const DEFAULT_ENV_MAP_NEG: &str = "skyboxes/night-skyboxes/PondNight";

/// Program to allow you to view distorted space
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// Directory containing positive-w env maps
    #[arg(long, default_value_t = DEFAULT_ENV_MAP_POS.to_string())]
    env_map_pos: String,
    /// Directory containing negative-w env maps
    #[arg(long, default_value_t = DEFAULT_ENV_MAP_NEG.to_string())]
    env_map_neg: String,
}

////////////////////////////////////////////////////////////////////////
// Shape: Representation of something to be drawn in OpenGL with a
// single `draw_elements` call.
//

pub struct Shape {
    vao: VertexArray,
    vbo: Buffer,
    ibo: Buffer,
    num_elts: i32,
}

impl Shape {
    // Create vertex and index buffers, and vertex array to describe vertex buffer.
    fn new(gl: &Context) -> Shape {
        unsafe {
            // We construct buffer, data will be uploaded later.
            let ibo = gl.create_buffer().unwrap();
            let vbo = gl.create_buffer().unwrap();
            gl.bind_buffer(glow::ARRAY_BUFFER, Some(vbo));

            // We now construct a vertex array to describe the format of the input buffer
            let vao = gl.create_vertex_array().unwrap();
            gl.bind_vertex_array(Some(vao));
            gl.vertex_attrib_pointer_f32(
                0,
                2,
                glow::FLOAT,
                false,
                core::mem::size_of::<f32>() as i32 * 2,
                0,
            );

            Shape {
                vbo,
                vao,
                ibo,
                num_elts: 0,
            }
        }
    }

    fn rebuild(&mut self, gl: &Context, vertices: &[f32], indices: &[u32]) {
        unsafe {
            let vertices_u8: &[u8] = core::slice::from_raw_parts(
                vertices.as_ptr() as *const u8,
                std::mem::size_of_val(vertices),
            );
            gl.bind_buffer(glow::ARRAY_BUFFER, Some(self.vbo));
            gl.buffer_data_u8_slice(glow::ARRAY_BUFFER, vertices_u8, glow::STATIC_DRAW);

            let indices_u8: &[u8] = core::slice::from_raw_parts(
                indices.as_ptr() as *const u8,
                std::mem::size_of_val(indices),
            );
            gl.bind_buffer(glow::ELEMENT_ARRAY_BUFFER, Some(self.ibo));
            gl.buffer_data_u8_slice(glow::ELEMENT_ARRAY_BUFFER, indices_u8, glow::STATIC_DRAW);

            self.num_elts = indices.len() as i32;
        }
    }

    pub fn draw(&self, gl: &Context, gl_type: u32) {
        // Assumes program, uniforms, etc. are set.
        unsafe {
            gl.bind_vertex_array(Some(self.vao));
            gl.enable_vertex_attrib_array(0);
            gl.bind_buffer(glow::ELEMENT_ARRAY_BUFFER, Some(self.ibo));
            gl.draw_elements(gl_type, self.num_elts, glow::UNSIGNED_INT, 0);
            gl.disable_vertex_attrib_array(0);
        }
    }

    fn close(&self, gl: &Context) {
        unsafe {
            gl.delete_vertex_array(self.vao);
            gl.delete_buffer(self.vbo);
            gl.delete_buffer(self.ibo);
        }
    }
}

////////////////////////////////////////////////////////////////////////
// winit: Shared between wasm32 and glutin_winit.
//

#[derive(Debug)]
pub enum UserEvent {
    Redraw(std::time::Duration),
}

struct Platform {
    gl: std::sync::Arc<Context>,
    shader_version: &'static str,
    window: winit::window::Window,
    event_loop: Option<winit::event_loop::EventLoop<UserEvent>>,

    gl_surface: glutin::surface::Surface<glutin::surface::WindowSurface>,
    gl_context: glutin::context::PossiblyCurrentContext,
}

impl Platform {
    fn run(mut self, mut drawable: Drawable) {
        use winit::event::*;

        // `run` "uses up" the event_loop, so we move it out.
        let mut event_loop = None;
        std::mem::swap(&mut event_loop, &mut self.event_loop);
        let event_loop = event_loop.expect("Event loop already run");

        let mut egui_glow =
            egui_glow::winit::EguiGlow::new(&event_loop, self.gl.clone(), None, None);

        let event_loop_proxy = egui::mutex::Mutex::new(event_loop.create_proxy());
        egui_glow
            .egui_ctx
            .set_request_repaint_callback(move |info| {
                event_loop_proxy
                    .lock()
                    .send_event(UserEvent::Redraw(info.delay))
                    .expect("Cannot send event");
            });

        let mut repaint_delay = std::time::Duration::MAX;
        // Used to detect dragging on the image to pan.
        let mut left_button_down = false;

        let event_fn =
            move |event,
                  event_loop_window_target: &winit::event_loop::EventLoopWindowTarget<
                UserEvent,
            >| {
                let mut redraw = || {
                    let mut quit = false;

                    egui_glow.run(&self.window, |egui_ctx| {
                        drawable.ui(egui_ctx, &self.gl);
                    });

                    if quit {
                        event_loop_window_target.exit();
                    } else {
                        event_loop_window_target.set_control_flow(if repaint_delay.is_zero() {
                            self.window.request_redraw();
                            winit::event_loop::ControlFlow::Poll
                        } else if let Some(repaint_after_instant) =
                            web_time::Instant::now().checked_add(repaint_delay)
                        {
                            winit::event_loop::ControlFlow::WaitUntil(repaint_after_instant)
                        } else {
                            winit::event_loop::ControlFlow::Wait
                        });
                    }

                    {
                        unsafe {
                            use glow::HasContext as _;
                            // self.gl.clear_color(clear_color[0], clear_color[1], clear_color[2], 1.0);
                            self.gl.clear(glow::COLOR_BUFFER_BIT);
                        }

                        // draw things behind egui here
                        let size = self.window.inner_size();
                        drawable.draw(&self.gl, size.width, size.height);

                        egui_glow.paint(&self.window);

                        // draw things on top of egui here

                        self.swap_buffers();
                    }
                };

                let mut needs_retex = false;
                match event {
                    Event::WindowEvent { event, .. } => {
                        if matches!(event, WindowEvent::CloseRequested | WindowEvent::Destroyed) {
                            event_loop_window_target.exit();
                            return;
                        }

                        if matches!(event, WindowEvent::RedrawRequested) {
                            redraw();
                            return;
                        }

                        if let WindowEvent::Resized(physical_size) = &event {
                            self.resize(physical_size);
                        }

                        // We're interacting if the mouse button is
                        // down, whether it's in an egui element or
                        // not.
                        if let WindowEvent::MouseInput {
                            button: MouseButton::Left,
                            state,
                            ..
                        } = &event
                        {
                            drawable.fast_draw = *state == ElementState::Pressed;
                            if !drawable.fast_draw {
                                needs_retex = true;
                            }
                        }

                        let event_response = egui_glow.on_window_event(&self.window, &event);

                        if event_response.repaint {
                            self.window.request_redraw();
                        }

                        if !event_response.consumed {
                            match event {
                                // We check the WindowEvent rather
                                // than the DeviceEvent in order to
                                // allow egui to consume it first.
                                WindowEvent::MouseInput { state, button, .. } => {
                                    if button == MouseButton::Left {
                                        left_button_down = state == ElementState::Pressed;
                                    }
                                }
                                // We will make use of keyboard
                                // auto-repeat for movement, rather
                                // than doing our own key-held
                                // logic. As we're using WASD keys,
                                // we'll use the PhysicalKey.
                                WindowEvent::KeyboardInput { event, .. } => {
                                    use winit::keyboard::*;
                                    if let KeyEvent {
                                        physical_key: PhysicalKey::Code(k),
                                        state: ElementState::Pressed,
                                        ..
                                    } = event
                                    {
                                        match k {
                                            KeyCode::KeyW => {}
                                            KeyCode::KeyS => {}
                                            KeyCode::KeyA => {}
                                            KeyCode::KeyD => {}
                                            KeyCode::KeyQ => {}
                                            KeyCode::KeyE => {}
                                            _ => {}
                                        }
                                    }
                                }
                                _ => {}
                            }
                        }
                    }

                    Event::DeviceEvent { event, .. } => {
                        if left_button_down {
                            // DeviceEvent is better than WindowEvent for
                            // this kind of camera dragging, according to
                            // the docs.
                            if let DeviceEvent::MouseMotion { delta } = event {
                                let size = self.window.inner_size();
                                let x = delta.0 * 360.0 / size.width as f64;
                                let y = delta.1 * 180.0 / size.height as f64;

                                let turn = &mut drawable.turn;
                                let tilt = &mut drawable.tilt;
                                *turn -= x;
                                if *turn > 180.0 {
                                    *turn -= 360.0;
                                } else if *turn < -180.0 {
                                    *turn += 360.0;
                                }
                                *tilt = (*tilt + y).min(90.0).max(-90.0);

                                drawable.rebuild_tex(&self.gl)
                            }
                        }
                    }

                    Event::UserEvent(UserEvent::Redraw(delay)) => {
                        repaint_delay = delay;
                    }
                    Event::LoopExiting => {
                        egui_glow.destroy();
                        drawable.close(&self.gl);
                    }
                    Event::NewEvents(StartCause::ResumeTimeReached { .. }) => {
                        self.window.request_redraw();
                    }

                    _ => (),
                }
                // Do this right after everything else in the event
                // loop, so we get a nice high-res redraw after all
                // the interaction-processing.
                if needs_retex {
                    drawable.rebuild_tex(&self.gl);
                }
            };

        Self::run_event_loop(event_loop, event_fn);
    }
}

impl Platform {
    fn new(width: u32, height: u32, name: &str) -> Result<Platform> {
        use glutin::{
            config::{ConfigTemplateBuilder, GlConfig},
            context::{ContextApi, ContextAttributesBuilder, NotCurrentGlContext},
            display::{GetGlDisplay, GlDisplay},
            surface::{GlSurface, SwapInterval},
        };
        use glutin_winit::{DisplayBuilder, GlWindow};
        use raw_window_handle::HasRawWindowHandle;
        use std::num::NonZeroU32;

        let event_loop =
            winit::event_loop::EventLoopBuilder::<UserEvent>::with_user_event().build()?;

        let window_builder = winit::window::WindowBuilder::new()
            .with_title(name)
            .with_inner_size(winit::dpi::LogicalSize::new(width as f32, height as f32));
        let template = ConfigTemplateBuilder::new();
        let display_builder = DisplayBuilder::new().with_window_builder(Some(window_builder));
        let (window, gl_config) = display_builder
            .build(&event_loop, template, |configs| {
                configs
                    .reduce(|accum, config| {
                        if config.num_samples() > accum.num_samples() {
                            config
                        } else {
                            accum
                        }
                    })
                    .unwrap()
            })
            .map_err(|_| anyhow!("Couldn't build display"))?;

        let raw_window_handle = window.as_ref().map(|window| window.raw_window_handle());

        let window = window.ok_or_else(|| anyhow!("Couldn't get window"))?;

        let gl_display = gl_config.display();
        let context_attributes = ContextAttributesBuilder::new()
            .with_context_api(ContextApi::OpenGl(Some(glutin::context::Version {
                major: 4,
                minor: 10,
            })))
            .build(raw_window_handle);

        let (gl, gl_surface, gl_context) = unsafe {
            let not_current_gl_context =
                gl_display.create_context(&gl_config, &context_attributes)?;
            let attrs = window.build_surface_attributes(Default::default());
            let gl_surface = gl_display.create_window_surface(&gl_config, &attrs)?;
            let gl_context = not_current_gl_context.make_current(&gl_surface)?;
            let gl = glow::Context::from_loader_function_cstr(|s| gl_display.get_proc_address(s));
            (gl, gl_surface, gl_context)
        };

        gl_surface
            .set_swap_interval(&gl_context, SwapInterval::Wait(NonZeroU32::new(1).unwrap()))?;

        Ok(Platform {
            gl: std::sync::Arc::new(gl),
            shader_version: "#version 410",
            window,
            event_loop: Some(event_loop),

            gl_surface,
            gl_context,
        })
    }

    fn swap_buffers(&self) {
        use glutin::prelude::GlSurface;
        self.gl_surface.swap_buffers(&self.gl_context).unwrap();
        self.window.set_visible(true);
    }

    fn resize(&self, physical_size: &winit::dpi::PhysicalSize<u32>) {
        // In a native window, resizing the window changes both
        // logical and physical size. Thus the ratio stays the same,
        // and the egui interface stays the same size. Zoom is handled
        // separately, and works like web zoom.
        use glutin::prelude::GlSurface;
        self.gl_surface.resize(
            &self.gl_context,
            physical_size.width.try_into().unwrap(),
            physical_size.height.try_into().unwrap(),
        );
    }

    fn run_event_loop(
        event_loop: winit::event_loop::EventLoop<UserEvent>,
        event_fn: impl FnMut(
                winit::event::Event<UserEvent>,
                &winit::event_loop::EventLoopWindowTarget<UserEvent>,
            ) + 'static,
    ) {
        let _ = event_loop.run(event_fn);
    }
}

////////////////////////////////////////////////////////////////////////
// Main code.
//

const NAME: &str = "Tray Racer";
const WIDTH: u32 = 1024;
const HEIGHT: u32 = 768;

fn main() -> Result<()> {
    env_logger::init();

    let args = Args::parse();
    let mut p = Platform::new(WIDTH, HEIGHT, NAME)?;

    let drawable = Drawable::new(
        &p.gl,
        p.shader_version,
        Path::new(&args.env_map_pos),
        Path::new(&args.env_map_neg),
    );

    unsafe {
        p.gl.clear_color(0.1, 0.2, 0.3, 1.0);
    }

    // `run` should call `drawable.close(&p.gl)` when done. We don't
    // call it here, as `run` may run the event loop asynchronously
    // (e.g. for web).
    p.run(drawable);

    Ok(())
}

struct Drawable {
    tracer: Tracer,
    program: Program,
    tilt: f64,
    turn: f64,
    shape: Shape,
    tex: Texture,
    fast_draw: bool,
}

const VERT_SRC: &str = include_str!("shader/vertex.glsl");
const FRAG_SRC: &str = include_str!("shader/fragment.glsl");

// The tracing resolution used during interactive updates.
const FAST_RES: usize = 128;

impl Drawable {
    fn new(
        gl: &Context,
        shader_version: &str,
        env_map_path_pos: &Path,
        env_map_path_neg: &Path,
    ) -> Drawable {
        let env_map_pos = EnvMap::from(env_map_path_pos).unwrap();
        let env_map_neg = EnvMap::from(env_map_path_neg).unwrap();

        unsafe {
            let program = gl.create_program().expect("Cannot create program");

            let shader_sources = [
                (glow::VERTEX_SHADER, VERT_SRC),
                (glow::FRAGMENT_SHADER, FRAG_SRC),
            ];

            let mut shaders = Vec::with_capacity(shader_sources.len());

            for (shader_type, shader_source) in shader_sources.iter() {
                let shader = gl
                    .create_shader(*shader_type)
                    .expect("Cannot create shader");
                gl.shader_source(shader, &format!("{}\n{}", shader_version, shader_source));
                gl.compile_shader(shader);
                if !gl.get_shader_compile_status(shader) {
                    panic!("{}", gl.get_shader_info_log(shader));
                }
                gl.attach_shader(program, shader);
                shaders.push(shader);
            }

            gl.link_program(program);
            if !gl.get_program_link_status(program) {
                panic!("{}", gl.get_program_info_log(program));
            }

            for shader in shaders {
                gl.detach_shader(program, shader);
                gl.delete_shader(shader);
            }

            let mut shape = Shape::new(gl);

            shape.rebuild(gl, &[0.0, 0.0, 1.0, 0.0, 1.0, 1.0, 0.0, 1.0], &[0, 1, 3, 2]);

            let tex = gl.create_texture().unwrap();

            let drawable = Drawable {
                tracer: Tracer {
                    env_map_pos,
                    env_map_neg,
                    w_scale: 0.25,
                    radius: 0.1,
                    infinity: 4.0,
                },
                program,
                tilt: 0.0,
                turn: 0.0,
                shape,
                tex,
                fast_draw: false,
            };
            drawable.rebuild_tex(gl);
            drawable
        }
    }

    fn ui(&mut self, ctx: &egui::Context, gl: &Context) {
        egui::Window::new("Controls").show(ctx, |ui| {
            // TODO
            // if ui.button("Quit").clicked() {}
            let mut need_retex = false;
            need_retex |= ui
                .add(egui::Slider::new(&mut self.tilt, -90.0..=90.0).text("Tilt"))
                .changed();
            need_retex |= ui
                .add(egui::Slider::new(&mut self.turn, -180.0..=180.0).text("Turn"))
                .changed();
            need_retex |= ui
                .add(egui::Slider::new(&mut self.tracer.radius, -1.0..=1.0).text("Wormhole radius"))
                .changed();
            need_retex |= ui
                .add(egui::Slider::new(&mut self.tracer.w_scale, 0.1..=1.0).text("Smoothness"))
                .changed();
            need_retex |= ui
                .add(egui::Slider::new(&mut self.tracer.infinity, 1.0..=10.0).text("Infinity"))
                .changed();

            if need_retex {
                self.rebuild_tex(gl);
            }
        });
    }

    fn rebuild_tex(&self, gl: &Context) {
        // TODO: Pull this from the context or whatever.
        let (base_w, base_h) = (1024, 768);
        let (w, h) = if self.fast_draw {
            (FAST_RES, FAST_RES * base_h / base_w)
        } else {
            (base_w, base_h)
        };

        let tex_data = self.tracer.render(
            &CanvasConfig {
                width: w,
                height: h,
                aspect: 1.0,
                fov_degrees: 90.0,
            },
            self.tilt,
            self.turn,
        );

        unsafe {
            gl.bind_texture(glow::TEXTURE_2D, Some(self.tex));

            gl.tex_image_2d(
                glow::TEXTURE_2D,
                0,
                glow::RGBA as i32,
                w as i32,
                h as i32,
                0,
                glow::RGBA,
                glow::UNSIGNED_BYTE,
                Some(&tex_data),
            );
            gl.generate_mipmap(glow::TEXTURE_2D);
        }
    }

    fn draw(&mut self, gl: &Context, width: u32, height: u32) {
        unsafe {
            gl.viewport(0, 0, width as i32, height as i32);
            gl.use_program(Some(self.program));
            gl.bind_texture(glow::TEXTURE_2D, Some(self.tex));
            self.shape.draw(gl, glow::TRIANGLE_STRIP);
        }
    }

    fn close(&self, gl: &Context) {
        unsafe {
            gl.delete_program(self.program);
        }
        self.shape.close(gl);
    }
}
