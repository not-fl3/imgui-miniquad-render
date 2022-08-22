use miniquad::Context as QuadContext;
use miniquad::*;

use clipboard::ClipboardProvider;

use imgui::{ClipboardBackend, DrawCmd, DrawCmdParams, DrawVert};

const MAX_VERTICES: usize = 30000;
const MAX_INDICES: usize = 50000;

struct Stage {
    imgui: imgui::Context,
    last_frame: std::time::Instant,

    pipeline: Pipeline,
    font_texture: Texture,
    draw_calls: Vec<Bindings>,

    on_draw: Box<dyn FnMut(&mut imgui::Ui) -> ()>,
    on_quit: Option<Box<dyn FnOnce() -> ()>>,
}

/// Clipboard support from the clipboard crate
struct ClipboardSupport(pub clipboard::ClipboardContext);
impl ClipboardBackend for ClipboardSupport {
    fn get(&mut self) -> Option<String> {
        self.0.get_contents().ok()
    }

    fn set(&mut self, text: &str) {
        let _ = self.0.set_contents(text.to_owned());
    }
}

/// Platform dependent APIs not directly connected to imgui
pub mod platform {
    use miniquad::Context as QuadContext;
    use std::{cell::RefCell, rc::Rc};

    static mut QUAD_CONTEXT: Option<Rc<RefCell<QuadContext>>> = None;

    pub(crate) fn set_ctx(ctx: Rc<RefCell<QuadContext>>) {
        unsafe { QUAD_CONTEXT = Some(ctx) };
    }

    /// Close window. "quit" event will be triggered.
    pub fn request_quit() {
        unsafe { QUAD_CONTEXT.as_ref() }
            .unwrap()
            .borrow_mut()
            .request_quit();
    }
}

impl Stage {
    fn new(
        ctx: &mut QuadContext,
        on_draw: Box<dyn FnMut(&mut imgui::Ui) -> ()>,
        on_init: Option<Box<dyn FnOnce(&mut imgui::Context)>>,
        on_quit: Option<Box<dyn FnOnce()>>,
    ) -> Stage {
        let shader = Shader::new(ctx, shader::VERTEX, shader::FRAGMENT, shader::meta()).unwrap();

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
                color_blend: Some(BlendState::new(
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
                    rasterizer_multiply: 1.75,
                    ..FontConfig::default()
                }),
            }]);

            let (w, h) = ctx.screen_size();
            let mut io = imgui.io_mut();

            io[Key::Tab] = KeyCode::Tab as _;
            io[Key::LeftArrow] = KeyCode::Left as _;
            io[Key::RightArrow] = KeyCode::Right as _;
            io[Key::UpArrow] = KeyCode::Up as _;
            io[Key::DownArrow] = KeyCode::Down as _;
            io[Key::PageUp] = KeyCode::PageUp as _;
            io[Key::PageDown] = KeyCode::PageDown as _;
            io[Key::Home] = KeyCode::Home as _;
            io[Key::End] = KeyCode::End as _;
            io[Key::Insert] = KeyCode::Insert as _;
            io[Key::Delete] = KeyCode::Delete as _;
            io[Key::Backspace] = KeyCode::Backspace as _;
            io[Key::Space] = KeyCode::Space as _;
            io[Key::Enter] = KeyCode::Enter as _;
            io[Key::Escape] = KeyCode::Escape as _;
            io[Key::KeyPadEnter] = KeyCode::KpEnter as _;
            io[Key::A] = KeyCode::A as _;
            io[Key::C] = KeyCode::C as _;
            io[Key::V] = KeyCode::V as _;
            io[Key::X] = KeyCode::X as _;
            io[Key::Y] = KeyCode::Y as _;
            io[Key::Z] = KeyCode::Z as _;

            io.font_global_scale = 1.0;
            io.display_size = [w, h];
            io.mouse_pos = [0., 0.];
        }

        let font_texture = {
            let mut fonts = imgui.fonts();
            let texture = fonts.build_rgba32_texture();

            Texture::from_rgba8(
                ctx,
                texture.width as u16,
                texture.height as u16,
                texture.data,
            )
        };

        if let Some(clip_backend) = clipboard::ClipboardContext::new()
            .ok()
            .map(ClipboardSupport)
        {
            imgui.set_clipboard_backend(clip_backend);
        } else {
            eprintln!("failed to initialize clipboard!");
        }

        //platform::set_ctx(ctx.clone());

        if let Some(on_init) = on_init {
            on_init(&mut imgui);
        }

        Stage {
            imgui,
            pipeline,
            font_texture,
            last_frame: std::time::Instant::now(),
            draw_calls: Vec::with_capacity(200),
            on_draw,
            on_quit,
        }
    }
}

