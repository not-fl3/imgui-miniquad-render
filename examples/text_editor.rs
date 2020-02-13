use imgui::*;
use imgui_miniquad_render::platform;

fn main() {
    let mut text = ImString::with_capacity(10);

    imgui_miniquad_render::Window::new("Test")
        .on_init(|imgui| {
            let style = imgui.style_mut();
            style.window_rounding = 0.;
            style.use_light_colors();
        })
        .main_loop(|ui| {
            let [width, height] = ui.io().display_size;

            Window::new(im_str!("Modbus Testing Tool"))
                .position([0.0, 0.0], Condition::Always)
                .size([width, height], Condition::Always)
                .title_bar(false)
                .resizable(false)
                .menu_bar(true)
                .build(ui, || {
                    if let Some(menu_bar) = ui.begin_menu_bar() {
                        if let Some(menu) = ui.begin_menu(im_str!("File"), true) {
    
                            if MenuItem::new(im_str!("Exit")).build(ui) {
                                platform::request_quit();
                            }
                            menu.end(ui);
                        }

                        menu_bar.end(ui);
                    };


                    ui.input_text_multiline(&im_str!(""), &mut text, [-1., -1.])
                        .resize_buffer(true)
                        .build();
                })
        });
}
