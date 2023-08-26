use eframe::egui;
use crate::utils;
use crate::app;

pub fn render_variables_tab(ctx: &egui::Context, app: &mut app::App) {
    egui::SidePanel
        ::right("vars")
        .resizable(false)
        .show(ctx, |ui| {
            ui.set_min_width(345.0);
            ui.set_max_width(345.0);

            egui::TopBottomPanel::top("add_var").show_inside(ui, |ui| {
                ui.vertical_centered(|ui| {
                    if ui.button("+ Add Variable").clicked() {
                        let new_variable = app::Variable {
                            name: utils::uniquify_name(
                                "new".to_string(),
                                &app.app_state.functions[app.app_state.current_function].variables_list
                            ),
                            value: app::VariableValue::Boolean(true),
                            removable: true,
                        };
                        app.app_state.functions[app.app_state.current_function].variables_list.push(new_variable);
                    }
                });
            });

            let mut to_remove = None;
            let mut name_changed = (None, String::new());

            egui::ScrollArea::vertical()
                .auto_shrink([false; 2])
                .min_scrolled_height(64.0)
                .show(ui, |ui| {
                    ui.vertical(|ui| {
                        for (index, variable) in app.app_state.functions[
                            app.app_state.current_function
                        ].variables_list
                            .iter_mut()
                            .enumerate() {
                            ui.horizontal(|ui| {
                                let mut variable_name = variable.name.to_string();
                                if
                                    ui
                                        .add(
                                            egui::TextEdit::singleline(&mut variable_name).desired_width(
                                                133.0
                                            )
                                        )
                                        .changed()
                                {
                                    name_changed.0 = Some(index);
                                    name_changed.1 = variable_name;
                                }

                                let default_variable_values = [
                                    (app::VariableValue::String("".to_owned()), "String"),
                                    (app::VariableValue::Integer(0.0), "Integer"),
                                    (app::VariableValue::Float(0.0), "Float"),
                                    (app::VariableValue::Boolean(true), "Boolean"),
                                ];

                                egui::ComboBox
                                    ::from_id_source(index)
                                    .selected_text(variable.value.to_string())
                                    .width(74.0)
                                    .show_ui(ui, |ui| {
                                        for (value, name) in &default_variable_values {
                                            ui.selectable_value(
                                                &mut variable.value,
                                                value.clone(),
                                                name.to_string()
                                            );
                                        }
                                    });

                                match variable.value {
                                    app::VariableValue::String(ref mut x) => {
                                        ui.add(egui::TextEdit::singleline(x).desired_width(100.0));
                                    }
                                    app::VariableValue::Integer(ref mut x) => {
                                        ui.add(egui::DragValue::new(x).speed(1.0));
                                        *x = x.round();
                                    }
                                    app::VariableValue::Float(ref mut x) => {
                                        ui.add(egui::DragValue::new(x).speed(0.1));
                                    }
                                    app::VariableValue::Boolean(ref mut x) => {
                                        ui.checkbox(x, "".to_string());
                                    }
                                    app::VariableValue::Execution => {}
                                }

                                if variable.removable {
                                    if ui.button("x").clicked() {
                                        to_remove = Some(index);
                                    }
                                }
                            });
                        }
                    });
                });

            if let Some(index) = to_remove {
                app.app_state.functions[app.app_state.current_function].variables_list.remove(index);
            }
            if let Some(index) = name_changed.0 {
                app.app_state.functions[app.app_state.current_function].variables_list[index].name =
                    utils::uniquify_name(
                        name_changed.1,
                        &app.app_state.functions[app.app_state.current_function].variables_list
                    );
            }
        });
}
