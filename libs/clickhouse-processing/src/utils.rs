#![allow(dead_code)]
use anyhow::anyhow;
use block_mesh_common::interfaces::server_api::FeedElement;
use clickhouse::Client;
use csv::ReaderBuilder;
use regex::Regex;
use serde::{Deserialize, Serialize};
use sqlx::types::chrono::{NaiveDate, Utc};
use std::collections::HashMap;
use std::fs::File;
use std::io::{BufRead, Write};
use std::path::PathBuf;
use std::sync::Arc;
use std::{fs, io};
use twitter_scraping_helper::reactive::feed_element_try_from::feed_element_try_from;
use uuid::Uuid;

#[derive(Debug, Deserialize, Serialize)]
pub struct Record {
    pub user_name: String,
    pub origin_id: String,
}

#[derive(Debug, Serialize, Deserialize, Clone, clickhouse::Row)]
pub struct Output {
    pub user: String,
    pub id: String,
    pub link: String,
    pub tweet: String,
    pub date: String,
    pub reply: String,
    pub retweet: String,
    pub like: String,
}

#[derive(Debug, Serialize, Deserialize, Clone, clickhouse::Row)]
pub struct Output2 {
    pub user_name: String,
    pub origin_id: String,
    pub link: String,
    pub event_date: String,
    pub tweet: String,
    pub reply: u32,
    pub retweet: u32,
    pub like: u32,
}

impl From<Output2> for Output {
    fn from(value: Output2) -> Self {
        Self {
            user: value.user_name,
            id: value.origin_id,
            link: value.link,
            tweet: value.tweet,
            date: value.event_date,
            reply: value.reply.to_string(),
            retweet: value.retweet.to_string(),
            like: value.like.to_string(),
        }
    }
}

impl Output {
    pub fn merge(element: FeedElement, data_sink: DataSinkClickHouse) -> Self {
        Self {
            user: element.user_name,
            id: element.id,
            link: element.link,
            tweet: element.tweet.unwrap_or_default(),
            date: data_sink.created_at,
            reply: element.reply.unwrap_or_default().to_string(),
            retweet: element.retweet.unwrap_or_default().to_string(),
            like: element.like.unwrap_or_default().to_string(),
        }
    }

