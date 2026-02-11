#[cfg(test)]
mod test_node_data {
    use crate::node::{Node, NodePosition, NodeType};
    use crate::node_config::NodeData;

    #[test]
    fn print_node_serialization() {
        let node = Node {
            uuid: "start-node".to_string(),
            node_type: NodeType::Entry,
            name: "Start Node".to_string(),
            position: NodePosition { x: 100.0, y: 100.0 },
            data: NodeData::Start(()),
            ports: None,
        };

        let json = serde_json::to_string_pretty(&node).unwrap();
        println!("{}", json);
    }
}
