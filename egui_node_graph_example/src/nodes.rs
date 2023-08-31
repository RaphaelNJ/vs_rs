use std::borrow::Cow;
use std::collections::HashMap;

use eframe::egui;
use egui_node_graph::*;

use std::io::{ Read, Write };
use std::fs::{ File, OpenOptions };
use bincode;

use crate::functions;
use crate::types;
use crate::app::{ self, MyGraph };
/// The NodeData holds a custom data struct inside each node. It's useful to
/// store additional information that doesn't live in parameters. For this
/// example, the node data stores the template (i.e. the "type") of the node.
#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct MyNodeData {
    pub template: MyNodeTemplate,
}

/// NodeTemplate is a mechanism to define node templates. It's what the graph
/// will display in the "new node" popup. The user code needs to tell the
/// library how to convert a NodeTemplate into a Node.
use strum::IntoEnumIterator; // 0.17.1
use strum_macros::EnumIter; // 0.17.1

#[derive(Clone, Copy, Debug, PartialEq, Eq, EnumIter, serde::Serialize, serde::Deserialize)]
pub enum MyNodeTemplate {
    Enter,
    Print,
    Ask,
    If,

    CategoryAdd,
    AddNumber,
    AddString,
    
    Function(Option<functions::FunctionId>),
}

pub struct NodeParams {
    node_type: &'static NodeType,
    label: &'static str,
    shape_shift_category: Option<ShapeShiftCategory>,
}

#[derive(PartialEq)]
pub enum ShapeShiftCategory {
    Add,
}

pub enum NodeType {
    ExecutedAndExecute(&'static str, &'static str),
    Executed(&'static str),
    Execute(&'static str),
    Data,
}

impl MyNodeTemplate {
    fn get_node_params(&self) -> &'static NodeParams {
        match self {
            MyNodeTemplate::Enter =>
                &(NodeParams {
                    shape_shift_category: None,
                    node_type: &NodeType::Execute("Enter"),
                    label: "Enter",
                }),
            MyNodeTemplate::Print =>
                &(NodeParams {
                    shape_shift_category: None,
                    node_type: &NodeType::ExecutedAndExecute("", ""),
                    label: "Print",
                }),
            MyNodeTemplate::Ask =>
                &(NodeParams {
                    shape_shift_category: None,
                    node_type: &NodeType::ExecutedAndExecute("", ""),
                    label: "Ask",
                }),
            MyNodeTemplate::If =>
                &(NodeParams {
                    shape_shift_category: None,
                    node_type: &NodeType::ExecutedAndExecute("", "Continue"),
                    label: "If",
                }),

            MyNodeTemplate::CategoryAdd => {
                &(NodeParams {
                    shape_shift_category: Some(ShapeShiftCategory::Add),
                    node_type: &NodeType::Data,
                    label: "Add two types together",
                })
            }
            MyNodeTemplate::AddNumber =>
                &(NodeParams {
                    shape_shift_category: Some(ShapeShiftCategory::Add),
                    node_type: &NodeType::Data,
                    label: "Add Number",
                }),
            MyNodeTemplate::AddString =>
                &(NodeParams {
                    shape_shift_category: Some(ShapeShiftCategory::Add),
                    node_type: &NodeType::Data,
                    label: "Add String",
                }),
            MyNodeTemplate::Function(_) =>
                &(NodeParams {
                    shape_shift_category: None,
                    node_type: &NodeType::ExecutedAndExecute("", ""),
                    label: "Function",
                }),
        }
    }
}

impl MyNodeTemplate {
    pub fn evaluate_data(
        &self,
        graph: &MyGraph,
        output_node: &Node<MyNodeData>,
        outputs_cache: &HashMap<OutputId, String>,
        inputs: &Vec<String>
    ) -> String {
        match self {
            Self::AddNumber => format!("(+ {} {})", inputs[0], inputs[1]),
            Self::AddString => format!("(.. {} {})", inputs[0], inputs[1]),
            _ => String::new(),
        }
    }

