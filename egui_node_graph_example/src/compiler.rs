use crate::app;
use egui_node_graph::*;


pub fn compile(
    graph: &app::MyGraph,
    node_id: NodeId,
    outputs_cache: &mut app::OutputsCache
) -> anyhow::Result<app::MyValueType> {
    







    Ok(app::MyValueType::Boolean { value: true })
}