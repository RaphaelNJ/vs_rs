use std::collections::HashMap;

use egui_node_graph::{ NodeId, OutputId, InputId, Node };
use slotmap::Key;

use crate::app::{ self, MyGraph, MyValueType, MyDataType };
use crate::nodes::{self, CompilesTo};

pub fn compile(
    app_state: &app::AppState,
    enter_node: nodes::MyNodeTemplate
) -> anyhow::Result<app::MyValueType, String> {
    let mut already_a_enter = false;
    let mut is_enter_node_id = None;
    for x in app_state.functions.iter() {
        for y in x.1.graph.state.graph.nodes.iter() {
            if y.1.user_data.template == enter_node {
                if x.0 == app_state.main_graph_id {
                    if !already_a_enter {
                        is_enter_node_id = Some(y.0);
                        already_a_enter = true;
                    } else {
                        return Err("You have Too many Enter Nodes".to_string());
                    }
                } else {
                    return Err("A Enter node in a function".to_string());
                }
            }
        }
    }
    let enter_node_id = if let Some(x) = is_enter_node_id {
        x
    } else {
        return Err("You don't have any Enter node".to_string());
    };

    let graph = &app_state.functions.get(app_state.main_graph_id).unwrap().graph.state.graph;
    let enter_node = graph.nodes.get(enter_node_id).unwrap();

    println!("\n\n-----------\n\n");

    let result = evaluate_functionn(graph, enter_node, &mut HashMap::new());

    println!("\n\n-----------\n\n");


    println!("{}", result);

    Ok(app::MyValueType::Boolean { value: true })
}

fn evaluate_functionn(
    graph: &MyGraph,
    next_node: &Node<nodes::MyNodeData>,
    outputs_cache: &mut HashMap<OutputId, String>
) -> String {
    // println!("{}", node.label);

    let mut inputs = vec![];

    for y in next_node.inputs(graph) {
        if y.typ == MyDataType::Execution {
            inputs.push(None);
            continue;
        }
        if let Some(z) = graph.connection(y.id) {
            inputs.push(
                outputs_cache
                    .get(&z)
                    .clone()
                    .map_or(None, |x| Some(x.to_string()))
            );
            println!("{:?} -- {:?}", z, outputs_cache);
        } else {
            inputs.push(
                Some(match &y.value {
                    MyValueType::String { value } => format!("\"{value}\""),
                    MyValueType::Integer { value } => value.to_string(),
                    MyValueType::Float { value } => value.to_string(),
                    MyValueType::Boolean { value } => value.to_string(),
                    MyValueType::Execution { value } => "".to_string(),
                })
            );
        }
    }
    let filtered_inputs: Vec<String> = inputs
        .into_iter()
        .filter_map(|option| option)
        .collect();

    println!("{:?}", filtered_inputs);
    println!("{:?}", next_node.user_data.template);

    let mut executions = vec![];
    let mut executions_index = vec![];

    for y in next_node.outputs(graph) {
        if y.typ == MyDataType::Execution {
            executions_index.push(y.id);
            continue;
        }
        outputs_cache.insert(y.id, format!("var_{:?}", y.id.data()));
    }

    for y in executions_index.iter() {
        for x in graph.iter_connections() {
            if x.1 == *y {
                executions.push(
                    if let Some(z) = graph.nodes.get(graph[x.0].node) {
                        evaluate_functionn(graph, z, outputs_cache)
                    } else {
                        "".to_owned()
                    }
                );
            }
        }
    }

    println!("exe -> {:?}", executions);

    let script_line = next_node.user_data.template.compile_to(outputs_cache, &executions, &filtered_inputs, next_node);

    println!("THE LINE -> {}", script_line);

    format!(
        "{} {}",
        script_line,
        executions.get(0).map_or("", |x| x)
    )
}