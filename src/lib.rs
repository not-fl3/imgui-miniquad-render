use miniquad::Context as QuadContext;
use miniquad::*;

use std::collections::HashSet;

use imgui::{DrawCmd, DrawCmdParams};

const MAX_VERTICES: usize = 10000;
const MAX_INDICES: usize = 5000;

struct Stage {    
    imgui: imgui::Context,
    last_frame: std::time::Instant,

    pipeline: Pipeline,
    font_texture: Texture,
    draw_calls: Vec<Bindings>,
    keys_pressed: HashSet<KeyCode>,

    f: Box<dyn FnMut(&mut imgui::Ui) -> ()>,
}

impl Stage {
    fn new(ctx: &mut QuadContext, f: Box<dyn FnMut(&mut imgui::Ui) -> ()>) -> Stage {
            let shader = Shader::new(ctx, shader::VERTEX, shader::FRAGMENT, shader::META);
    
            let pipeline = Pipeline::with_params(
                ctx,
                &[BufferLayout::default()],
                &[
                    VertexAttribute::new("position", VertexFormat::Float2),
                    VertexAttribute::new("texcoord", VertexFormat::Float2),
                    VertexAttribute::new("color0", VertexFormat::Byte4),
                ],
                shader,
                PipelineParams {
                    color_blend: Some((
                        Equation::Add,
                        BlendFactor::Value(BlendValue::SourceAlpha),
                        BlendFactor::OneMinusValue(BlendValue::SourceAlpha),
                    )),
                    ..Default::default()
                },
            );
    

            let mut imgui = imgui::Context::create();

            {
                use imgui::*;

                imgui.fonts().add_font(&[FontSource::DefaultFontData {
                    config: Some(FontConfig {
                        size_pixels: 13.0,
                        ..FontConfig::default()
                    }),
                }]);

                let (w, h) = ctx.screen_size();
                let mut io = imgui.io_mut();
                io.font_global_scale = 1.0;
                io.display_size = [w, h];
                io.mouse_pos = [0., 0.];
            }
    
            let font_texture = {
                let mut fonts = imgui.fonts();
                let texture = fonts.build_rgba32_texture();
    
                Texture::from_rgba8(texture.width as u16, texture.height as u16, texture.data)
            };

            Stage {
                imgui,
                pipeline,
                font_texture,
                last_frame: std::time::Instant::now(),
                draw_calls: Vec::with_capacity(200),    
                keys_pressed: HashSet::new(),
                f
            }
        }
    
    }

impl EventHandler for Stage {
    fn resize_event(&mut self, _ctx: &mut QuadContext, width: f32, height: f32) {
        let mut io = self.imgui.io_mut();
        io.display_size = [width, height];
    }

    fn key_down_event(&mut self, _: &mut QuadContext, keycode: KeyCode, _: KeyMods, _: bool) {
        self.keys_pressed.insert(keycode);
    }

    fn key_up_event(&mut self, _: &mut QuadContext, keycode: KeyCode, _: KeyMods) {
        self.keys_pressed.remove(&keycode);
    }

    fn mouse_motion_event(&mut self, _ctx: &mut QuadContext, x: f32, y: f32, _dx: f32, _dy: f32) {
        let mut io = self.imgui.io_mut();
        io.mouse_pos = [x, y];
    }
    fn mouse_wheel_event(&mut self, _ctx: &mut QuadContext, _x: f32, _y: f32) {}
    fn mouse_button_down_event(
        &mut self,
        _: &mut QuadContext,
        button: MouseButton,
        _x: f32,
        _y: f32,
    ) {
        let mut io = self.imgui.io_mut();
        let mouse_left = button == MouseButton::Left;
        let mouse_right = button == MouseButton::Right;
        io.mouse_down = [mouse_left, mouse_right, false, false, false];
    }
    fn mouse_button_up_event(
        &mut self,
        _: &mut QuadContext,
        _button: MouseButton,
        _x: f32,
        _y: f32,
    ) {
        let mut io = self.imgui.io_mut();
        io.mouse_down = [false, false, false, false, false];
    }

    fn update(&mut self, _ctx: &mut QuadContext) {}

