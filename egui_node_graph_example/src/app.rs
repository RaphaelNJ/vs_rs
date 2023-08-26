use std::fmt;
use std::{ borrow::Cow, collections::HashMap };

use eframe::egui::{ self, DragValue, TextStyle };
use egui_node_graph::*;
use egui_file::FileDialog;
use std::path::PathBuf;

#[cfg(feature = "persistence")]
use std::io::{ Read, Write };
#[cfg(feature = "persistence")]
use std::fs::{ File, OpenOptions };
#[cfg(feature = "persistence")]
use bincode;
use slotmap::SlotMap;

use crate::functions;
use crate::utils;
use crate::variables;

const DISABLE_RECURSIVE_FUNCTIONS: bool = true;

#[cfg(feature = "persistence")]
use serde::{ Deserialize, Serialize };

// ========= First, define your user data types =============

/// The NodeData holds a custom data struct inside each node. It's useful to
/// store additional information that doesn't live in parameters. For this
/// example, the node data stores the template (i.e. the "type") of the node.
#[derive(Debug)]
#[cfg_attr(feature = "persistence", derive(serde::Serialize, serde::Deserialize))]
pub struct MyNodeData {
    template: MyNodeTemplate,
}

/// `DataType`s are what defines the possible range of connections when
/// attaching two ports together. The graph UI will make sure to not allow
/// attaching incompatible datatypes.
#[derive(PartialEq, Eq, Debug)]
#[cfg_attr(feature = "persistence", derive(serde::Serialize, serde::Deserialize))]
pub enum MyDataType {
    String,
    Integer,
    Float,
    Boolean,
    Execution,
}

/// In the graph, input parameters can optionally have a constant value. This
/// value can be directly edited in a widget inside the node itself.
///
/// There will usually be a correspondence between DataTypes and ValueTypes. But
/// this library makes no attempt to check this consistency. For instance, it is
/// up to the user code in this example to make sure no parameter is created
/// with a DataType of Scalar and a ValueType of Vec2.
#[derive(Clone, Debug)]
#[cfg_attr(feature = "persistence", derive(serde::Serialize, serde::Deserialize))]
pub enum MyValueType {
    String {
        value: String,
    },
    Integer {
        value: i32,
    },
    Float {
        value: f64,
    },
    Boolean {
        value: bool,
    },
    Execution,
}

impl Default for MyValueType {
    fn default() -> Self {
        // NOTE: This is just a dummy `Default` implementation. The library
        // requires it to circumvent some internal borrow checker issues.
        Self::Boolean { value: false }
    }
}

impl MyValueType {
    pub fn try_to_string(self) -> anyhow::Result<String> {
        if let MyValueType::String { value } = self {
            Ok(value)
        } else {
            anyhow::bail!("Invalid cast from {:?} to bool", self)
        }
    }
    pub fn try_to_integer(self) -> anyhow::Result<i32> {
        if let MyValueType::Integer { value } = self {
            Ok(value)
        } else {
            anyhow::bail!("Invalid cast from {:?} to bool", self)
        }
    }
    pub fn try_to_float(self) -> anyhow::Result<f64> {
        if let MyValueType::Float { value } = self {
            Ok(value)
        } else {
            anyhow::bail!("Invalid cast from {:?} to bool", self)
        }
    }
    pub fn try_to_bool(self) -> anyhow::Result<bool> {
        if let MyValueType::Boolean { value } = self {
            Ok(value)
        } else {
            anyhow::bail!("Invalid cast from {:?} to bool", self)
        }
    }
}

/// NodeTemplate is a mechanism to define node templates. It's what the graph
/// will display in the "new node" popup. The user code needs to tell the
/// library how to convert a NodeTemplate into a Node.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "persistence", derive(serde::Serialize, serde::Deserialize))]
pub enum MyNodeTemplate {
    Enter,
    Print,
    Ask,
    Function(Option<functions::FunctionId>),
}

