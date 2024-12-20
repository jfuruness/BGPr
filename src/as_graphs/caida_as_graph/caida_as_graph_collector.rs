use platform_dirs::AppDirs;
use std::fs::create_dir_all;
use std::fs::{File};
use std::io::{self, Read, Write};
use std::path::PathBuf;
use chrono::{DateTime, Duration, Utc};
use bzip2::read::BzDecoder;
use reqwest::blocking::Client;
use scraper::{Html, Selector};
use tempfile::TempDir;
use crate::as_graphs::base::as_graph_collector::{BaseASGraphCollector, ASGraphCollector};
use crate::define_base_asgraph_collector;

// Use the macro to define the base collector type and logic.
define_base_asgraph_collector!(CAIDAASGraphCollector);

impl ASGraphCollector for CAIDAASGraphCollector {
    fn default_dl_time() -> chrono::DateTime<chrono::Utc> {
        let dl_time = Utc::now() - Duration::days(10);
        dl_time.date_naive().and_hms_opt(0, 0, 0).unwrap().and_utc()
    }

    fn _run(&self) -> Result<PathBuf, Box<dyn std::error::Error>> {
        if !self.cache_path().exists() {
            println!("No cached CAIDA graph. Caching...");
            let tmp_dir = TempDir::new()?;
            let bz2_path = tmp_dir.path().join("download.bz2");

            self.download_bz2_file(
                &self.get_url(self.dl_time)?,
                &bz2_path,
            )?;
            self.unzip_and_write_to_cache(&bz2_path)?;
        }
        Ok(self.cache_path().clone())
    }
}

impl CAIDAASGraphCollector {
    fn get_url(&self, dl_time: DateTime<Utc>) -> Result<String, Box<dyn std::error::Error>> {
        let base_url = "http://data.caida.org/datasets/as-relationships/serial-2/";
        let hrefs = self.get_hrefs(base_url)?;

        let target_date = dl_time.format("%Y%m01").to_string();
        for href in hrefs {
            if href.contains(&target_date) {
                return Ok(format!("{}{}", base_url, href));
            }
        }

        Err("No URLs found for the specified download time".into())
    }

    fn get_hrefs(&self, url: &str) -> Result<Vec<String>, Box<dyn std::error::Error>> {
        let client = Client::new();
        let response = client.get(url).send()?.text()?;

        let document = Html::parse_document(&response);
        let selector = Selector::parse("a").unwrap();

        let hrefs = document
            .select(&selector)
            .filter_map(|a| a.value().attr("href"))
            .map(String::from)
            .collect();

        Ok(hrefs)
    }

    fn download_bz2_file(&self, url: &str, bz2_path: &PathBuf) -> Result<(), Box<dyn std::error::Error>> {
        let client = Client::new();
        let mut response = client.get(url).send()?;
        let mut file = File::create(bz2_path)?;

        io::copy(&mut response, &mut file)?;
        Ok(())
    }

    fn unzip_and_write_to_cache(&self, bz2_path: &PathBuf) -> io::Result<()> {
        let mut bz2_file = File::open(bz2_path)?;
        let mut decoder = BzDecoder::new(&mut bz2_file);
        let mut cache_file = File::create(self.cache_path())?;

        let mut buffer = String::new();
        decoder.read_to_string(&mut buffer)?;

        for line in buffer.lines() {
            writeln!(cache_file, "{}", line.trim())?;
        }

        Ok(())
    }
}