impl EventHandler for Stage {
    fn resize_event(&mut self, _ctx: &mut QuadContext, width: f32, height: f32) {
        let mut io = self.imgui.io_mut();
        io.display_size = [width, height];
    }

    fn char_event(&mut self, _ctx: &mut QuadContext, character: char, mods: KeyMods, _: bool) {
        let io = self.imgui.io_mut();

        io.key_ctrl = mods.ctrl;
        io.key_alt = mods.alt;
        io.key_shift = mods.shift;

        io.add_input_character(character);
    }

    fn key_down_event(&mut self, _ctx: &mut QuadContext, keycode: KeyCode, mods: KeyMods, _: bool) {
        let mut io = self.imgui.io_mut();

        // when the keycode is the modifier itself - mods.MODIFIER is false yet, however the modifier button is just pressed and is actually true
        io.key_ctrl = mods.ctrl;
        io.key_alt = mods.alt;
        io.key_shift = mods.shift;

        io.keys_down[keycode as usize] = true;
    }

    fn key_up_event(&mut self, _ctx: &mut QuadContext, keycode: KeyCode, mods: KeyMods) {
        let mut io = self.imgui.io_mut();

        // when the keycode is the modifier itself - mods.MODIFIER is true, however the modifier is actually released
        io.key_ctrl =
            keycode != KeyCode::LeftControl && keycode != KeyCode::RightControl && mods.ctrl;
        io.key_alt = keycode != KeyCode::LeftAlt && keycode != KeyCode::RightAlt && mods.alt;
        io.key_shift =
            keycode != KeyCode::LeftShift && keycode != KeyCode::RightShift && mods.shift;

        io.keys_down[keycode as usize] = false;
    }

    fn mouse_motion_event(&mut self, _ctx: &mut QuadContext, x: f32, y: f32) {
        let mut io = self.imgui.io_mut();
        io.mouse_pos = [x, y];
    }
    fn mouse_wheel_event(&mut self, _ctx: &mut QuadContext, _x: f32, y: f32) {
        let mut io = self.imgui.io_mut();
        io.mouse_wheel = y;
    }
    fn mouse_button_down_event(
        &mut self,
        _ctx: &mut QuadContext,
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
        _ctx: &mut QuadContext,
        _button: MouseButton,
        _x: f32,
        _y: f32,
    ) {
        let mut io = self.imgui.io_mut();
        io.mouse_down = [false, false, false, false, false];
    }

    fn update(&mut self, _ctx: &mut QuadContext) {}

    fn quit_requested_event(&mut self, _ctx: &mut QuadContext) {
        if let Some(on_quit) = self.on_quit.take() {
            on_quit();
        }
    }