    pub fn compile_to(
        &self,
        outputs_cache: &HashMap<OutputId, String>,
        executions: &Vec<String>,
        filtered_inputs: &Vec<String>,
        next_node: &Node<MyNodeData>
    ) -> String {
        match self {
            Self::Enter => "".to_string(),
            Self::Print => format!("(io.write {})", filtered_inputs[0]),
            Self::Ask => {
                format!(
                    "(io.write {}) (local {} (io.read))",
                    filtered_inputs[0],
                    outputs_cache.get(&next_node.outputs[1].1).unwrap()
                )
            }
            Self::If => {
                format!(
                    "(if (= {} 1) (do {}) (do {}))",
                    filtered_inputs[0],
                    executions.get(1).map_or("", |x| x),
                    executions.get(2).map_or("", |x| x)
                )
            }
            Self::Function(_) => "".to_string(),
            _ => String::new(),
        }
    }
}

// A trait for the node kinds, which tells the library how to build new nodes
// from the templates in the node finder
impl NodeTemplateTrait for MyNodeTemplate {
    type NodeData = MyNodeData;
    type DataType = types::MyDataType;
    type ValueType = types::MyValueType;
    type UserState = app::MyGraphState;
    type CategoryType = &'static str;

    fn node_finder_label(&self, _user_state: &mut Self::UserState) -> Cow<'_, str> {
        Cow::Borrowed(self.get_node_params().label)
    }