/// The response type is used to encode side-effects produced when drawing a
/// node in the graph. Most side-effects (creating new nodes, deleting existing
/// nodes, handling connections...) are already handled by the library, but this
/// mechanism allows creating additional side effects from user code.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MyResponse {
    SetActiveNode(NodeId),
    ClearActiveNode,
    AsignFunction(NodeId, Option<functions::FunctionId>),
}

/// The graph 'global' state. This state struct is passed around to the node and
/// parameter drawing callbacks. The contents of this struct are entirely up to
/// the user. For this example, we use it to keep track of the 'active' node.
#[derive(Default)]
#[cfg_attr(feature = "persistence", derive(serde::Serialize, serde::Deserialize))]
pub struct MyGraphState {
    pub active_node: Option<NodeId>,
    pub functions: SlotMap<functions::FunctionId, GraphFunction>,
    pub graph_id: functions::FunctionId,
    pub main_graph_id: functions::FunctionId,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "persistence", derive(Serialize, Deserialize))]
pub enum SaveOrLoad {
    Save,
    Load,
}

impl Default for SaveOrLoad {
    fn default() -> Self {
        SaveOrLoad::Load
    }
}

// =========== Then, you need to implement some traits ============

// A trait for the data types, to tell the library how to display them
impl DataTypeTrait<MyGraphState> for MyDataType {
    fn data_type_color(&self, _user_state: &mut MyGraphState) -> egui::Color32 {
        match self {
            MyDataType::String => egui::Color32::from_rgb(38, 109, 211),
            MyDataType::Integer => egui::Color32::from_rgb(238, 207, 255),
            MyDataType::Float => egui::Color32::from_rgb(38, 211, 109),
            MyDataType::Boolean => egui::Color32::from_rgb(211, 109, 38),
            MyDataType::Execution => egui::Color32::from_rgb(255, 255, 255),
        }
    }

    fn name(&self) -> Cow<'_, str> {
        match self {
            MyDataType::String => Cow::Borrowed("String"),
            MyDataType::Integer => Cow::Borrowed("Integer"),
            MyDataType::Float => Cow::Borrowed("Float"),
            MyDataType::Boolean => Cow::Borrowed("Boolean"),
            MyDataType::Execution => Cow::Borrowed("Execution"),
        }
    }
}

// A trait for the node kinds, which tells the library how to build new nodes
// from the templates in the node finder
impl NodeTemplateTrait for MyNodeTemplate {
    type NodeData = MyNodeData;
    type DataType = MyDataType;
    type ValueType = MyValueType;
    type UserState = MyGraphState;
    type CategoryType = &'static str;

