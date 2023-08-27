use std::collections::HashMap;

use egui_node_graph::{ NodeId, OutputId, InputParamKind, InputParam };
use slotmap::Key;

use crate::app::{ self, NodeGraphExample, MyGraphState, MyGraph, MyValueType, MyDataType };

pub fn compile(
    app_state: &app::AppState,
    enter_node: app::MyNodeTemplate
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

    println!("\n\n-----------\n\n");

    evaluate_function(
        &app_state.functions.get(app_state.main_graph_id).unwrap().graph.state.graph,
        enter_node_id
    );

    println!("\n\n-----------\n\n");

    Ok(app::MyValueType::Boolean { value: true })
}

fn evaluate_function(graph: &MyGraph, enter_node: NodeId) -> String {
    //println!("{:?}", graph.nodes.get(enter_node).unwrap());

    let mut script = String::new();

    let mut outputs_cache: HashMap<OutputId, String> = HashMap::new();

    let mut current_node = Some(enter_node);
    let mut next_node = None;

    loop {
        for x in graph.iter_connections() {
            if x.1 == graph.nodes.get(current_node.unwrap()).unwrap().outputs[0].1 {
                let node = graph.nodes.get(graph[x.0].node).unwrap();
                next_node = Some(node.id);

                let mut outputs = vec![];

                for y in node.inputs(graph) {
                    if y.typ == MyDataType::Execution {
                        outputs.push(None);
                        continue;
                    }
                    if let Some(z) = graph.connection(y.id) {
                        outputs.push(
                            outputs_cache
                                .get(&z)
                                .clone()
                                .map_or(None, |x| Some(x.to_string()))
                        );
                    } else {
                        outputs.push(
                            Some(match &y.value {
                                MyValueType::String { value } => format!("\"{value}\""),
                                MyValueType::Integer { value } => value.to_string(),
                                MyValueType::Float { value } => value.to_string(),
                                MyValueType::Boolean { value } => value.to_string(),
                                MyValueType::Execution => "".to_string(),
                            })
                        );
                    }
                }

                let filtered_outputs: Vec<String> = outputs
                    .into_iter()
                    .filter_map(|option| option)
                    .collect();

                println!("{:?}", filtered_outputs);
                println!("{:?}", graph.nodes.get(graph[x.0].node).unwrap().user_data.template);


                
                for y in node.outputs(graph) {
                    if y.typ == MyDataType::Execution {
                        continue;
                    }
                    outputs_cache.insert(y.id, format!("var_{:?}", y.id.data()));
                }

                let script_line = match
                    graph.nodes.get(graph[x.0].node).unwrap().user_data.template
                {
                    app::MyNodeTemplate::Enter => "".to_string(),
                    app::MyNodeTemplate::Print => format!("io.write({})", filtered_outputs[0]),
                    app::MyNodeTemplate::Ask => format!("local {} = io.read({})", outputs_cache.get(&node.outputs[1].1).unwrap(), filtered_outputs[0]),
                    app::MyNodeTemplate::Function(_) => "".to_string()
                };

                script = format!("{script}\n{script_line}");
            }
        }
        if next_node.is_none() {
            break;
        }
        current_node = std::mem::replace(&mut next_node, None);
        println!("");
    }

    println!("{script}");
    "".to_owned()
}
