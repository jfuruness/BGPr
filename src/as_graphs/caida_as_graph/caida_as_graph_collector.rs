use std::fs::create_dir_all;
use std::path::PathBuf;
use chrono::{Duration, Utc, TimeZone};
use platform_dirs::AppDirs;


//use super::base::{BaseASGraphCollector, ASGraphCollector};
use crate::as_graphs::base::as_graph_collector::{BaseASGraphCollector, ASGraphCollector};
use crate::define_base_asgraph_collector;

// Use the macro to define the base collector type and logic.
define_base_asgraph_collector!(CAIDAASGraphCollector);

// Now implement the final trait which requires _run().
// Here you add your custom run logic.
impl ASGraphCollector for CAIDAASGraphCollector {
    fn default_dl_time() -> chrono::DateTime<chrono::Utc> {
        let today = Utc::now().date_naive();
        let target_date = today - Duration::days(10);
        let midnight = target_date.and_hms_micro_opt(0, 0, 0, 0).unwrap();
        Utc.from_utc_datetime(&midnight)
    }

    fn _run(&self) -> Result<PathBuf, Box<dyn std::error::Error>> {
        println!("Running custom logic for CAIDAASGraphCollector...");
        Ok(self.cache_path().clone())
    }
}