    // this is what allows the library to show collapsible lists in the node finder.
    fn node_finder_categories(&self, _user_state: &mut Self::UserState) -> Vec<&'static str> {
        match self {
            MyNodeTemplate::AddString | MyNodeTemplate::AddNumber => vec![],
            MyNodeTemplate::CategoryAdd | MyNodeTemplate::If => vec!["Logic"],
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
        let classic_input = |
            graph: &mut app::MyGraph,
            name: &str,
            typ: types::MyDataType,
            value: types::MyValueType
        | {
            graph.add_input_param(
                node_id,
                name.to_string(),
                typ,
                value,
                InputParamKind::ConnectionOrConstant,
                true
            );
        };
        let classic_output = |graph: &mut app::MyGraph, name: &str, typ: types::MyDataType| {
            graph.add_output_param(node_id, name.to_string(), typ);
        };

        let exe_input = |graph: &mut app::MyGraph, name: &str| {
            graph.add_input_param(
                node_id,
                name.to_string(),
                types::MyDataType::Execution,
                types::MyValueType::Execution,
                InputParamKind::ConnectionOnly,
                true
            );
        };
        let exe_output = |graph: &mut app::MyGraph, name: &str| {
            graph.add_output_param(node_id, name.to_string(), types::MyDataType::Execution);
        };

        match self.get_node_params().node_type {
            NodeType::Execute(x) => {
                exe_output(graph, x);
            }
            NodeType::Executed(x) => {
                exe_input(graph, x);
            }
            NodeType::ExecutedAndExecute(x, y) => {
                exe_input(graph, x);
                exe_output(graph, y);
            }
            NodeType::Data => {}
        }

        match self {
            MyNodeTemplate::Enter => {}

            Self::CategoryAdd => {}
            Self::AddNumber => {
                classic_input(
                    graph,
                    "What ?",
                    types::MyDataType::Integer,
                    types::MyValueType::Integer {
                        value: 0,
                    }
                );

                classic_input(
                    graph,
                    "What ?",
                    types::MyDataType::Integer,
                    types::MyValueType::Integer {
                        value: 0,
                    }
                );
                classic_output(graph, "What ?", types::MyDataType::Integer);
            }
            
            Self::AddString => {
                classic_input(
                    graph,
                    "What ?",
                    types::MyDataType::String,
                    types::MyValueType::String {
                        value: String::new(),
                    }
                );

                classic_input(
                    graph,
                    "What ?",
                    types::MyDataType::String,
                    types::MyValueType::String {
                        value: String::new(),
                    }
                );
                classic_output(graph, "What ?", types::MyDataType::String);
            }
            MyNodeTemplate::Ask => {
                classic_input(
                    graph,
                    "What ?",
                    types::MyDataType::String,
                    types::MyValueType::String {
                        value: "".to_string(),
                    }
                );
                classic_output(graph, "Answer", types::MyDataType::String);
            }
            MyNodeTemplate::If => {
                exe_output(graph, "If");
                exe_output(graph, "Else");
                classic_input(graph, "", types::MyDataType::Integer, types::MyValueType::Integer {
                    value: 0,
                });
            }
            MyNodeTemplate::Print => {
                classic_input(
                    graph,
                    "What ?",
                    types::MyDataType::String,
                    types::MyValueType::String {
                        value: "".to_string(),
                    }
                );
            }
            MyNodeTemplate::Function(x) => {
                if let Some(function_index) = x {
                    for input in user_state.functions[*function_index].input.iter() {
                        match &input.value {
                            types::VariableValue::Boolean(x) => {
                                classic_input(
                                    graph,
                                    &input.name,
                                    types::MyDataType::Boolean,
                                    types::MyValueType::Boolean { value: x.to_owned() }
                                );
                            }
                            types::VariableValue::String(x) => {
                                classic_input(
                                    graph,
                                    &input.name,
                                    types::MyDataType::String,
                                    types::MyValueType::String { value: x.to_owned() }
                                );
                            }
                            types::VariableValue::Integer(x) => {
                                classic_input(
                                    graph,
                                    &input.name,
                                    types::MyDataType::Integer,
                                    types::MyValueType::Integer { value: x.to_owned() as i32 }
                                );
                            }
                            types::VariableValue::Float(x) => {
                                classic_input(
                                    graph,
                                    &input.name,
                                    types::MyDataType::Float,
                                    types::MyValueType::Float { value: x.to_owned() }
                                );
                            }
                            types::VariableValue::Execution => {
                                exe_input(graph, &input.name);
                            }
                        }
                    }
                    for output in user_state.functions[*function_index].output.iter() {
                        match &output.value {
                            types::VariableValue::Boolean(_) => {
                                classic_output(graph, &output.name, types::MyDataType::Boolean);
                            }
                            types::VariableValue::String(_) => {
                                classic_output(graph, &output.name, types::MyDataType::String);
                            }
                            types::VariableValue::Integer(_) => {
                                classic_output(graph, &output.name, types::MyDataType::Integer);
                            }
                            types::VariableValue::Float(_) => {
                                classic_output(graph, &output.name, types::MyDataType::Float);
                            }
                            types::VariableValue::Execution => {
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

        let mut vec = vec![];

        for x in MyNodeTemplate::iter() {
            vec.push(x);
        }
        vec
    }
}

impl UserResponseTrait for app::MyResponse {}
impl NodeDataTrait for MyNodeData {
    type Response = app::MyResponse;
    type UserState = app::MyGraphState;
    type DataType = types::MyDataType;
    type ValueType = types::MyValueType;

    // This method will be called when drawing each node. This allows adding
    // extra ui elements inside the nodes. In this case, we create an "active"
    // button which introduces the concept of having an active node in the
    // graph. This is done entirely from user code with no modifications to the
    // node graph library.
    fn bottom_ui(
        &self,
        ui: &mut egui::Ui,
        node_id: NodeId,
        graph: &Graph<MyNodeData, types::MyDataType, types::MyValueType>,
        user_state: &mut Self::UserState
    ) -> Vec<NodeResponse<app::MyResponse, MyNodeData>>
        where app::MyResponse: UserResponseTrait
    {
        let mut responses = vec![];

        // This logic is entirely up to the user. In this case, we check if the
        // current node we're drawing is the active one, by comparing against
        // the value stored in the global user state, and draw different button
        // UIs based on that.

        if let Some(category) = &self.template.get_node_params().shape_shift_category {
            let mut nodes = vec![];
            for x in MyNodeTemplate::iter() {
                if let Some(y) = &x.get_node_params().shape_shift_category {
                    if category == y {
                        nodes.push(x);
                    }
                }
            }

            let mut current_value = self.template.clone();

            egui::ComboBox
                ::from_id_source(node_id)
                .selected_text(self.template.node_graph_label(user_state))
                .width(74.0)
                .show_ui(ui, |ui| {
                    for x in nodes.iter() {
                        ui.selectable_value(
                            &mut current_value,
                            *x,
                            &x.node_graph_label(user_state)
                        );
                    }
                });
            if current_value != self.template {
                responses.push(
                    NodeResponse::User(app::MyResponse::NodeShapeShiftFromCategory(node_id, current_value))
                );
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
                        if !(app::DISABLE_RECURSIVE_FUNCTIONS && user_state.graph_id == x.0) {
                            if user_state.main_graph_id != x.0 {
                                ui.selectable_value(&mut current_value, Some(x.0), &x.1.name);
                            }
                        }
                    }
                });

            if value != current_value {
                responses.push(
                    NodeResponse::User(app::MyResponse::AsignFunction(node_id, current_value))
                );
            }
        }

        responses
    }
}
