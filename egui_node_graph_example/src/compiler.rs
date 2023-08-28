use std::collections::HashMap;

use egui_node_graph::{ NodeId, OutputId, InputId, Node };
use slotmap::Key;

use crate::app::{ self, MyGraph, MyValueType, MyDataType, MyNodeData };

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
    next_node: &Node<MyNodeData>,
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

    let script_line = match next_node.user_data.template {
        app::MyNodeTemplate::Enter => "".to_string(),
        app::MyNodeTemplate::Print => format!("io.write({})", filtered_inputs[0]),
        app::MyNodeTemplate::Ask => {
            format!(
                "local {} = io.read({}, (func -> {}))",
                outputs_cache.get(&next_node.outputs[2].1).unwrap(),
                filtered_inputs[0],
                executions.get(1).map_or("default", |x| x),
            )
        }
        app::MyNodeTemplate::Function(_) => "".to_string(),
    };

    println!("THE LINE -> {}", script_line);

    format!(
        "{} -- {}",
        script_line,
        executions.get(0).map_or("default", |x| x)
    )

    // for x in graph.iter_connections() {
    //     if
    //         node
    //             .output_ids()
    //             .find(|y| y == &x.1)
    //             .is_some()
    //     {
    //         // println!("{:?}", graph[x.0].node);

    //         if let MyDataType::Execution = graph[x.0].typ {
    //             evaluate_functionn(graph, graph[x.0].node, outputs_cache);
    //         }

    //     }
    // }

    // match node.user_data.template {
    //     app::MyNodeTemplate::Enter => {}
    //     app::MyNodeTemplate::Print => {}
    //     app::MyNodeTemplate::Ask => {}
    //     app::MyNodeTemplate::Function(_) => {}
    // }
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
                                MyValueType::Execution { value } =>
                                    evaluate_function(graph, current_node.unwrap()),
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
                    app::MyNodeTemplate::Ask =>
                        format!(
                            "local {} = io.read({})",
                            outputs_cache.get(&node.outputs[1].1).unwrap(),
                            filtered_outputs[0]
                        ),
                    app::MyNodeTemplate::Function(_) => "".to_string(),
                };

                script = format!("{script}\n{script_line}");
            }
        }
        if next_node.is_none() {
            break;
        }
        current_node = std::mem::replace(&mut next_node, None);
    }

    script
}
