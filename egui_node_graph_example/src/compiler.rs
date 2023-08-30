use std::collections::HashMap;

use egui_node_graph::{ NodeId, OutputId, InputId, Node };
use slotmap::Key;

use crate::app::{ self, MyGraph };
use crate::nodes::{ self };
use crate::types::{ self, VariableValue };

pub fn compile(
    app_state: &app::AppState,
    enter_node: nodes::MyNodeTemplate
) -> anyhow::Result<String, String> {
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

    let mut result = String::new();

    for x in &app_state.functions.get(app_state.main_graph_id).unwrap().variables_list {
        result = format!("{} (local {} {})", result, x.name, match &x.value {
            VariableValue::Boolean(y) => y.to_string(),
            VariableValue::Float(y) => y.to_string(),
            VariableValue::Integer(y) => y.to_string(),
            VariableValue::String(y) => format!("{:?}", y),
            VariableValue::Execution => String::new(),
        });
    }

    result = format!("{} {:?}", result, evaluate_function(graph, enter_node, &mut HashMap::new()));

    Ok(result)
}

fn evaluate_function(
    graph: &MyGraph,
    next_node: &Node<nodes::MyNodeData>,
    outputs_cache: &mut HashMap<OutputId, String>
) -> Result<String, (OutputId, InputId)> {
    let mut inputs = vec![];

    for y in next_node.inputs(graph) {
        if y.typ == types::MyDataType::Execution {
            inputs.push(None);
            continue;
        }
        if let Some(z) = graph.connection(y.id) {
            let mut input = outputs_cache
                .get(&z)
                .clone()
                .map_or(None, |x| Some(x.to_string()));
            if input.is_none() {
                input = Some(
                    evaluate_output(
                        &graph,
                        &graph[graph.get_output(z).node],
                        outputs_cache,
                        &mut vec![]
                    )?
                );
            }
            inputs.push(input);
        } else {
            inputs.push(
                Some(match &y.value {
                    types::MyValueType::String { value } => format!("\"{value}\""),
                    types::MyValueType::Integer { value } => value.to_string(),
                    types::MyValueType::Float { value } => value.to_string(),
                    types::MyValueType::Boolean { value } => value.to_string(),
                    types::MyValueType::Execution => String::new(),
                })
            );
        }
    }
    let filtered_inputs: Vec<String> = inputs
        .into_iter()
        .filter_map(|option| option)
        .collect();

    let mut executions = vec![];
    let mut executions_index = vec![];

    for y in next_node.outputs(graph) {
        if y.typ == types::MyDataType::Execution {
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
                        evaluate_function(graph, z, outputs_cache)?
                    } else {
                        String::new()
                    }
                );
            }
        }
    }

    let script_line = next_node.user_data.template.compile_to(
        outputs_cache,
        &executions,
        &filtered_inputs,
        next_node
    );

    Ok(format!(
        "{} {}",
        script_line,
        executions.get(0).map_or("", |x| x)
    ))
}

fn evaluate_output(
    graph: &MyGraph,
    output_node: &Node<nodes::MyNodeData>,
    outputs_cache: &mut HashMap<OutputId, String>,
    already_explored_nodes: &mut Vec<NodeId>
) -> Result<String, (OutputId, InputId)> {
    already_explored_nodes.push(output_node.id);

    let mut inputs = vec![];

    for x in output_node.input_ids() {
        if let Some(y) = graph.connection(x) {
            if already_explored_nodes.contains(&graph[graph[y].node].id) {
                return Err((y, x));
            } else {
                let output = evaluate_output(
                    graph,
                    &graph[graph[y].node],
                    outputs_cache,
                    already_explored_nodes
                )?;
                outputs_cache.insert(y, output.clone()); // technically, its not nessesary to put it in the cache, but if we dont want to recalculate it agin, thats preferable
                inputs.push(output);
            }
        } else {
            inputs.push(match &graph.get_input(x).value {
                types::MyValueType::String { value } => format!("\"{value}\""),
                types::MyValueType::Integer { value } => value.to_string(),
                types::MyValueType::Float { value } => value.to_string(),
                types::MyValueType::Boolean { value } => value.to_string(),
                types::MyValueType::Execution => String::new(),
            });
        }
    }
    return Ok(output_node.user_data.template.evaluate_data(graph, output_node, outputs_cache, &inputs));
}