    fn node_finder_label(&self, _user_state: &mut Self::UserState) -> Cow<'_, str> {
        Cow::Borrowed(match self {
            MyNodeTemplate::Enter => "Enter Execution",
            MyNodeTemplate::Print => "Print",
            MyNodeTemplate::Ask => "Ask",
            MyNodeTemplate::Function(_) => "Function",
        })
    }

    // this is what allows the library to show collapsible lists in the node finder.
    fn node_finder_categories(&self, _user_state: &mut Self::UserState) -> Vec<&'static str> {
        match self {
            MyNodeTemplate::Print | MyNodeTemplate::Ask => vec!["I/O"],
            MyNodeTemplate::Enter | MyNodeTemplate::Function(_) => vec!["Special"],
        }
    }

    fn node_graph_label(&self, user_state: &mut Self::UserState) -> String {
        // It's okay to delegate this to node_finder_label if you don't want to
        // show different names in the node finder and the node itself.
        self.node_finder_label(user_state).into()
    }

    fn user_data(&self, _user_state: &mut Self::UserState) -> Self::NodeData {
        MyNodeData { template: *self }
    }

    fn build_node(
        &self,
        graph: &mut Graph<Self::NodeData, Self::DataType, Self::ValueType>,
        user_state: &mut Self::UserState,
        node_id: NodeId
    ) {
        // The nodes are created empty by default. This function needs to take
        // care of creating the desired inputs and outputs based on the template

        // We define some closures here to avoid boilerplate. Note that this is
        // entirely optional.
        let classic_input = |graph: &mut MyGraph, name: &str, typ: MyDataType, value: MyValueType| {
            graph.add_input_param(
                node_id,
                name.to_string(),
                typ,
                value,
                InputParamKind::ConnectionOrConstant,
                true
            );
        };
        let classic_output = |graph: &mut MyGraph, name: &str, typ: MyDataType| {
            graph.add_output_param(node_id, name.to_string(), typ);
        };

        let exe_input = |graph: &mut MyGraph, name: &str| {
            graph.add_input_param(
                node_id,
                name.to_string(),
                MyDataType::Execution,
                MyValueType::Execution,
                InputParamKind::ConnectionOnly,
                true
            );
        };
        let exe_output = |graph: &mut MyGraph, name: &str| {
            graph.add_output_param(node_id, name.to_string(), MyDataType::Execution);
        };

        match self {
            MyNodeTemplate::Enter => {
                exe_output(graph, "Enter");
            }

            MyNodeTemplate::Ask => {
                exe_input(graph, "");
                exe_output(graph, "");
                classic_input(graph, "What ?", MyDataType::String, MyValueType::String {
                    value: "".to_string(),
                });
                classic_output(graph, "Answer", MyDataType::String);
            }
            MyNodeTemplate::Print => {
                exe_input(graph, "");
                exe_output(graph, "");
                classic_input(graph, "What ?", MyDataType::String, MyValueType::String {
                    value: "".to_string(),
                });
            }
            MyNodeTemplate::Function(x) => {
                if let Some(function_index) = x {
                    for input in user_state.functions[*function_index].input.iter() {
                        match &input.value {
                            VariableValue::Boolean(x) => {
                                classic_input(
                                    graph,
                                    &input.name,
                                    MyDataType::Boolean,
                                    MyValueType::Boolean { value: x.to_owned() }
                                );
                            }
                            VariableValue::String(x) => {
                                classic_input(
                                    graph,
                                    &input.name,
                                    MyDataType::String,
                                    MyValueType::String { value: x.to_owned() }
                                );
                            }
                            VariableValue::Integer(x) => {
                                classic_input(
                                    graph,
                                    &input.name,
                                    MyDataType::Integer,
                                    MyValueType::Integer { value: x.to_owned() as i32 }
                                );
                            }
                            VariableValue::Float(x) => {
                                classic_input(
                                    graph,
                                    &input.name,
                                    MyDataType::Float,
                                    MyValueType::Float { value: x.to_owned() }
                                );
                            }
                            VariableValue::Execution => {
                                exe_input(graph, &input.name);
                            }
                        }
                    }
                    for output in user_state.functions[*function_index].output.iter() {
                        match &output.value {
                            VariableValue::Boolean(_) => {
                                classic_output(
                                    graph,
                                    &output.name,
                                    MyDataType::Boolean);
                            }
                            VariableValue::String(_) => {
                                classic_output(
                                    graph,
                                    &output.name,
                                    MyDataType::String);
                            }
                            VariableValue::Integer(_) => {
                                classic_output(
                                    graph,
                                    &output.name,
                                    MyDataType::Integer);
                            }
                            VariableValue::Float(_) => {
                                classic_output(
                                    graph,
                                    &output.name,
                                    MyDataType::Float);
                            }
                            VariableValue::Execution => {
                                exe_output(graph, &output.name);
                            }
                        }
                    }
                }
            }
        }
    }
}

pub struct AllMyNodeTemplates;
impl NodeTemplateIter for AllMyNodeTemplates {
    type Item = MyNodeTemplate;

