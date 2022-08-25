use imgui::*;
use imgui_miniquad_render::platform;

fn main() {
    let mut text = String::with_capacity(10);

    imgui_miniquad_render::Window::new("Test")
        .on_init(|imgui| {
            let style = imgui.style_mut();
            style.window_rounding = 0.;
            style.use_light_colors();
        })
        .main_loop(|ui| {
            let [width, height] = ui.io().display_size;

            Window::new("Modbus Testing Tool")
                .position([0.0, 0.0], Condition::Always)
                .size([width, height], Condition::Always)
                .title_bar(false)
                .resizable(false)
                .menu_bar(true)
                .build(ui, || {
                    if let Some(menu_bar) = ui.begin_menu_bar() {
                        if let Some(menu) = ui.begin_menu("File") {
                            if MenuItem::new("Exit").build(ui) {
                                platform::request_quit();
                            }
                            menu.end();
                        }

                        menu_bar.end();
                    };

                    ui.input_text_multiline("", &mut text, [-1., -1.]).build();
                });
        });
}
