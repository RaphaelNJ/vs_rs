use std::fmt;
use std::{ borrow::Cow, collections::HashMap, hash::Hash };

use eframe::egui::{ self, DragValue, TextStyle, Grid, ScrollArea, TextEdit, Window };
use egui_node_graph::*;
use egui_file::FileDialog;
use std::path::PathBuf;

use std::fs::{ File, OpenOptions };
use std::io::{ Read, Write };
use bincode;

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
    Scalar,
    Vec2,
}

/// In the graph, input parameters can optionally have a constant value. This
/// value can be directly edited in a widget inside the node itself.
///
/// There will usually be a correspondence between DataTypes and ValueTypes. But
/// this library makes no attempt to check this consistency. For instance, it is
/// up to the user code in this example to make sure no parameter is created
/// with a DataType of Scalar and a ValueType of Vec2.
#[derive(Copy, Clone, Debug)]
#[cfg_attr(feature = "persistence", derive(serde::Serialize, serde::Deserialize))]
pub enum MyValueType {
    Vec2 {
        value: egui::Vec2,
    },
    Scalar {
        value: f32,
    },
}

impl Default for MyValueType {
    fn default() -> Self {
        // NOTE: This is just a dummy `Default` implementation. The library
        // requires it to circumvent some internal borrow checker issues.
        Self::Scalar { value: 0.0 }
    }
}

impl MyValueType {
    /// Tries to downcast this value type to a vector
    pub fn try_to_vec2(self) -> anyhow::Result<egui::Vec2> {
        if let MyValueType::Vec2 { value } = self {
            Ok(value)
        } else {
            anyhow::bail!("Invalid cast from {:?} to vec2", self)
        }
    }

    /// Tries to downcast this value type to a scalar
    pub fn try_to_scalar(self) -> anyhow::Result<f32> {
        if let MyValueType::Scalar { value } = self {
            Ok(value)
        } else {
            anyhow::bail!("Invalid cast from {:?} to scalar", self)
        }
    }
}

/// NodeTemplate is a mechanism to define node templates. It's what the graph
/// will display in the "new node" popup. The user code needs to tell the
/// library how to convert a NodeTemplate into a Node.
#[derive(Clone, Copy, Debug)]
#[cfg_attr(feature = "persistence", derive(serde::Serialize, serde::Deserialize))]
pub enum MyNodeTemplate {
    MakeScalar,
    AddScalar,
    SubtractScalar,
    MakeVector,
    AddVector,
    SubtractVector,
    VectorTimesScalar,
}

/// The response type is used to encode side-effects produced when drawing a
/// node in the graph. Most side-effects (creating new nodes, deleting existing
/// nodes, handling connections...) are already handled by the library, but this
/// mechanism allows creating additional side effects from user code.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MyResponse {
    SetActiveNode(NodeId),
    ClearActiveNode,
}