    fn all_kinds(&self) -> Vec<Self::Item> {
        // This function must return a list of node kinds, which the node finder
        // will use to display it to the user. Crates like strum can reduce the
        // boilerplate in enumerating all variants of an enum.
        vec![
            MyNodeTemplate::Function(None),
            MyNodeTemplate::Enter,
            MyNodeTemplate::Ask,
            MyNodeTemplate::Print
        ]
    }
}

impl WidgetValueTrait for MyValueType {
    type Response = MyResponse;
    type UserState = MyGraphState;
    type NodeData = MyNodeData;
    fn value_widget(
        &mut self,
        param_name: &str,
        _node_id: NodeId,
        ui: &mut egui::Ui,
        _user_state: &mut MyGraphState,
        _node_data: &MyNodeData,
        _kind: InputParamKind
    ) -> Vec<MyResponse> {
        // This trait is used to tell the library which UI to display for the
        // inline parameter widgets.

        let should_draw = match _kind {
            InputParamKind::ConnectionOnly => false,
            InputParamKind::ConnectionOrConstant => true,
            InputParamKind::ConstantOnly => true,
        };

        if !should_draw {
            ui.label(param_name);
            return Vec::new();
        }

        match self {
            MyValueType::Integer { value } => {
                ui.horizontal(|ui| {
                    ui.label(param_name);
                    ui.add(DragValue::new(value));
                });
            }
            MyValueType::Float { value } => {
                ui.horizontal(|ui| {
                    ui.label(param_name);
                    ui.add(DragValue::new(value));
                });
            }
            MyValueType::String { value } => {
                ui.horizontal(|ui| {
                    ui.label(param_name);
                    ui.text_edit_singleline(value);
                });
            }
            MyValueType::Boolean { value } => {
                ui.horizontal(|ui| {
                    ui.label(param_name);
                    ui.checkbox(value, "")
                });
            }
            MyValueType::Execution => {
                ui.horizontal(|ui| {
                    ui.label(param_name);
                });
            }
        }
        // This allows you to return your responses from the inline widgets.
        Vec::new()
    }
}

impl UserResponseTrait for MyResponse {}
impl NodeDataTrait for MyNodeData {
    type Response = MyResponse;
    type UserState = MyGraphState;
    type DataType = MyDataType;
    type ValueType = MyValueType;

    // This method will be called when drawing each node. This allows adding
    // extra ui elements inside the nodes. In this case, we create an "active"
    // button which introduces the concept of having an active node in the
    // graph. This is done entirely from user code with no modifications to the
    // node graph library.
    fn bottom_ui(
        &self,
        ui: &mut egui::Ui,
        node_id: NodeId,
        graph: &Graph<MyNodeData, MyDataType, MyValueType>,
        user_state: &mut Self::UserState
    ) -> Vec<NodeResponse<MyResponse, MyNodeData>>
        where MyResponse: UserResponseTrait
    {
        // This logic is entirely up to the user. In this case, we check if the
        // current node we're drawing is the active one, by comparing against
        // the value stored in the global user state, and draw different button
        // UIs based on that.

        let mut responses = vec![];
        let is_active = user_state.active_node.map(|id| id == node_id).unwrap_or(false);

        // Pressing the button will emit a custom user response to either set,
        // or clear the active node. These responses do nothing by themselves,
        // the library only makes the responses available to you after the graph
        // has been drawn. See below at the update method for an example.
        if !is_active {
            if ui.button("👁 Set active").clicked() {
                responses.push(NodeResponse::User(MyResponse::SetActiveNode(node_id)));
            }
        } else {
            let button = egui::Button
                ::new(egui::RichText::new("👁 Active").color(egui::Color32::BLACK))
                .fill(egui::Color32::GOLD);
            if ui.add(button).clicked() {
                responses.push(NodeResponse::User(MyResponse::ClearActiveNode));
            }
        }

        if let MyNodeTemplate::Function(mut current_value) = graph[node_id].user_data.template {
            let value = current_value.clone();
            if let Some(x) = current_value {
                if !user_state.functions.contains_key(x) {
                    current_value = None;
                }
            }

            egui::ComboBox
                ::from_id_source(node_id)
                .selected_text(
                    current_value.map_or("Choose a Function", |x| {
                        user_state.functions.get(x).map_or("Choose a Function", |y| { &y.name })
                    })
                )
                .width(74.0)
                .show_ui(ui, |ui| {
                    for x in user_state.functions.iter() {
                        if !(DISABLE_RECURSIVE_FUNCTIONS && user_state.graph_id == x.0) {
                            if user_state.main_graph_id != x.0 {
                                ui.selectable_value(&mut current_value, Some(x.0), &x.1.name);
                            }
                        }
                    }
                });

            if value != current_value {
                responses.push(
                    NodeResponse::User(MyResponse::AsignFunction(node_id, current_value))
                );
            }
        }

        responses
    }
}

