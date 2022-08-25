use imgui::*;

fn main() {
    let mut input = String::with_capacity(128);

    imgui_miniquad_render::Window::new("Test").main_loop(|ui| {
        Window::new("window_title")
            .size([300.0, 300.0], Condition::FirstUseEver)
            .build(ui, || {
                ui.text("Hello world!");
                ui.text("This...is...imgui-rs!");
                ui.separator();
                let mouse_pos = ui.io().mouse_pos;
                ui.text(format!(
                    "Mouse Position: ({:.1},{:.1})",
                    mouse_pos[0], mouse_pos[1]
                ));
                ui.input_text("edit", &mut input).build();
            });
    });
}
