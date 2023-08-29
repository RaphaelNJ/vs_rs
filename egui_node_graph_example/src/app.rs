use std::fmt;
use std::{ borrow::Cow, collections::HashMap };

use eframe::egui::{ self, DragValue, TextStyle };
use egui_node_graph::*;
use egui_file::FileDialog;
use std::path::PathBuf;

use std::io::{ Read, Write };
use std::fs::{ File, OpenOptions };
use bincode;

use serde::{ Deserialize, Serialize };

use slotmap::SlotMap;

use crate::functions;
use crate::utils;
use crate::variables;
use crate::compiler;
use crate::nodes;
use crate::types;

pub const DISABLE_RECURSIVE_FUNCTIONS: bool = true;



/// The response type is used to encode side-effects produced when drawing a
/// node in the graph. Most side-effects (creating new nodes, deleting existing
/// nodes, handling connections...) are already handled by the library, but this
/// mechanism allows creating additional side effects from user code.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MyResponse {
    AsignFunction(NodeId, Option<functions::FunctionId>),
}

/// The graph 'global' state. This state struct is passed around to the node and
/// parameter drawing callbacks. The contents of this struct are entirely up to
/// the user. For this example, we use it to keep track of the 'active' node.
#[derive(Default, Serialize, Deserialize)]
pub struct MyGraphState {
    pub active_node: Option<NodeId>,
    pub functions: SlotMap<functions::FunctionId, GraphFunction>,
    pub graph_id: functions::FunctionId,
    pub main_graph_id: functions::FunctionId,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum SaveOrLoad {
    Save,
    Load,
}

impl Default for SaveOrLoad {
    fn default() -> Self {
        SaveOrLoad::Load
    }
}


pub type MyGraph = Graph<nodes::MyNodeData, types::MyDataType, types::MyValueType>;
type MyEditorState = GraphEditorState<
    nodes::MyNodeData,
    types::MyDataType,
    types::MyValueType,
    nodes::MyNodeTemplate,
    MyGraphState
>;

#[derive(Default, Serialize, Deserialize)]
pub struct NodeGraphExample {
    // The `GraphEditorState` is the top-level object. You "register" all your
    // custom types by specifying it as its generic parameters.
    pub state: MyEditorState,

    pub user_state: MyGraphState,
}

const PERSISTENCE_KEY: &str = "egui_node_graph";

impl NodeGraphExample {
    /// If the persistence feature is enabled, Called once before the first frame.
    /// Load previous app state (if any).
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let state = cc.storage
            .and_then(|storage| eframe::get_value(storage, PERSISTENCE_KEY))
            .unwrap_or_default();
        Self {
            state,
            user_state: MyGraphState::default(),
        }
    }
}

impl AppState {
    // Save the struct to the specified location using bincode
    pub fn save_to_file(&self, file_path: &PathBuf) -> Result<(), Box<dyn std::error::Error>> {
        let mut file = OpenOptions::new().write(true).create(true).truncate(true).open(file_path)?;

        let encoded = bincode::serialize(self)?;
        file.write_all(&encoded)?;

        Ok(())
    }

    // Load the struct from the specified location using bincode
    pub fn load_from_file(file_path: &PathBuf) -> Result<Self, Box<dyn std::error::Error>> {
        let mut file = File::open(file_path)?;
        let mut buffer = Vec::new();
        file.read_to_end(&mut buffer)?;

        let decoded: Self = bincode::deserialize(&buffer)?;

        Ok(decoded)
    }

    pub fn load(&mut self, file_path: &PathBuf) -> Result<(), Box<dyn std::error::Error>> {
        *self = AppState::load_from_file(file_path)?;
        Ok(())
    }
}

pub struct App {
    pub save_load_actions: Option<PathBuf>,
    pub open_file_dialog: Option<(FileDialog, SaveOrLoad)>,
    pub new_function_window: Option<CreateFunctionDialog>,
    pub app_state: AppState,
}

