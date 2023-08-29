use std::fmt;
use std::borrow::Cow;

use eframe::egui::{self, DragValue};
use egui_node_graph::*;

#[cfg(feature = "persistence")]
use bincode;
#[cfg(feature = "persistence")]
use serde::{ Deserialize, Serialize };

use crate::app;
use crate::nodes;

// ========= First, define your user data types =============


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
    Execution {
        value: String
    },
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


// =========== Then, you need to implement some traits ============

// A trait for the data types, to tell the library how to display them
impl DataTypeTrait<app::MyGraphState> for MyDataType {
    fn data_type_color(&self, _user_state: &mut app::MyGraphState) -> egui::Color32 {
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


impl WidgetValueTrait for MyValueType {
    type Response = app::MyResponse;
    type UserState = app::MyGraphState;
    type NodeData = nodes::MyNodeData;
    fn value_widget(
        &mut self,
        param_name: &str,
        _node_id: NodeId,
        ui: &mut egui::Ui,
        _user_state: &mut app::MyGraphState,
        _node_data: &nodes::MyNodeData,
        _kind: InputParamKind
    ) -> Vec<app::MyResponse> {
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
            MyValueType::Execution { value: _ } => {
                ui.horizontal(|ui| {
                    ui.label(param_name);
                });
            }
        }
        // This allows you to return your responses from the inline widgets.
        Vec::new()
    }
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