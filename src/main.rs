mod as_graphs;

use as_graphs::CAIDAASGraphCollector;
use crate::as_graphs::{BaseASGraphCollector, ASGraphCollector};

fn main() {
    let collector = CAIDAASGraphCollector::new(None, None);
    collector.run();
}
