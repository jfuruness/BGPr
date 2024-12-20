mod as_graphs;

use as_graphs::{CAIDAASGraphCollector,CAIDAASGraphConstructor};
use crate::as_graphs::{BaseASGraphCollector, ASGraphCollector};

fn main() {
    let collector = CAIDAASGraphCollector::new(None, None);
    let constructor = CAIDAASGraphConstructor::new(collector);
    constructor.run();
}
