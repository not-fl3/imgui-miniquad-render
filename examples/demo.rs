use imgui::*;

fn main() {
    imgui_miniquad_render::Window::new("Test").main_loop(|ui| {
        let mut opened = true;
        ui.show_demo_window(&mut opened);
    });
}
