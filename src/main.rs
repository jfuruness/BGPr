mod as_graphs;

use as_graphs::{CAIDAASGraphCollector,CAIDAASGraphConstructor, AutonomousSystem, ASGraph};
use crate::as_graphs::{BaseASGraphCollector, ASGraphCollector};

fn main() {
    let collector = CAIDAASGraphCollector::new(None, None);
    let constructor = CAIDAASGraphConstructor::new(collector);
    let as_graph_info = constructor.run().unwrap();
    let as_graph = ASGraph::new(as_graph_info);
}