type MyGraph = Graph<MyNodeData, MyDataType, MyValueType>;
type MyEditorState = GraphEditorState<
    MyNodeData,
    MyDataType,
    MyValueType,
    MyNodeTemplate,
    MyGraphState
>;

#[derive(Default)]
#[cfg_attr(feature = "persistence", derive(Serialize, Deserialize))]
pub struct NodeGraphExample {
    // The `GraphEditorState` is the top-level object. You "register" all your
    // custom types by specifying it as its generic parameters.
    pub state: MyEditorState,

    pub user_state: MyGraphState,
}

#[cfg(feature = "persistence")]
const PERSISTENCE_KEY: &str = "egui_node_graph";

#[cfg(feature = "persistence")]
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

#[cfg(feature = "persistence")]
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

#[cfg_attr(feature = "persistence", derive(Serialize, Deserialize))]
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

#[cfg_attr(feature = "persistence", derive(Serialize, Deserialize))]
pub struct GraphFunction {
    pub graph: NodeGraphExample,
    pub name: String,
    pub removable: bool,
    pub modifiable_name: bool,
    pub variables_list: Vec<Variable>,
    pub input: Vec<FunctionIO>,
    pub output: Vec<FunctionIO>,
}

#[cfg_attr(feature = "persistence", derive(Serialize, Deserialize))]
pub struct FunctionIO {
    pub name: String,
    pub value: VariableValue,
}

#[cfg_attr(feature = "persistence", derive(Serialize, Deserialize))]
pub struct Variable {
    pub name: String,
    pub value: VariableValue,
    pub removable: bool,
}

#[derive(PartialEq, Clone, Debug)]
#[cfg_attr(feature = "persistence", derive(Serialize, Deserialize))]
pub enum VariableValue {
    String(String),
    Integer(f64),
    Float(f64),
    Boolean(bool),
    Execution,
}