/// The graph 'global' state. This state struct is passed around to the node and
/// parameter drawing callbacks. The contents of this struct are entirely up to
/// the user. For this example, we use it to keep track of the 'active' node.
#[derive(Default)]
#[cfg_attr(feature = "persistence", derive(serde::Serialize, serde::Deserialize))]
pub struct MyGraphState {
    pub active_node: Option<NodeId>,
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
            MyDataType::Scalar => egui::Color32::from_rgb(38, 109, 211),
            MyDataType::Vec2 => egui::Color32::from_rgb(238, 207, 255),
        }
    }

    fn name(&self) -> Cow<'_, str> {
        match self {
            MyDataType::Scalar => Cow::Borrowed("scalar"),
            MyDataType::Vec2 => Cow::Borrowed("2d vector"),
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
            MyNodeTemplate::MakeScalar => "New scalar",
            MyNodeTemplate::AddScalar => "Scalar add",
            MyNodeTemplate::SubtractScalar => "Scalar subtract",
            MyNodeTemplate::MakeVector => "New vector",
            MyNodeTemplate::AddVector => "Vector add",
            MyNodeTemplate::SubtractVector => "Vector subtract",
            MyNodeTemplate::VectorTimesScalar => "Vector times scalar",
        })
    }

    // this is what allows the library to show collapsible lists in the node finder.
    fn node_finder_categories(&self, _user_state: &mut Self::UserState) -> Vec<&'static str> {
        match self {
            | MyNodeTemplate::MakeScalar
            | MyNodeTemplate::AddScalar
            | MyNodeTemplate::SubtractScalar => vec!["Scalar"],
            | MyNodeTemplate::MakeVector
            | MyNodeTemplate::AddVector
            | MyNodeTemplate::SubtractVector => vec!["Vector"],
            MyNodeTemplate::VectorTimesScalar => vec!["Vector", "Scalar"],
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
        _user_state: &mut Self::UserState,
        node_id: NodeId
    ) {
        // The nodes are created empty by default. This function needs to take
        // care of creating the desired inputs and outputs based on the template

        // We define some closures here to avoid boilerplate. Note that this is
        // entirely optional.
        let input_scalar = |graph: &mut MyGraph, name: &str| {
            graph.add_input_param(
                node_id,
                name.to_string(),
                MyDataType::Scalar,
                MyValueType::Scalar { value: 0.0 },
                InputParamKind::ConnectionOrConstant,
                true
            );
        };
        let input_vector = |graph: &mut MyGraph, name: &str| {
            graph.add_input_param(
                node_id,
                name.to_string(),
                MyDataType::Vec2,
                MyValueType::Vec2 {
                    value: egui::vec2(0.0, 0.0),
                },
                InputParamKind::ConnectionOrConstant,
                true
            );
        };

        let output_scalar = |graph: &mut MyGraph, name: &str| {
            graph.add_output_param(node_id, name.to_string(), MyDataType::Scalar);
        };
        let output_vector = |graph: &mut MyGraph, name: &str| {
            graph.add_output_param(node_id, name.to_string(), MyDataType::Vec2);
        };

        match self {
            MyNodeTemplate::AddScalar => {
                // The first input param doesn't use the closure so we can comment
                // it in more detail.
                graph.add_input_param(
                    node_id,
                    // This is the name of the parameter. Can be later used to
                    // retrieve the value. Parameter names should be unique.
                    "A".into(),
                    // The data type for this input. In this case, a scalar
                    MyDataType::Scalar,
                    // The value type for this input. We store zero as default
                    MyValueType::Scalar { value: 10.0 },
                    // The input parameter kind. This allows defining whether a
                    // parameter accepts input connections and/or an inline
                    // widget to set its value.
                    InputParamKind::ConnectionOnly,
                    true
                );
                input_scalar(graph, "B");
                output_scalar(graph, "out");
            }
            MyNodeTemplate::SubtractScalar => {
                input_scalar(graph, "A");
                input_scalar(graph, "B");
                output_scalar(graph, "out");
            }
            MyNodeTemplate::VectorTimesScalar => {
                input_scalar(graph, "scalar");
                input_vector(graph, "vector");
                output_vector(graph, "out");
            }
            MyNodeTemplate::AddVector => {
                input_vector(graph, "v1");
                input_vector(graph, "v2");
                output_vector(graph, "out");
            }
            MyNodeTemplate::SubtractVector => {
                input_vector(graph, "v1");
                input_vector(graph, "v2");
                output_vector(graph, "out");
            }
            MyNodeTemplate::MakeVector => {
                input_scalar(graph, "x");
                input_scalar(graph, "y");
                output_vector(graph, "out");
            }
            MyNodeTemplate::MakeScalar => {
                input_scalar(graph, "value");
                output_scalar(graph, "out");
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
            MyNodeTemplate::MakeScalar,
            MyNodeTemplate::MakeVector,
            MyNodeTemplate::AddScalar,
            MyNodeTemplate::SubtractScalar,
            MyNodeTemplate::AddVector,
            MyNodeTemplate::SubtractVector,
            MyNodeTemplate::VectorTimesScalar
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
            MyValueType::Vec2 { value } => {
                ui.label(param_name);
                ui.horizontal(|ui| {
                    ui.label("x");
                    ui.add(DragValue::new(&mut value.x));
                    ui.label("y");
                    ui.add(DragValue::new(&mut value.y));
                });
            }
            MyValueType::Scalar { value } => {
                ui.horizontal(|ui| {
                    ui.label(param_name);
                    ui.add(DragValue::new(value));
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
        _graph: &Graph<MyNodeData, MyDataType, MyValueType>,
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
    state: MyEditorState,

    user_state: MyGraphState,
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
impl NodeGraphExample {
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
        *self = NodeGraphExample::load_from_file(file_path)?;
        Ok(())
    }
}

pub struct App {
    pub graph: NodeGraphExample,
    pub save_load_actions: Option<PathBuf>,
    pub open_file_dialog: Option<(FileDialog, SaveOrLoad)>,
    pub functions: Vec<GraphFunction>,
    pub current_function: usize,
    pub new_function_window: Option<CreateFunctionDialog>,
}

pub struct CreateFunctionDialog {
    pub name: String,
    pub input: Vec<FunctionIO>,
    pub output: Vec<FunctionIO>,
}

pub struct GraphFunction {
    pub graph: NodeGraphExample,
    pub name: String,
    pub removable: bool,
    pub modifiable_name: bool,
    pub variables_list: Vec<Variable>,
    pub input: Vec<FunctionIO>,
    pub output: Vec<FunctionIO>,
}

pub struct FunctionIO {
    pub name: String,
    pub value: VariableValue,
}

pub struct Variable {
    pub name: String,
    pub value: VariableValue,
    pub removable: bool,
}

#[derive(PartialEq, Clone, Debug)]
pub enum VariableValue {
    String(String),
    Integer(f64),
    Float(f64),
    Boolean(bool),
}

fn show_functionio(
    row_index: usize,
    function_oi: &mut FunctionIO,
    ui: &mut egui::Ui,
    id: &str
) -> (Option<String>, bool) {
    let default_variable_values = [
        (VariableValue::String("".to_owned()), "String"),
        (VariableValue::Integer(0.0), "Integer"),
        (VariableValue::Float(0.0), "Float"),
        (VariableValue::Boolean(true), "Boolean"),
    ];

    let mut changed = (None, false);

    let mut function_name = function_oi.name.to_string();

    if ui.add(TextEdit::singleline(&mut function_name).desired_width(133.0)).changed() {
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
        VariableValue::String(ref mut x) => {
            ui.add(TextEdit::singleline(x).desired_width(100.0));
        }
        VariableValue::Integer(ref mut x) => {
            ui.add(egui::DragValue::new(x).speed(1.0));
            *x = x.round();
        }
        VariableValue::Float(ref mut x) => {
            ui.add(egui::DragValue::new(x).speed(0.1));
        }
        VariableValue::Boolean(ref mut x) => {
            ui.checkbox(x, "".to_string());
        }
    }
    if ui.button("x").clicked() {
        changed.1 = true;
    }
    changed
}

impl fmt::Display for VariableValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            VariableValue::String(_) => write!(f, "String"),
            VariableValue::Integer(_) => write!(f, "Integer"),
            VariableValue::Float(_) => write!(f, "Float"),
            VariableValue::Boolean(_) => write!(f, "Boolean"),
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
        Self {
            graph: NodeGraphExample::default(),
            save_load_actions: None,
            open_file_dialog: None,
            functions: vec![GraphFunction {
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
            }],
            current_function: 0,
            new_function_window: None,
        }
    }
}

trait GetName {
    fn get_name(&self) -> String;
}

fn uniquify_name(input_name: String, vec: &Vec<impl GetName>) -> String {
    let mut times = 0;
    let name = input_name.replace(" ", "_");
    loop {
        let x = 'x: {
            for obj in vec.iter() {
                if times == 0 {
                    if obj.get_name() == name {
                        times += 1;
                        break 'x true;
                    }
                } else {
                    if format!("{}_{}", name, times) == obj.get_name() {
                        times += 1;
                        break 'x true;
                    }
                }
            }
            false
        };
        if !x {
            if times == 0 {
                return name;
            }
            return format!("{}_{}", name, times);
        }
    }
}

impl GetName for GraphFunction {
    fn get_name(&self) -> String {
        self.name.clone()
    }
}

impl GetName for FunctionIO {
    fn get_name(&self) -> String {
        self.name.clone()
    }
}

impl GetName for Variable {
    fn get_name(&self) -> String {
        self.name.clone()
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        self.graph.update(ctx, frame);
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
                                self.graph.load(&file.to_path_buf());
                                println!("Load : {:?}", file.to_path_buf());
                            }
                            SaveOrLoad::Save => {
                                self.graph.save_to_file(&file.to_path_buf());
                                println!("Save : {:?}", file.to_path_buf());
                            }
                        }
                    }
                }
            }
        });

        // Render The functions tab
        egui::SidePanel::left("funcs").show(ctx, |ui| {
            ui.set_min_width(190.0);
            ui.set_max_width(190.0);

            egui::TopBottomPanel::top("add_func").show_inside(ui, |ui| {
                ui.vertical_centered(|ui| {
                    if ui.button("+ Add Function").clicked() {
                        self.new_function_window = Some(CreateFunctionDialog::default());
                    }
                });
            });

            let mut to_remove = None;
            let mut to_modify = (None, String::new());
            let mut change_current_function = None;

            ScrollArea::vertical()
                .auto_shrink([false; 2])
                .min_scrolled_height(64.0)
                .show(ui, |ui| {
                    ui.vertical(|ui| {
                        for (index, function) in self.functions.iter().enumerate() {
                            ui.horizontal(|ui| {
                                let mut function_name = function.name.to_string();

                                if
                                    ui
                                        .selectable_label(self.current_function == index, "↪")
                                        .clicked()
                                {
                                    change_current_function = Some(index);
                                }
                                if function.modifiable_name {
                                    if
                                        ui
                                            .add(
                                                TextEdit::singleline(
                                                    &mut function_name
                                                ).desired_width(133.0)
                                            )
                                            .changed()
                                    {
                                        to_modify.1 = function_name;
                                        to_modify.0 = Some(index);
                                    }
                                } else {
                                    ui.label(function_name);
                                }
                                if function.removable {
                                    if ui.button("x").clicked() {
                                        to_remove = Some(index);
                                    }
                                }
                            });
                        }
                    });
                });

            if let Some(index) = to_modify.0 {
                self.functions[index].name = uniquify_name(to_modify.1, &self.functions);
            }

            if let Some(index) = to_remove {
                self.functions.remove(index);
                self.current_function = 0;
            }
            if let Some(index) = change_current_function {
                self.functions[self.current_function].graph = std::mem::replace(
                    &mut self.graph,
                    std::mem::replace(&mut self.functions[index].graph, NodeGraphExample::default())
                );
                self.current_function = index;
            }
        });

        // Render The variables tab
        egui::SidePanel::right("vars").show(ctx, |ui| {
            ui.set_min_width(345.0);
            ui.set_max_width(345.0);

            egui::TopBottomPanel::top("add_var").show_inside(ui, |ui| {
                ui.vertical_centered(|ui| {
                    if ui.button("+ Add Variable").clicked() {
                        let new_variable = Variable {
                            name: uniquify_name(
                                "new".to_string(),
                                &self.functions[self.current_function].variables_list
                            ),
                            value: VariableValue::Boolean(true),
                            removable: true,
                        };
                        self.functions[self.current_function].variables_list.push(new_variable);
                    }
                });
            });

            let mut to_remove = None;
            let mut name_changed = (None, String::new());

            ScrollArea::vertical()
                .auto_shrink([false; 2])
                .min_scrolled_height(64.0)
                .show(ui, |ui| {
                    ui.vertical(|ui| {
                        for (index, variable) in self.functions[
                            self.current_function
                        ].variables_list
                            .iter_mut()
                            .enumerate() {
                            ui.horizontal(|ui| {
                                let mut variable_name = variable.name.to_string();
                                if
                                    ui
                                        .add(
                                            TextEdit::singleline(&mut variable_name).desired_width(
                                                133.0
                                            )
                                        )
                                        .changed()
                                {
                                    name_changed.0 = Some(index);
                                    name_changed.1 = variable_name;
                                }

                                let default_variable_values = [
                                    (VariableValue::String("".to_owned()), "String"),
                                    (VariableValue::Integer(0.0), "Integer"),
                                    (VariableValue::Float(0.0), "Float"),
                                    (VariableValue::Boolean(true), "Boolean"),
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
                                    VariableValue::String(ref mut x) => {
                                        ui.add(TextEdit::singleline(x).desired_width(100.0));
                                    }
                                    VariableValue::Integer(ref mut x) => {
                                        ui.add(egui::DragValue::new(x).speed(1.0));
                                        *x = x.round();
                                    }
                                    VariableValue::Float(ref mut x) => {
                                        ui.add(egui::DragValue::new(x).speed(0.1));
                                    }
                                    VariableValue::Boolean(ref mut x) => {
                                        ui.checkbox(x, "".to_string());
                                    }
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
                self.functions[self.current_function].variables_list.remove(index);
            }
            if let Some(index) = name_changed.0 {
                self.functions[self.current_function].variables_list[index].name = uniquify_name(
                    name_changed.1,
                    &self.functions[self.current_function].variables_list
                );
            }
        });

        if let Some(create_function) = &mut self.new_function_window {
            let mut is_new_function_window = true;
            let mut is_new_function_created = false;
            egui::Window
                ::new("Create Function")
                .open(&mut is_new_function_window)
                .collapsible(false)
                .resizable(false)
                .show(ctx, |ui| {
                    ui.set_height(460.0);
                    ui.set_width(660.0);
                    ui.allocate_space(egui::vec2(0.0, 5.0));

                    ui.label("Function Name :");
                    ui.text_edit_singleline(&mut create_function.name);
                    create_function.name = uniquify_name(
                        create_function.name.clone(),
                        &self.functions
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
                                            create_function.input[row_index].name = uniquify_name(
                                                function_name,
                                                &create_function.input
                                            );
                                        };
                                        if rep.1 == true {
                                            to_remove = (Some(true), row_index);
                                        }
                                    } else if input_len + 1 == row_index {
                                        ui.vertical_centered(|ui| {
                                            if ui.button("+ Add Input").clicked() {
                                                create_function.input.push(FunctionIO {
                                                    name: uniquify_name(
                                                        "new".to_string(),
                                                        &create_function.input
                                                    ),
                                                    value: VariableValue::Boolean(true),
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
                                            create_function.output[row_index].name = uniquify_name(
                                                function_name,
                                                &create_function.output
                                            );
                                        };
                                        if rep.1 == true {
                                            to_remove = (Some(false), row_index);
                                        }
                                    } else if output_len + 1 == row_index {
                                        ui.vertical_centered(|ui| {
                                            if ui.button("+ Add Output").clicked() {
                                                create_function.output.push(FunctionIO {
                                                    name: uniquify_name(
                                                        "new".to_string(),
                                                        &create_function.output
                                                    ),
                                                    value: VariableValue::Boolean(true),
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
                            self.functions.push(GraphFunction {
                                graph: NodeGraphExample::default(),
                                name: std::mem::replace(&mut create_function.name, "".to_string()),
                                removable: true,
                                modifiable_name: true,
                                variables_list: vec![],
                                input: std::mem::replace(&mut create_function.input, vec![]),
                                output: std::mem::replace(&mut create_function.output, vec![]),
                            });
                            is_new_function_created = true;
                        }
                    });
                });
            if !is_new_function_window || is_new_function_created {
                self.new_function_window = None;
            }
        }
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
                    if out.typ == MyDataType::Scalar {
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

    struct Evaluator<'a> {
        graph: &'a MyGraph,
        outputs_cache: &'a mut OutputsCache,
        node_id: NodeId,
    }
    impl<'a> Evaluator<'a> {
        fn new(graph: &'a MyGraph, outputs_cache: &'a mut OutputsCache, node_id: NodeId) -> Self {
            Self {
                graph,
                outputs_cache,
                node_id,
            }
        }
        fn evaluate_input(&mut self, name: &str) -> anyhow::Result<MyValueType> {
            // Calling `evaluate_input` recursively evaluates other nodes in the
            // graph until the input value for a paramater has been computed.
            evaluate_input(self.graph, self.node_id, name, self.outputs_cache)
        }
        fn populate_output(
            &mut self,
            name: &str,
            value: MyValueType
        ) -> anyhow::Result<MyValueType> {
            // After computing an output, we don't just return it, but we also
            // populate the outputs cache with it. This ensures the evaluation
            // only ever computes an output once.
            //
            // The return value of the function is the "final" output of the
            // node, the thing we want to get from the evaluation. The example
            // would be slightly more contrived when we had multiple output
            // values, as we would need to choose which of the outputs is the
            // one we want to return. Other outputs could be used as
            // intermediate values.
            //
            // Note that this is just one possible semantic interpretation of
            // the graphs, you can come up with your own evaluation semantics!
            populate_output(self.graph, self.outputs_cache, self.node_id, name, value)
        }
        fn input_vector(&mut self, name: &str) -> anyhow::Result<egui::Vec2> {
            self.evaluate_input(name)?.try_to_vec2()
        }
        fn input_scalar(&mut self, name: &str) -> anyhow::Result<f32> {
            self.evaluate_input(name)?.try_to_scalar()
        }
        fn output_vector(&mut self, name: &str, value: egui::Vec2) -> anyhow::Result<MyValueType> {
            self.populate_output(name, MyValueType::Vec2 { value })
        }
        fn output_scalar(&mut self, name: &str, value: f32) -> anyhow::Result<MyValueType> {
            self.populate_output(name, MyValueType::Scalar { value })
        }
    }

    let node = &graph[node_id];
    let mut evaluator = Evaluator::new(graph, outputs_cache, node_id);

    println!("\n\n\n{:?}", graph[node_id]);
    println!("{:?}", graph[node_id].get_input("B"));
    if let Ok(t) = evaluator.evaluate_input("B") {
        println!("{:?}", t);
    } else {
        println!("NO B");
    }
    println!("{:?}", graph.connections);

    match node.user_data.template {
        MyNodeTemplate::AddScalar => {
            let a = evaluator.input_scalar("A")?;
            let b = evaluator.input_scalar("B")?;
            evaluator.output_scalar("out", a + b)
        }
        MyNodeTemplate::SubtractScalar => {
            let a = evaluator.input_scalar("A")?;
            let b = evaluator.input_scalar("B")?;
            evaluator.output_scalar("out", a - b)
        }
        MyNodeTemplate::VectorTimesScalar => {
            let scalar = evaluator.input_scalar("scalar")?;
            let vector = evaluator.input_vector("vector")?;
            evaluator.output_vector("out", vector * scalar)
        }
        MyNodeTemplate::AddVector => {
            let v1 = evaluator.input_vector("v1")?;
            let v2 = evaluator.input_vector("v2")?;
            evaluator.output_vector("out", v1 + v2)
        }
        MyNodeTemplate::SubtractVector => {
            let v1 = evaluator.input_vector("v1")?;
            let v2 = evaluator.input_vector("v2")?;
            evaluator.output_vector("out", v1 - v2)
        }
        MyNodeTemplate::MakeVector => {
            let x = evaluator.input_scalar("x")?;
            let y = evaluator.input_scalar("y")?;
            evaluator.output_vector("out", egui::vec2(x, y))
        }
        MyNodeTemplate::MakeScalar => {
            let value = evaluator.input_scalar("value")?;
            evaluator.output_scalar("out", value)
        }
    }
}

fn populate_output(
    graph: &MyGraph,
    outputs_cache: &mut OutputsCache,
    node_id: NodeId,
    param_name: &str,
    value: MyValueType
) -> anyhow::Result<MyValueType> {
    let output_id = graph[node_id].get_output(param_name)?;
    outputs_cache.insert(output_id, value);
    Ok(value)
}

// Evaluates the input value of
fn evaluate_input(
    graph: &MyGraph,
    node_id: NodeId,
    param_name: &str,
    outputs_cache: &mut OutputsCache
) -> anyhow::Result<MyValueType> {
    let input_id = graph[node_id].get_input(param_name)?;

    // The output of another node is connected.
    if let Some(other_output_id) = graph.connection(input_id) {
        // The value was already computed due to the evaluation of some other
        // node. We simply return value from the cache.
        if let Some(other_value) = outputs_cache.get(&other_output_id) {
            Ok(*other_value)
        } else {
            // This is the first time encountering this node, so we need to
            // recursively evaluate it.
            // Calling this will populate the cache
            evaluate_node(graph, graph[other_output_id].node, outputs_cache)?;

            // Now that we know the value is cached, return it
            Ok(*outputs_cache.get(&other_output_id).expect("Cache should be populated"))
        }
    } else {
        // No existing connection, take the inline value instead.
        Ok(graph[input_id].value)
    }
}