    pub fn merge_mini(element: FeedElement, date: String) -> Self {
        Self {
            user: element.user_name,
            id: element.id,
            link: element.link,
            tweet: element.tweet.unwrap_or_default(),
            date,
            reply: element.reply.unwrap_or_default().to_string(),
            retweet: element.retweet.unwrap_or_default().to_string(),
            like: element.like.unwrap_or_default().to_string(),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Raw {
    pub raw: String,
}

#[derive(Debug, Serialize, Deserialize, Clone, clickhouse::Row)]
pub struct DataSinkClickHouse {
    #[serde(with = "clickhouse::serde::uuid")]
    pub id: Uuid,
    #[serde(with = "clickhouse::serde::uuid")]
    pub user_id: Uuid,
    pub origin: String,
    pub origin_id: String,
    pub user_name: String,
    pub link: String,
    pub created_at: String,
    pub updated_at: String,
    pub raw: String,
    pub reply: String,
    pub retweet: String,
    pub like: String,
    pub tweet: String,
}

#[derive(Debug, Serialize, Deserialize, Clone, clickhouse::Row)]
pub struct DataSinkClickHouseMini {
    pub origin_id: String,
    pub raw: String,
}

pub fn read_files_from_dir(path: &str) -> anyhow::Result<Vec<PathBuf>> {
    // Read the directory
    let entries = fs::read_dir(path).map_err(|e| {
        eprintln!("[read_files_from_dir]::Error = {} | Path = {}", e, path);
        anyhow!(e)
    })?;
    let mut files: Vec<PathBuf> = Vec::new();

    // Iterate through entries
    for entry in entries {
        let entry = entry?;
        let path = entry.path();

        // Check if it's a file
        if path.is_file() {
            files.push(path);
        }
    }
    Ok(files)
}

pub fn process_raw(raws: Vec<Raw>, limit: i32, date: String) -> Vec<Output> {
    let mut output: Vec<Output> = Vec::with_capacity(1_000_000);
    let origin = "https://x.com";
    for (count, raw) in raws.iter().enumerate() {
        if count % 1_000 == 0 {
            println!("[{}]::count = {}", Utc::now(), count);
        }
        match feed_element_try_from(&raw.raw, origin) {
            Ok(element) => {
                output.push(Output::merge_mini(element, date.clone()));
            }
            Err(e) => eprintln!("error = {}", e),
        }
        if limit > 0 && count > limit as usize {
            break;
        }
    }
    output
}

pub fn read_csv_file<T>(path: &str) -> Vec<T>
where
    T: for<'a> Deserialize<'a>,
{
    let mut reader = ReaderBuilder::new().from_path(path).unwrap();
    let mut records: Vec<T> = Vec::with_capacity(3_000_000);
    for result in reader.deserialize() {
        let record: T = result.unwrap();
        records.push(record);
    }
    records
}

pub fn file_date(input: &str) -> anyhow::Result<NaiveDate> {
    let date_pattern = Regex::new(r"(?P<year>\d{4})-(?P<month>\d{2})-(?P<day>\d{2})")?;

    if let Some(captures) = date_pattern.captures(input) {
        // Extract each named group
        let year = captures.name("year").unwrap().as_str();
        let month = captures.name("month").unwrap().as_str();
        let day = captures.name("day").unwrap().as_str();
        println!("Year: {} | Month: {} | Day: {}", year, month, day);
        Ok(NaiveDate::from_ymd_opt(year.parse()?, month.parse()?, day.parse()?).unwrap())
    } else {
        eprintln!("No valid date found at the start of the string.");
        Err(anyhow!("Can't parse {}", input))
    }
}

pub fn read_ljson_for_merge(path: &str) -> anyhow::Result<HashMap<String, Output>> {
    let file = File::open(path).map_err(|e| {
        eprintln!("[read_ljson_for_merge]::Error = {} | path = {}", e, path);
        anyhow!(e)
    })?;
    let mut output: HashMap<String, Output> = HashMap::with_capacity(50_000_000);
    let reader = io::BufReader::new(file);
    for (index, line) in reader.lines().enumerate() {
        if index % 100_000 == 0 {
            println!(
                "[read_ljson_for_merge][{}][{}]::index = {}",
                path,
                Utc::now(),
                index
            );
        }
        let line = line?;
        let json: Output = serde_json::from_str(&line).map_err(|e| {
            eprintln!("[read_ljson_for_merge]::Error = {} | line = {}", e, line);
            anyhow!(e)
        })?;
        output.insert(json.id.clone(), json);
    }
    Ok(output)
}

pub fn read_ljson_for_merge2(path: &str) -> anyhow::Result<HashMap<String, Output>> {
    let file = File::open(path).map_err(|e| {
        eprintln!("[read_ljson_for_merge]::Error = {} | path = {}", e, path);
        anyhow!(e)
    })?;
    let mut output: HashMap<String, Output> = HashMap::with_capacity(50_000_000);
    let reader = io::BufReader::new(file);
    for (index, line) in reader.lines().enumerate() {
        if index % 100_000 == 0 {
            println!(
                "[read_ljson_for_merge][{}][{}]::index = {}",
                path,
                Utc::now(),
                index
            );
        }
        let line = line?;
        if line.is_empty() {
            continue;
        }
        let json: Output2 = serde_json::from_str(&line).map_err(|e| {
            eprintln!("[read_ljson_for_merge]::Error = {} | line = {}", e, line);
            anyhow!(e)
        })?;
        output.insert(json.origin_id.clone(), json.into());
    }
    Ok(output)
}

pub fn read_lson(path: &str) -> anyhow::Result<Vec<Raw>> {
    let file = File::open(path).map_err(|e| {
        eprintln!("[read_lson]::Error = {} | path = {}", e, path);
        anyhow!(e)
    })?;
    let reader = io::BufReader::new(file);
    let mut raws: Vec<Raw> = Vec::with_capacity(1_000_000);
    for (index, line) in reader.lines().enumerate() {
        if index % 1000 == 0 {
            println!("[read_lson][{}][{}]::index = {}", path, Utc::now(), index);
        }
        let line = line?;
        if line.is_empty() {
            continue;
        }
        let json: Raw = serde_json::from_str(&line)?;
        raws.push(json);
    }
    Ok(raws)
}

pub fn write_to_file_ljson<T>(records: Vec<T>, path: &str)
where
    T: Serialize,
{
    let mut file =
        File::create(path).unwrap_or_else(|_| panic!("[write_to_file_ljson]::Error {}", path));
    let string_records: Vec<String> = records
        .iter()
        .filter_map(|i| match serde_json::to_string(&i) {
            Ok(v) => Some(v),
            Err(_) => None,
        })
        .collect();

    let s = string_records.join("\n");
    write!(file, "{}", s)
        .unwrap_or_else(|_| panic!("[write_to_file_ljson]::Error write! {}", path));
}

pub fn write_to_csv_file<T>(records: Vec<T>, path: &str)
where
    T: Serialize,
{
    let mut wtr = csv::Writer::from_path(path).unwrap();
    for record in records {
        wtr.serialize(record).unwrap();
    }
    wtr.flush().unwrap();
}

pub fn query_builder(records: &[Record], date: NaiveDate) -> String {
    let vec: Vec<String> = records
        .iter()
        .map(|i| format!("('{}', '{}')", i.user_name, i.origin_id))
        .collect();
    let vec_str = vec.join(",");
    format!(
        r#"
            SELECT ?fields
            FROM data_sinks_clickhouse
            WHERE event_date = '{date}'
            AND (user_name, origin_id) in ({vec_str})
    "#
    )
}

pub async fn process_chunk(
    clickhouse_client: Arc<Client>,
    date: NaiveDate,
    records: &[Record],
) -> anyhow::Result<Vec<DataSinkClickHouse>> {
    let query_str = query_builder(records, date);
    let data = clickhouse_client
        .query(&query_str)
        .fetch_all::<DataSinkClickHouse>()
        .await
        .map_err(|e| {
            eprintln!("process_chunk {}", e);
            anyhow!(e.to_string())
        })?;
    Ok(data)
}

pub fn write_chunk(records: Vec<DataSinkClickHouse>, index: u64, date: &NaiveDate) {
    let mut wtr = csv::Writer::from_path(format!("./CSV_OUTPUT/{}_{}.csv", date, index)).unwrap();
    for record in records {
        wtr.serialize(record).unwrap();
    }
    wtr.flush().unwrap();
}

pub fn is_exists(path: &str) -> bool {
    fs::metadata(path).is_ok()
}