    fn draw(&mut self, ctx: &mut QuadContext) {
        let draw_data = {
            let io = self.imgui.io_mut();
            let now = std::time::Instant::now();
            io.update_delta_time(now.duration_since(self.last_frame));
            self.last_frame = now;

            let mut ui = self.imgui.frame();
            (self.on_draw)(&mut ui);

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
                    MAX_VERTICES * std::mem::size_of::<DrawVert>(),
                );
                let index_buffer = Buffer::stream(
                    ctx,
                    BufferType::IndexBuffer,
                    MAX_INDICES * std::mem::size_of::<u16>(),
                );
                let bindings = Bindings {
                    vertex_buffers: vec![vertex_buffer],
                    index_buffer,
                    images: vec![],
                };
                self.draw_calls.push(bindings);
            }

            let dc = &mut self.draw_calls[n];

            if vertices.len() * std::mem::size_of::<DrawVert>() > dc.vertex_buffers[0].size() {
                println!("imgui: Vertex buffer too small, reallocating");

                dc.vertex_buffers[0] = Buffer::stream(
                    ctx,
                    BufferType::VertexBuffer,
                    vertices.len() * std::mem::size_of::<DrawVert>(),
                );
            }

            if indices.len() * std::mem::size_of::<u16>() > dc.index_buffer.size() {
                println!("imgui: Index buffer too small, reallocating");

                dc.index_buffer = Buffer::stream(
                    ctx,
                    BufferType::IndexBuffer,
                    indices.len() * std::mem::size_of::<u16>() * std::mem::size_of::<u16>(),
                );
            }

            dc.vertex_buffers[0].update(ctx, vertices);
            dc.index_buffer.update(ctx, indices);
            dc.images = vec![self.font_texture];

            let mut slice_start = 0;
            for cmd in draw_list.commands() {
                match cmd {
                    DrawCmd::Elements {
                        count,
                        cmd_params: DrawCmdParams { clip_rect, .. },
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
    on_init: Option<Box<dyn FnOnce(&mut imgui::Context) -> ()>>,
    on_quit: Option<Box<dyn FnOnce() -> ()>>,
}

impl Window {
    pub fn new(_label: &str) -> Window {
        Window {
            on_init: None,
            on_quit: None,
        }
    }

    pub fn on_init(self, f: impl FnOnce(&mut imgui::Context)) -> Self {
        let closure: Box<dyn FnOnce(&mut imgui::Context)> = Box::new(f);
        let closure: Box<dyn FnOnce(&mut imgui::Context) + 'static> =
            unsafe { std::mem::transmute(closure) };

        Self {
            on_init: Some(closure),
            ..self
        }
    }

    pub fn on_quit(self, f: impl FnOnce()) -> Self {
        let closure: Box<dyn FnOnce()> = Box::new(f);
        let closure: Box<dyn FnOnce() + 'static> = unsafe { std::mem::transmute(closure) };

        Self {
            on_quit: Some(closure),
            ..self
        }
    }

    pub fn main_loop(self, on_draw: impl FnMut(&mut imgui::Ui) -> ()) -> ! {
        let on_draw = Box::new(on_draw);

        // Allocate `closure` on the heap and erase the lifetime bound.
        // This is safe because we will never leave this function (alive)
        // The same applies for closure in on_init
        let closure: Box<dyn FnMut(&mut imgui::Ui)> = Box::new(on_draw);
        let closure: Box<dyn FnMut(&mut imgui::Ui) + 'static> =
            unsafe { std::mem::transmute(closure) };

        miniquad::start(conf::Conf::default(), move |ctx| {
            Box::new(Stage::new(ctx, closure, self.on_init, self.on_quit))
        });

        std::process::exit(0)
    }
}

pub use miniquad::KeyCode;

mod shader {
    use miniquad::{ShaderMeta, UniformBlockLayout, UniformDesc, UniformType};

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

    pub fn meta() -> ShaderMeta {
        ShaderMeta {
            images: vec!["Texture".to_string()],
            uniforms: UniformBlockLayout {
                uniforms: vec![UniformDesc::new("Projection", UniformType::Mat4)],
            },
        }
    }

    #[repr(C)]
    #[derive(Debug)]
    pub struct Uniforms {
        pub projection: glam::Mat4,
    }
}