    fn draw(&mut self, ctx: &mut QuadContext) {
        let draw_data = {
            let io = self.imgui.io_mut();
            self.last_frame = io.update_delta_time(self.last_frame);
            let mut ui = self.imgui.frame();
            (self.f)(&mut ui);
            ui.render()
        };

        let (width, height) = ctx.screen_size();
        let projection = glam::Mat4::orthographic_rh_gl(0., width, height, 0., -1., 1.);

        ctx.begin_default_pass(PassAction::clear_color(0.1, 0.1, 0.1, 0.));

        let clip_off = draw_data.display_pos;
        let clip_scale = draw_data.framebuffer_scale;

        for (n, draw_list) in draw_data.draw_lists().enumerate() {
            let vertices = draw_list.vtx_buffer();
            let indices = draw_list.idx_buffer();

            if n >= self.draw_calls.len() {
                let vertex_buffer = Buffer::stream(
                    ctx,
                    BufferType::VertexBuffer,
                    MAX_VERTICES * std::mem::size_of::<f32>(),
                );
                let index_buffer = Buffer::stream(
                    ctx,
                    BufferType::IndexBuffer,
                    MAX_INDICES * std::mem::size_of::<u16>(),
                );
                let bindings = Bindings {
                    vertex_buffers: vec![vertex_buffer],
                    index_buffer: index_buffer,
                    images: vec![],
                };
                self.draw_calls.push(bindings);
            }

            let dc = &mut self.draw_calls[n];

            dc.vertex_buffers[0].update(ctx, vertices);
            dc.index_buffer.update(ctx, indices);
            dc.images = vec![self.font_texture];

            let mut slice_start = 0;
            for cmd in draw_list.commands() {
                match cmd {
                    DrawCmd::Elements {
                        count,
                        cmd_params:
                            DrawCmdParams {
                                clip_rect,
                                ..
                            },
                    } => {
                        let clip_rect = [
                            (clip_rect[0] - clip_off[0]) * clip_scale[0],
                            (clip_rect[1] - clip_off[1]) * clip_scale[1],
                            (clip_rect[2] - clip_off[0]) * clip_scale[0],
                            (clip_rect[3] - clip_off[1]) * clip_scale[1],
                        ];
                        ctx.apply_pipeline(&self.pipeline);
                        let h = clip_rect[3] - clip_rect[1];

                        ctx.apply_scissor_rect(
                            clip_rect[0] as i32,
                            height as i32 - (clip_rect[1] + h) as i32,
                            (clip_rect[2] - clip_rect[0]) as i32,
                            h as i32,
                        );
                        
                        ctx.apply_bindings(&dc);
                        ctx.apply_uniforms(&shader::Uniforms { projection });
                        ctx.draw(slice_start, count as i32, 1);
                        slice_start += count as i32;
                    }
                    _ => {}
                }
            }
        }

        ctx.end_render_pass();

        ctx.commit_frame();
    }
}

pub struct Window {
    on_init: Option<Box<dyn FnOnce() -> ()>>,
}

impl Window {
    pub fn new(_label: &str) -> Window {
        Window { on_init: None }
    }

    pub fn on_init(self, f: impl FnOnce() -> ()) -> Self {
        let closure: Box<dyn FnOnce()> = Box::new(f);
        let closure: Box<dyn FnOnce() + 'static> = unsafe { std::mem::transmute(closure) };

        Self {
            on_init: Some(closure),
            ..self
        }
    }

    pub fn main_loop(mut self, f: impl FnMut(&mut imgui::Ui) -> ()) -> ! {
        let f = Box::new(f);

        // Allocate `clsoure` on the heap and erase the lifetime bound.
        // This is safe because we will never leave this function (alive)
        // The same applies for closure in on_init
        let closure: Box<dyn FnMut(&mut imgui::Ui)> = Box::new(f);
        let closure: Box<dyn FnMut(&mut imgui::Ui) + 'static> =
            unsafe { std::mem::transmute(closure) };

        miniquad::start(conf::Conf::default(), move |ctx| {
            if let Some(on_init) = self.on_init.take() {
                on_init();
            }

            Box::new(Stage::new(ctx, closure))
        });

        std::process::exit(0)
    }
}

pub use miniquad::KeyCode;

mod shader {
    use miniquad::{ShaderMeta, UniformBlockLayout, UniformType};

    pub const VERTEX: &str = r#"#version 100
    attribute vec2 position;
    attribute vec2 texcoord;
    attribute vec4 color0;

    varying lowp vec2 uv;
    varying lowp vec4 color;
    
    uniform mat4 Projection;

    void main() {
        gl_Position = Projection * vec4(position, 0, 1);
        gl_Position.z = 0.;
        color = color0 / 255.0;
        uv = texcoord;
    }"#;

    pub const FRAGMENT: &str = r#"#version 100
    varying lowp vec4 color;
    varying lowp vec2 uv;
    
    uniform sampler2D Texture;

    void main() {
        gl_FragColor = color * texture2D(Texture, uv);
    }"#;

    pub const META: ShaderMeta = ShaderMeta {
        images: &["Texture"],
        uniforms: UniformBlockLayout {
            uniforms: &[("Projection", UniformType::Mat4)],
        },
    };

    #[repr(C)]
    #[derive(Debug)]
    pub struct Uniforms {
        pub projection: glam::Mat4,
    }
}