impl fmt::Display for VariableValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            VariableValue::String(_) => write!(f, "String"),
            VariableValue::Integer(_) => write!(f, "Integer"),
            VariableValue::Float(_) => write!(f, "Float"),
            VariableValue::Boolean(_) => write!(f, "Boolean"),
            VariableValue::Execution => write!(f, "Execution"),
        }
    }
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
                    value: VariableValue::String("World !".to_string()),
                    removable: true,
                },
                Variable {
                    name: "Hello_World".to_string(),
                    value: VariableValue::Boolean(true),
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
                    // TODO
                }
            });
            if let Some(dialog) = &mut self.open_file_dialog {
                if dialog.0.show(ctx).selected() {
                    #[cfg(feature = "persistence")]
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
    #[cfg(feature = "persistence")]
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
                    AllMyNodeTemplates,
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
                    MyResponse::SetActiveNode(node) => {
                        self.user_state.active_node = Some(node);
                    }
                    MyResponse::ClearActiveNode => {
                        self.user_state.active_node = None;
                    }
                    MyResponse::AsignFunction(node, function) => {
                        self.state.graph.nodes[node].user_data.template =
                            MyNodeTemplate::Function(function);
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
                    if out.typ == MyDataType::Execution {
                        // Remove all the others connections with the same input.
                        self.state.graph.connections.retain(
                            |inp, out| !(inp != input && *out == output)
                        );
                    }
                }
            }
        }

        if let Some(node) = self.user_state.active_node {
            if self.state.graph.nodes.contains_key(node) {
                let text = match evaluate_node(&self.state.graph, node, &mut HashMap::new()) {
                    Ok(value) => format!("The result is: {:?}", value),
                    Err(err) => format!("Execution error: {}", err),
                };
                ctx.debug_painter().text(
                    egui::pos2(10.0, 35.0),
                    egui::Align2::LEFT_TOP,
                    text,
                    TextStyle::Button.resolve(&ctx.style()),
                    egui::Color32::WHITE
                );
            } else {
                self.user_state.active_node = None;
            }
        }
    }
}

type OutputsCache = HashMap<OutputId, MyValueType>;

/// Recursively evaluates all dependencies of this node, then evaluates the node itself.
pub fn evaluate_node(
    graph: &MyGraph,
    node_id: NodeId,
    outputs_cache: &mut OutputsCache
) -> anyhow::Result<MyValueType> {
    // To solve a similar problem as creating node types above, we define an
    // Evaluator as a convenience. It may be overkill for this small example,
    // but something like this makes the code much more readable when the
    // number of nodes starts growing.

    return Ok(MyValueType::Boolean { value: true });

    // struct Evaluator<'a> {
    //     graph: &'a MyGraph,
    //     outputs_cache: &'a mut OutputsCache,
    //     node_id: NodeId,
    // }
    // impl<'a> Evaluator<'a> {
    //     fn new(graph: &'a MyGraph, outputs_cache: &'a mut OutputsCache, node_id: NodeId) -> Self {
    //         Self {
    //             graph,
    //             outputs_cache,
    //             node_id,
    //         }
    //     }
    //     fn evaluate_input(&mut self, name: &str) -> anyhow::Result<MyValueType> {
    //         // Calling `evaluate_input` recursively evaluates other nodes in the
    //         // graph until the input value for a paramater has been computed.
    //         evaluate_input(self.graph, self.node_id, name, self.outputs_cache)
    //     }
    //     fn populate_output(
    //         &mut self,
    //         name: &str,
    //         value: MyValueType
    //     ) -> anyhow::Result<MyValueType> {
    //         // After computing an output, we don't just return it, but we also
    //         // populate the outputs cache with it. This ensures the evaluation
    //         // only ever computes an output once.
    //         //
    //         // The return value of the function is the "final" output of the
    //         // node, the thing we want to get from the evaluation. The example
    //         // would be slightly more contrived when we had multiple output
    //         // values, as we would need to choose which of the outputs is the
    //         // one we want to return. Other outputs could be used as
    //         // intermediate values.
    //         //
    //         // Note that this is just one possible semantic interpretation of
    //         // the graphs, you can come up with your own evaluation semantics!
    //         populate_output(self.graph, self.outputs_cache, self.node_id, name, value)
    //     }
    //     fn input_vector(&mut self, name: &str) -> anyhow::Result<egui::Vec2> {
    //         self.evaluate_input(name)?.try_to_vec2()
    //     }
    //     fn input_scalar(&mut self, name: &str) -> anyhow::Result<f32> {
    //         self.evaluate_input(name)?.try_to_scalar()
    //     }
    //     fn output_vector(&mut self, name: &str, value: egui::Vec2) -> anyhow::Result<MyValueType> {
    //         self.populate_output(name, MyValueType::Vec2 { value })
    //     }
    //     fn output_scalar(&mut self, name: &str, value: f32) -> anyhow::Result<MyValueType> {
    //         self.populate_output(name, MyValueType::Scalar { value })
    //     }
    // }

    // let node = &graph[node_id];
    // let mut evaluator = Evaluator::new(graph, outputs_cache, node_id);

    // println!("\n\n\n{:?}", graph[node_id]);
    // println!("{:?}", graph[node_id].get_input("B"));
    // if let Ok(t) = evaluator.evaluate_input("B") {
    //     println!("{:?}", t);
    // } else {
    //     println!("NO B");
    // }
    // println!("{:?}", graph.connections);

    // match node.user_data.template {
    //     MyNodeTemplate::AddScalar => {
    //         let a = evaluator.input_scalar("A")?;
    //         let b = evaluator.input_scalar("B")?;
    //         evaluator.output_scalar("out", a + b)
    //     }
    //     MyNodeTemplate::SubtractScalar => {
    //         let a = evaluator.input_scalar("A")?;
    //         let b = evaluator.input_scalar("B")?;
    //         evaluator.output_scalar("out", a - b)
    //     }
    //     MyNodeTemplate::VectorTimesScalar => {
    //         let scalar = evaluator.input_scalar("scalar")?;
    //         let vector = evaluator.input_vector("vector")?;
    //         evaluator.output_vector("out", vector * scalar)
    //     }
    //     MyNodeTemplate::AddVector => {
    //         let v1 = evaluator.input_vector("v1")?;
    //         let v2 = evaluator.input_vector("v2")?;
    //         evaluator.output_vector("out", v1 + v2)
    //     }
    //     MyNodeTemplate::SubtractVector => {
    //         let v1 = evaluator.input_vector("v1")?;
    //         let v2 = evaluator.input_vector("v2")?;
    //         evaluator.output_vector("out", v1 - v2)
    //     }
    //     MyNodeTemplate::MakeVector => {
    //         let x = evaluator.input_scalar("x")?;
    //         let y = evaluator.input_scalar("y")?;
    //         evaluator.output_vector("out", egui::vec2(x, y))
    //     }
    //     MyNodeTemplate::MakeScalar => {
    //         let value = evaluator.input_scalar("value")?;
    //         evaluator.output_scalar("out", value)
    //     }
    //     MyNodeTemplate::Function(_) => {
    //         let value = 0.0;
    //         evaluator.output_scalar("out", value)
    //     }
    // }
}