#[derive(Serialize, Deserialize)]
pub struct AppState {
    pub current_function: functions::FunctionId,
    pub functions: SlotMap<functions::FunctionId, GraphFunction>,
    pub graph: NodeGraphExample,
    pub main_graph_id: functions::FunctionId,
}

pub struct CreateFunctionDialog {
    pub name: String,
    pub input: Vec<FunctionIO>,
    pub output: Vec<FunctionIO>,
}

#[derive(Serialize, Deserialize)]
pub struct GraphFunction {
    pub graph: NodeGraphExample,
    pub name: String,
    pub removable: bool,
    pub modifiable_name: bool,
    pub variables_list: Vec<Variable>,
    pub input: Vec<FunctionIO>,
    pub output: Vec<FunctionIO>,
}

#[derive(Serialize, Deserialize)]
pub struct FunctionIO {
    pub name: String,
    pub value: types::VariableValue,
}

#[derive(Serialize, Deserialize)]
pub struct Variable {
    pub name: String,
    pub value: types::VariableValue,
    pub removable: bool,
}

impl Default for CreateFunctionDialog {
    fn default() -> Self {
        Self { name: "new_function".to_string(), input: vec![], output: vec![] }
    }
}

impl Default for App {
    fn default() -> Self {
        let mut functions = SlotMap::default();
        let current_function = functions.insert(GraphFunction {
            graph: NodeGraphExample::default(),
            name: "Main".to_owned(),
            removable: false,
            modifiable_name: false,
            variables_list: vec![
                Variable {
                    name: "Hello".to_string(),
                    value: types::VariableValue::String("World !".to_string()),
                    removable: true,
                },
                Variable {
                    name: "Hello_World".to_string(),
                    value: types::VariableValue::Boolean(true),
                    removable: true,
                }
            ],
            input: vec![],
            output: vec![],
        });
        let mut graph = NodeGraphExample::default();
        graph.user_state.graph_id = current_function;
        graph.user_state.main_graph_id = current_function;
        Self {
            save_load_actions: None,
            open_file_dialog: None,
            new_function_window: None,
            app_state: AppState {
                main_graph_id: current_function,
                current_function,
                functions,
                graph,
            },
        }
    }
}

impl utils::GetName for GraphFunction {
    fn get_name(&self) -> String {
        self.name.clone()
    }
}

impl utils::GetName for FunctionIO {
    fn get_name(&self) -> String {
        self.name.clone()
    }
}

