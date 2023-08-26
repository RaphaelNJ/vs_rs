use crate::app;
use crate::utils;

use eframe::egui;
use slotmap;
slotmap::new_key_type! {
    pub struct FunctionId;
}

pub fn show_function_window(
    ctx: &egui::Context,
    create_function: &mut app::CreateFunctionDialog,
    functions: &mut slotmap::SlotMap<FunctionId, app::GraphFunction>,
    main_function: FunctionId,
) -> bool {
    let mut is_new_function_window = true;
    let mut is_new_function_created = false;
    egui::Window
        ::new("Create Function")
        .open(&mut is_new_function_window)
        .collapsible(false)
        .resizable(false)
        .show(ctx, |ui| {
            ui.set_height(480.0);
            ui.set_width(730.0);
            ui.allocate_space(egui::vec2(0.0, 5.0));

            ui.label("Function Name :");
            ui.text_edit_singleline(&mut create_function.name);
            create_function.name = utils::uniquify_name_slot(
                create_function.name.clone(),
                functions
            );

            ui.separator();

            use egui_extras::{ Column, TableBuilder };

            let text_height = egui::TextStyle::Body.resolve(ui.style()).size + 5.0;

            let input_len = create_function.input.len();
            let output_len = create_function.output.len();

            let sup_len = if input_len < output_len { output_len } else { input_len };

            let mut to_remove = (None, 0);

            let table = TableBuilder::new(ui)
                .cell_layout(egui::Layout::left_to_right(egui::Align::Center))
                .column(Column::remainder())
                .column(Column::remainder())
                .min_scrolled_height(0.0);

            table
                .header(20.0, |mut header| {
                    header.col(|ui| {
                        ui.vertical_centered(|ui| {
                            ui.strong("Input");
                        });
                    });
                    header.col(|ui| {
                        ui.vertical_centered(|ui| {
                            ui.strong("Output");
                        });
                    });
                })
                .body(|body| {
                    body.rows(text_height, sup_len + 2, |row_index, mut row| {
                        row.col(|ui| {
                            if input_len > row_index {
                                let rep = show_functionio(
                                    row_index,
                                    &mut create_function.input[row_index],
                                    ui,
                                    "i"
                                );
                                if let Some(function_name) = rep.0 {
                                    create_function.input[row_index].name = utils::uniquify_name(
                                        function_name,
                                        &create_function.input
                                    );
                                }
                                if rep.1 == true {
                                    to_remove = (Some(true), row_index);
                                }
                            } else if input_len + 1 == row_index {
                                ui.vertical_centered(|ui| {
                                    if ui.button("+ Add Input").clicked() {
                                        create_function.input.push(app::FunctionIO {
                                            name: utils::uniquify_name(
                                                "new_input".to_string(),
                                                &create_function.input
                                            ),
                                            value: app::VariableValue::Boolean(true),
                                        });
                                    }
                                });
                            }
                        });
                        row.col(|ui| {
                            if output_len > row_index {
                                let rep = show_functionio(
                                    row_index,
                                    &mut create_function.output[row_index],
                                    ui,
                                    "o"
                                );
                                if let Some(function_name) = rep.0 {
                                    create_function.output[row_index].name = utils::uniquify_name(
                                        function_name,
                                        &create_function.output
                                    );
                                }
                                if rep.1 == true {
                                    to_remove = (Some(false), row_index);
                                }
                            } else if output_len + 1 == row_index {
                                ui.vertical_centered(|ui| {
                                    if ui.button("+ Add Output").clicked() {
                                        create_function.output.push(app::FunctionIO {
                                            name: utils::uniquify_name(
                                                "new_output".to_string(),
                                                &create_function.output
                                            ),
                                            value: app::VariableValue::Boolean(true),
                                        });
                                    }
                                });
                            }
                        });
                    });
                });

            if let Some(side) = to_remove.0 {
                if side {
                    create_function.input.remove(to_remove.1);
                } else {
                    create_function.output.remove(to_remove.1);
                }
            }

            ui.allocate_space(egui::vec2(0.0, 5.0));
            ui.vertical_centered(|ui| {
                if ui.button("Create").clicked() {
                    let new_function = functions.insert(app::GraphFunction {
                        graph: app::NodeGraphExample::default(),
                        name: std::mem::replace(&mut create_function.name, "".to_string()),
                        removable: true,
                        modifiable_name: true,
                        variables_list: vec![],
                        input: std::mem::replace(&mut create_function.input, vec![]),
                        output: std::mem::replace(&mut create_function.output, vec![]),
                    });
                    functions[new_function].graph.user_state.graph_id = new_function;
                    functions[new_function].graph.user_state.main_graph_id = main_function;
                    is_new_function_created = true;
                }
            });
        });
    if !is_new_function_window || is_new_function_created {
        return false;
    } else {
        return true;
    }
}

