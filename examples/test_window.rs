use imgui::*;

fn main() {
    let mut input = ImString::with_capacity(128);

    imgui_miniquad_render::Window::new("Test").main_loop(|ui| {
        Window::new(im_str!("window_title"))
            .size([300.0, 300.0], Condition::FirstUseEver)
            .build(ui, || {
                ui.text(im_str!("Hello world!"));
                ui.text(im_str!("This...is...imgui-rs!"));
                ui.separator();
                let mouse_pos = ui.io().mouse_pos;
                ui.text(format!(
                    "Mouse Position: ({:.1},{:.1})",
                    mouse_pos[0], mouse_pos[1]
                ));
                ui.input_text(im_str!("edit"), &mut input).build();
            });
    });
}