impl utils::GetName for Variable {
    fn get_name(&self) -> String {
        self.name.clone()
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        self.app_state.graph.user_state.functions = std::mem::replace(
            &mut self.app_state.functions,
            SlotMap::default()
        );
        self.app_state.graph.update(ctx, frame);
        self.app_state.functions = std::mem::replace(
            &mut self.app_state.graph.user_state.functions,
            SlotMap::default()
        );
        egui::TopBottomPanel::top("top").show(ctx, |ui| {
            egui::menu::bar(ui, |ui| {
                egui::widgets::global_dark_light_mode_switch(ui);
                if ui.button("Open").clicked() {
                    let mut dialog = FileDialog::open_file(self.save_load_actions.clone());
                    dialog.open();
                    self.open_file_dialog = Some((dialog, SaveOrLoad::Load));
                }
                if ui.button("Save").clicked() {
                    let mut dialog = FileDialog::save_file(self.save_load_actions.clone());
                    dialog.open();
                    self.open_file_dialog = Some((dialog, SaveOrLoad::Save));
                }
                if ui.button("Compile").clicked() {
                    self.app_state.graph = std::mem::replace(
                        &mut self.app_state.functions[self.app_state.current_function].graph,
                        std::mem::replace(&mut self.app_state.graph, NodeGraphExample::default())
                    );

                    let text = match compiler::compile(&self.app_state, nodes::MyNodeTemplate::Enter) {
                        Ok(value) => format!("The result is: {:?}", value),
                        Err(err) => format!("Execution error: {}", err),
                    };
                    println!("{}", text);
                    ctx.debug_painter().text(
                        egui::pos2(10.0, 35.0),
                        egui::Align2::LEFT_TOP,
                        text,
                        TextStyle::Button.resolve(&ctx.style()),
                        egui::Color32::WHITE
                    );

                    self.app_state.functions[self.app_state.current_function].graph =
                        std::mem::replace(
                            &mut self.app_state.graph,
                            std::mem::replace(
                                &mut self.app_state.functions
                                    [self.app_state.current_function].graph,
                                NodeGraphExample::default()
                            )
                        );
                }
            });
            if let Some(dialog) = &mut self.open_file_dialog {
                if dialog.0.show(ctx).selected() {
                    if let Some(file) = dialog.0.path() {
                        self.save_load_actions = Some(file.to_path_buf());
                        match dialog.1 {
                            SaveOrLoad::Load => {
                                self.app_state.load(&file.to_path_buf());
                                println!("Load : {:?}", file.to_path_buf());
                            }
                            SaveOrLoad::Save => {
                                self.app_state.save_to_file(&file.to_path_buf());
                                println!("Save : {:?}", file.to_path_buf());
                            }
                        }
                    }
                }
            }
            if let Some(create_function) = &mut self.new_function_window {
                if
                    !functions::show_function_window(
                        ctx,
                        create_function,
                        &mut self.app_state.functions,
                        self.app_state.main_graph_id
                    )
                {
                    self.new_function_window = None;
                }
            }
        });

        // Render The functions tab
        functions::render_functions_tab(ctx, self);

        // Render The variables tab
        variables::render_variables_tab(ctx, self);
    }
}

impl NodeGraphExample {
    /// If the persistence function is enabled,
    /// Called by the frame work to save state before shutdown.
    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        eframe::set_value(storage, PERSISTENCE_KEY, &self.state);
    }
    /// Called each time the UI needs repainting, which may be many times per second.
    /// Put your widgets into a `SidePanel`, `TopPanel`, `CentralPanel`, `Window` or `Area`.
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let graph_response = egui::CentralPanel
            ::default()
            .show(ctx, |ui| {
                self.state.draw_graph_editor(
                    ui,
                    nodes::AllMyNodeTemplates,
                    &mut self.user_state,
                    Vec::default()
                )
            }).inner;
        for node_response in graph_response.node_responses {
            // Here, we ignore all other graph events. But you may find
            // some use for them. For example, by playing a sound when a new
            // connection is created
            if let NodeResponse::User(user_event) = node_response {
                match user_event {
                    MyResponse::AsignFunction(node, function) => {
                        self.state.graph.nodes[node].user_data.template =
                            nodes::MyNodeTemplate::Function(function);
                        let _ = self.state.graph.rename_node(
                            node,
                            format!(
                                "Function {}",
                                function.map_or("", |x| { &self.user_state.functions[x].name })
                            )
                        );
                        self.state.graph.remove_all_nodes_connections(node);
                        self.state.graph.nodes[node].inputs.clear();
                        self.state.graph.nodes[node].outputs.clear();
                        let template = self.state.graph.nodes[node].user_data.template;
                        template.build_node(&mut self.state.graph, &mut self.user_state, node);
                    }
                }
            }
            if
                let NodeResponse::ConnectEventEnded {
                    output,
                    input,
                    node_input: _,
                    node_output: _,
                } = node_response
            {
                // Check if the output can be send to differents inputs
                if let Some(out) = self.state.graph.outputs.get(output) {
                    if out.typ == types::MyDataType::Execution {
                        // Remove all the others connections with the same input.
                        self.state.graph.connections.retain(
                            |inp, out| !(inp != input && *out == output)
                        );
                    }
                }
            }
        }
    }
}