fn show_functionio(
    row_index: usize,
    function_oi: &mut app::FunctionIO,
    ui: &mut egui::Ui,
    id: &str
) -> (Option<String>, bool) {
    let default_variable_values = [
        (app::VariableValue::String("".to_owned()), "String"),
        (app::VariableValue::Integer(0.0), "Integer"),
        (app::VariableValue::Float(0.0), "Float"),
        (app::VariableValue::Boolean(true), "Boolean"),
    ];

    let mut changed = (None, false);

    let mut function_name = function_oi.name.to_string();

    if ui.add(egui::TextEdit::singleline(&mut function_name).desired_width(133.0)).changed() {
        changed.0 = Some(function_name);
    }

    egui::ComboBox
        ::from_id_source(format!("{}{id}", row_index))
        .selected_text(function_oi.value.to_string())
        .width(74.0)
        .show_ui(ui, |ui| {
            for (value, name) in &default_variable_values {
                ui.selectable_value(&mut function_oi.value, value.clone(), name.to_string());
            }
        });
    match function_oi.value {
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
    }
    if ui.button("x").clicked() {
        changed.1 = true;
    }
    changed
}

pub fn render_functions_tab(ctx: &egui::Context, app: &mut app::App) {
    egui::SidePanel
        ::left("funcs")
        .resizable(false)
        .show(ctx, |ui| {
            ui.set_min_width(190.0);
            ui.set_max_width(190.0);

            egui::TopBottomPanel::top("add_func").show_inside(ui, |ui| {
                ui.vertical_centered(|ui| {
                    if ui.button("+ Add Function").clicked() {
                        app.new_function_window = Some(app::CreateFunctionDialog::default());
                    }
                });
            });

            let mut to_remove = None;
            let mut to_modify = (None, String::new());
            let mut change_current_function = None;

            egui::ScrollArea
                ::vertical()
                .auto_shrink([false; 2])
                .min_scrolled_height(64.0)
                .show(ui, |ui| {
                    ui.vertical(|ui| {
                        for function in app.app_state.functions.iter() {
                            ui.horizontal(|ui| {
                                let mut function_name = function.1.name.to_string();

                                if
                                    ui
                                        .selectable_label(
                                            app.app_state.current_function == function.0,
                                            "↪"
                                        )
                                        .clicked()
                                {
                                    change_current_function = Some(function.0);
                                }
                                if function.1.modifiable_name {
                                    if
                                        ui
                                            .add(
                                                egui::TextEdit
                                                    ::singleline(&mut function_name)
                                                    .desired_width(133.0)
                                            )
                                            .changed()
                                    {
                                        to_modify.1 = function_name;
                                        to_modify.0 = Some(function.0);
                                    }
                                } else {
                                    ui.label(function_name);
                                }
                                if function.1.removable {
                                    if ui.button("x").clicked() {
                                        to_remove = Some(function.0);
                                    }
                                }
                            });
                        }
                    });
                });

            if let Some(index) = to_modify.0 {
                app.app_state.functions[index].name = utils::uniquify_name_slot(
                    to_modify.1,
                    &app.app_state.functions
                );
            }

            if let Some(index) = to_remove {
                app.app_state.functions.remove(index);
                app.app_state.current_function = app.app_state.functions
                    .keys()
                    .into_iter()
                    .last()
                    .expect("at least one function");
            }
            if let Some(index) = change_current_function {
                app.app_state.functions[app.app_state.current_function].graph = std::mem::replace(
                    &mut app.app_state.graph,
                    std::mem::replace(
                        &mut app.app_state.functions[index].graph,
                        app::NodeGraphExample::default()
                    )
                );
                app.app_state.current_function = index;
            }
        });
}
