use std::path::PathBuf;
use chrono::{DateTime, Utc};
use std::fs;
use log::error;

// The base trait with shared logic.
// Notice: No _run() or run() here.
pub trait BaseASGraphCollector {
    fn new(dl_time: Option<DateTime<Utc>>, cache_dir: Option<PathBuf>) -> Self;
    fn default_cache_dir() -> PathBuf;
    fn name() -> &'static str;
    fn cache_path(&self) -> &PathBuf;
}

// The final trait that requires _run() and provides run().
// It depends on BaseASGraphCollector.
pub trait ASGraphCollector: BaseASGraphCollector {
    fn default_dl_time() -> chrono::DateTime<chrono::Utc> {
        chrono::Utc::now()
    }


    fn _run(&self) -> Result<PathBuf, Box<dyn std::error::Error>>;

    fn run(&self) -> PathBuf {
        match self._run() {
            Ok(path) => path,
            Err(e) => {
                error!(
                    "Error {:?}, deleting cached AS graph file at {:?}",
                    e, self.cache_path()
                );
                let _ = fs::remove_file(self.cache_path());
                panic!("Error during run: {:?}", e);
            }
        }
    }
}

// Local macros are experimental, afaik this is the only usable format for now
#[macro_export]
macro_rules! define_base_asgraph_collector {
    ($name:ident) => {
        pub struct $name {
            dl_time: chrono::DateTime<chrono::Utc>,
            cache_dir: std::path::PathBuf,
            cache_path: std::path::PathBuf,
        }

        impl BaseASGraphCollector for $name {
            fn name() -> &'static str {
                stringify!($name)
            }

            fn default_cache_dir() -> std::path::PathBuf {
                let base_cache_dir = AppDirs::new(Some("BGPr"), false)
                    .unwrap()
                    .cache_dir;
                let current_date_str = chrono::Utc::now().format("%Y_%m_%d").to_string();
                base_cache_dir.join(current_date_str)
            }

            fn new(
                dl_time: Option<chrono::DateTime<chrono::Utc>>,
                cache_dir: Option<std::path::PathBuf>
            ) -> Self {
                let dl_time = dl_time.unwrap_or_else(Self::default_dl_time);
                let cache_dir = cache_dir.unwrap_or_else(Self::default_cache_dir);

                if !cache_dir.exists() {
                    create_dir_all(&cache_dir).expect("Failed to create cache directory");
                }

                let fmt = format!("{}_{}.txt", Self::name(), dl_time.format("%Y.%m.%d"));
                let cache_path = cache_dir.join(fmt);

                Self {
                    dl_time,
                    cache_dir,
                    cache_path,
                }
            }

            fn cache_path(&self) -> &std::path::PathBuf {
                &self.cache_path
            }
        }
    };
}