// fn populate_output(
//     graph: &MyGraph,
//     outputs_cache: &mut OutputsCache,
//     node_id: NodeId,
//     param_name: &str,
//     value: MyValueType
// ) -> anyhow::Result<MyValueType> {
//     let output_id = graph[node_id].get_output(param_name)?;
//     outputs_cache.insert(output_id, value);
//     Ok(value)
// }

// // Evaluates the input value of
// fn evaluate_input(
//     graph: &MyGraph,
//     node_id: NodeId,
//     param_name: &str,
//     outputs_cache: &mut OutputsCache
// ) -> anyhow::Result<MyValueType> {
//     let input_id = graph[node_id].get_input(param_name)?;

//     // The output of another node is connected.
//     if let Some(other_output_id) = graph.connection(input_id) {
//         // The value was already computed due to the evaluation of some other
//         // node. We simply return value from the cache.
//         if let Some(other_value) = outputs_cache.get(&other_output_id) {
//             Ok(*other_value)
//         } else {
//             // This is the first time encountering this node, so we need to
//             // recursively evaluate it.
//             // Calling this will populate the cache
//             evaluate_node(graph, graph[other_output_id].node, outputs_cache)?;

//             // Now that we know the value is cached, return it
//             Ok(*outputs_cache.get(&other_output_id).expect("Cache should be populated"))
//         }
//     } else {
//         // No existing connection, take the inline value instead.
//         Ok(graph[input_id].value)
//     }
// }
