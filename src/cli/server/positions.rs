use super::super::positions;
use super::*;
use crate::types::SubDao;
use axum::{
    body::{self, Empty, Full},
    extract::Path,
    http::{header, HeaderValue},
    response::{IntoResponse, Response},
};
use solana_sdk::pubkey::Pubkey;
use std::ops::DerefMut;
use std::str::FromStr;
use tokio::{fs::File, io::AsyncReadExt};

#[derive(Debug)]
pub struct Memory {
    data: HashMap<i64, Arc<positions::PositionData>>,
    pub position: HashMap<Pubkey, positions::Position>,
    pub latest_data: Arc<positions::PositionData>,
    pub positions_by_owner: HashMap<Pubkey, Vec<Pubkey>>,
}

impl Memory {
    fn latest_delegated_positions_file(&self) -> String {
        format!("./delegated_positions_{}.csv", self.latest_data.timestamp)
    }

    fn latest_positions_file(&self) -> String {
        format!("./positions_{}.csv", self.latest_data.timestamp)
    }

    #[allow(unused)]
    pub async fn new(
        latest_data: positions::PositionData
    ) -> Result<Memory> {
        let mut memory = Self {
            data: HashMap::new(),
            position: HashMap::new(),
            latest_data: Arc::new(positions::PositionData::new()),
            positions_by_owner: HashMap::new(),
        };
        memory.update_data(latest_data).await?;
        Ok(memory)
    }

    async fn remove_csv(&self, path: String) -> Result {
        tokio::fs::remove_file(path).await?;
        Ok(())
    }

    fn write_latest_to_csv(&self) -> Result {
        #[derive(serde::Serialize)]
        struct Position<'a> {
            pub position_key: &'a str,
            pub owner: &'a str,
            pub hnt_amount: u64,
            pub start_ts: i64,
            pub genesis_end_ts: i64,
            pub end_ts: i64,
            pub duration_s: i64,
            pub vehnt: u128,
            pub lockup_type: &'a positions::LockupType,
            pub delegated_position_key: Option<&'a str>,
            pub delegated_sub_dao: Option<SubDao>,
            pub delagated_last_claimed_epoch: Option<u64>,
            pub delegated_pending_rewards: Option<u64>,
        }

        use csv::Writer;
        let mut position_wtr = Writer::from_path(self.latest_positions_file())?;
        let mut delegated_position_wtr = Writer::from_path(self.latest_delegated_positions_file())?;
        for position in self.latest_data.positions.iter() {
            if let Some(delegated) = &position.delegated {
                position_wtr.serialize(Position {
                    position_key: &position.position_key,
                    owner: &position.owner,
                    hnt_amount: position.hnt_amount,
                    start_ts: position.start_ts,
                    genesis_end_ts: position.genesis_end_ts,
                    end_ts: position.end_ts,
                    duration_s: position.duration_s,
                    vehnt: position.vehnt,
                    lockup_type: &position.lockup_type,
                    delegated_position_key: Some(&delegated.delegated_position_key),
                    delegated_sub_dao: Some(delegated.sub_dao),
                    delagated_last_claimed_epoch: Some(delegated.last_claimed_epoch),
                    delegated_pending_rewards: Some(delegated.pending_rewards),
                })?;
            } else {
                position_wtr.serialize(Position {
                    position_key: &position.position_key,
                    owner: &position.owner,
                    hnt_amount: position.hnt_amount,
                    start_ts: position.start_ts,
                    genesis_end_ts: position.genesis_end_ts,
                    end_ts: position.end_ts,
                    duration_s: position.duration_s,
                    vehnt: position.vehnt,
                    lockup_type: &position.lockup_type,
                    delegated_position_key: None,
                    delegated_sub_dao: None,
                    delagated_last_claimed_epoch: None,
                    delegated_pending_rewards: None,
                })?;
            }
        }
        for position in self.latest_data.delegated_positions.iter() {
            delegated_position_wtr.serialize(position)?;
        }
        Ok(())
    }

    async fn pull_latest_data(
        rpc_client: &Arc<RpcClient>,
        epoch_summaries: Arc<Mutex<epoch_info::Memory>>,
        position_owner_map: &mut HashMap<Pubkey, Pubkey>,
    ) -> Result<positions::PositionData> {
        let epoch_summaries = {
            let lock = epoch_summaries.lock().await;
            lock.latest_data.clone()
        };
        let mut latest_data =
            positions::get_data(rpc_client, epoch_summaries, position_owner_map).await?;
        latest_data.scale_down();
        Ok(latest_data)
    }

    async fn update_data(&mut self, latest_data: positions::PositionData) -> Result {
        print!("Updating data...");
        use chrono::Utc;
        let previous_file = self.latest_delegated_positions_file();
        let latest_data = Arc::new(latest_data);
        self.latest_data = latest_data.clone();

        // organize into map of positions pubkey to full position data
        self.position = latest_data
            .positions
            .iter()
            .map(|p| (Pubkey::from_str(&p.position_key).unwrap(), p.clone()))
            .collect();

        // organize into map of owner pubkey to [position pubkey]
        let mut positions_by_owner: HashMap<Pubkey, Vec<Pubkey>> = HashMap::new();
        for position in latest_data.positions.iter() {
            let owner = Pubkey::from_str(&position.owner)?;
            let position = Pubkey::from_str(&position.position_key)?;
            if let Some(entry) = positions_by_owner.get_mut(&owner) {
                entry.push(position);
            } else {
                positions_by_owner.insert(owner, vec![position]);
            }
        }
        self.positions_by_owner = positions_by_owner;

        // start a new Hashmap of all cached positions
        let mut data = HashMap::new();
        data.insert(latest_data.timestamp, latest_data.clone());

        // Only keep data that is less than 16 minutes old
        let current_time = Utc::now().timestamp();
        for (key, value) in &self.data {
            if value.timestamp > current_time - 60 * 16 {
                data.insert(*key, value.clone());
            }
        }
        println!(" History contains {} entries", data.len());
        self.data = data;
        self.write_latest_to_csv()?;
        if let Err(e) = self.remove_csv(previous_file).await {
            println!(
                "Failed to remove previous csv: {}. This is expected at first boot.",
                e
            );
        }
        Ok(())
    }
}

#[derive(Debug, Deserialize)]
pub struct QueryParams {
    timestamp: Option<i64>,
    start: Option<usize>,
    limit: Option<usize>,
}

#[derive(Default, Debug, serde::Serialize)]
pub struct DelegatedData {
    pub timestamp: i64,
    pub positions: Vec<positions::Position>,
    pub positions_total_len: usize,
}

pub async fn delegated_stakes(
    Extension(memory): Extension<Arc<Mutex<Option<Memory>>>>,
    query: Query<QueryParams>,
) -> HandlerResult {
    const DEFAULT_LIMIT: usize = 500;
    let query = query.0;
    let data = {
        let memory = memory.lock().await;
        if memory.is_none() {
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                "Data not initialized".to_string(),
            ));
        }
        let memory = memory.as_ref().unwrap();
        if let Some(timestamp) = query.timestamp {
            if let Some(data) = memory.data.get(&timestamp) {
                Ok(data.clone())
            } else {
                Err((
                    StatusCode::NOT_FOUND,
                    format!("Data not found for timestamp = {timestamp}"),
                ))
            }
        } else {
            Ok(memory.latest_data.clone())
        }
    }?;

    let start = query.start.map_or(0, |start| start);
    if start > data.delegated_positions.len() {
        return Err((
            StatusCode::BAD_REQUEST,
            format!(
                "Start index {start} is greater than the total number of positions {total}",
                total = data.delegated_positions.len()
            ),
        ));
    }

    let max_data = data.delegated_positions.len() - start;
    let limit = query.limit.map_or(DEFAULT_LIMIT, |limit| {
        limit.min(DEFAULT_LIMIT).min(max_data)
    });

    let mut delegated_positions = Vec::with_capacity(limit);
    delegated_positions.resize(limit, positions::PositionLegacy::default());
    delegated_positions.clone_from_slice(&data.delegated_positions[start..start + limit]);

    #[derive(Default, Debug, serde::Serialize, serde::Deserialize)]
    pub struct LegacyData {
        pub timestamp: i64,
        pub delegated_positions: Vec<positions::PositionLegacy>,
        pub positions_total_len: usize,
    }

    let data = LegacyData {
        positions_total_len: data.delegated_positions.len(),
        delegated_positions,
        timestamp: data.timestamp,
    };

    Ok(response::Json(json!(data)))
}

pub async fn positions(
    Extension(memory): Extension<Arc<Mutex<Option<Memory>>>>,
    query: Query<QueryParams>,
) -> HandlerResult {
    const DEFAULT_LIMIT: usize = 500;
    let query = query.0;
    let data = {
        let memory = memory.lock().await;
        if memory.is_none() {
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                "Data not initialized".to_string(),
            ));
        }
        let memory = memory.as_ref().unwrap();
        if let Some(timestamp) = query.timestamp {
            if let Some(data) = memory.data.get(&timestamp) {
                Ok(data.clone())
            } else {
                Err((
                    StatusCode::NOT_FOUND,
                    format!("Data not found for timestamp = {timestamp}"),
                ))
            }
        } else {
            Ok(memory.latest_data.clone())
        }
    }?;

    let start = query.start.map_or(0, |start| start);
    if start > data.positions.len() {
        return Err((
            StatusCode::BAD_REQUEST,
            format!(
                "Start index {start} is greater than the total number of positions {total}",
                total = data.positions.len()
            ),
        ));
    }

    let max_data = data.positions.len() - start;
    println!("max_data: {}", max_data);
    let limit = query.limit.map_or(DEFAULT_LIMIT, |limit| {
        limit.min(DEFAULT_LIMIT).min(max_data)
    });
    println!("limit: {}", limit);
    println!("start: {}", start);

    let mut positions = Vec::with_capacity(limit);
    positions.resize(limit, positions::Position::default());
    positions.clone_from_slice(&data.positions[start..start + limit]);

    let data = DelegatedData {
        positions_total_len: data.positions_total_len,
        positions,
        timestamp: data.timestamp,
    };

    Ok(response::Json(json!(data)))
}

pub async fn position(
    Extension(memory): Extension<Arc<Mutex<Option<Memory>>>>,
    Path(position): Path<String>,
) -> HandlerResult {
    if let Ok(pubkey) = Pubkey::from_str(&position) {
        let memory = memory.lock().await;
        if memory.is_none() {
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                "Data not initialized".to_string(),
            ));
        }
        let memory = memory.as_ref().unwrap();
        if let Some(position) = memory.position.get(&pubkey) {
            Ok(response::Json(json!(position)))
        } else {
            Err((
                StatusCode::NOT_FOUND,
                format!("\"{position}\" is not a known position from the voter stake registry"),
            ))
        }
    } else {
        Err((
            StatusCode::BAD_REQUEST,
            format!("\"{position}\" is not a valid base58 encoded Solana pubkey"),
        ))
    }
}

#[derive(Debug, Deserialize)]
pub struct QueryParamsMetadata {
    timestamp: Option<i64>,
}

pub async fn positions_metadata(
    Extension(memory): Extension<Arc<Mutex<Option<Memory>>>>,
    query: Query<QueryParamsMetadata>,
) -> HandlerResult {
    const DEFAULT_LIMIT: usize = 500;
    let query = query.0;
    let data = {
        let memory = memory.lock().await;
        if memory.is_none() {
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                "Data not initialized".to_string(),
            ));
        }
        let memory = memory.as_ref().unwrap();
        if let Some(timestamp) = query.timestamp {
            if let Some(data) = memory.data.get(&timestamp) {
                Ok(data.clone())
            } else {
                Err((
                    StatusCode::NOT_FOUND,
                    format!("Data not found for timestamp = {timestamp}"),
                ))
            }
        } else {
            Ok(memory.latest_data.clone())
        }
    }?;

    let data = Metadata {
        timestamp: data.timestamp,
        network: data.network,
        mobile: data.mobile,
        iot: data.iot,
        undelegated: data.undelegated,
    };

    Ok(response::Json(json!(data)))
}

#[derive(Default, Debug, serde::Serialize)]
pub struct Metadata {
    pub timestamp: i64,
    pub network: positions::Data,
    pub mobile: positions::Data,
    pub iot: positions::Data,
    pub undelegated: positions::Data,
}

pub async fn get_positions(
    rpc_client: Arc<RpcClient>,
    memory: Arc<Mutex<Option<Memory>>>,
    epoch_memory: Arc<Mutex<epoch_info::Memory>>,
) -> Result {
    let mut position_owner_map = HashMap::new();
    loop {
        println!("Pulling latest data");
        let mut latest_data =
            Memory::pull_latest_data(&rpc_client, epoch_memory.clone(), &mut position_owner_map)
                .await;
        // if the first pull fails, keep trying until it succeeds
        while latest_data.is_err() {
            latest_data = Memory::pull_latest_data(
                &rpc_client,
                epoch_memory.clone(),
                &mut position_owner_map,
            )
            .await;
        }
        {
            latest_data = Memory::pull_latest_data(
                &rpc_client,
                epoch_memory.clone(),
                &mut position_owner_map,
            )
            .await;
        }
        //safe to unwrap because of result check above
        let latest_data = latest_data.unwrap();
        {
            // acquire the lock and set the memory
            let mut memory = memory.lock().await;
            match memory.deref_mut() {
                None => {
                    *memory = Some(Memory::new(latest_data).await?);
                }

                Some(ref mut memory) => {
                    memory.update_data(latest_data).await?;
                }
            }
        }
        time::sleep(time::Duration::from_secs(60*5)).await;
    }
}

pub async fn server_latest_delegated_positions_as_csv(
    Extension(memory): Extension<Arc<Mutex<Option<Memory>>>>,
) -> impl IntoResponse {
    let memory_mutex = memory.lock().await;
    if memory_mutex.is_none() {
        return Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            "Data not initialized".to_string(),
        ));
    }
    let memory = memory_mutex.as_ref().unwrap();
    let latest_file = memory.latest_delegated_positions_file();
    let mime_type = mime_guess::from_path(&latest_file).first_or_text_plain();

    match File::open(&latest_file).await {
        Err(_) => Ok(Response::builder()
            .status(StatusCode::NOT_FOUND)
            .body(body::boxed(Empty::new()))
            .unwrap()),
        Ok(mut file) => {
            let mut contents = vec![];
            match file.read_to_end(&mut contents).await {
                Err(_) => Ok(Response::builder()
                    .status(StatusCode::NOT_FOUND)
                    .body(body::boxed(Empty::new()))
                    .unwrap()),
                Ok(_) => {
                    drop(memory_mutex);
                    Ok(Response::builder()
                        .status(StatusCode::OK)
                        .header(
                            header::CONTENT_TYPE,
                            HeaderValue::from_str(mime_type.as_ref()).unwrap(),
                        )
                        .header(
                            header::CONTENT_DISPOSITION,
                            HeaderValue::from_str(&format!(
                                "attachment; filename=\"{latest_file}\""
                            ))
                            .unwrap(),
                        )
                        .body(body::boxed(Full::from(contents)))
                        .unwrap())
                }
            }
        }
    }
}

pub async fn server_latest_positions_as_csv(
    Extension(memory): Extension<Arc<Mutex<Option<Memory>>>>,
) -> impl IntoResponse {
    let memory_mutex = memory.lock().await;
    if memory_mutex.is_none() {
        return Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            "Data not initialized".to_string(),
        ));
    }
    let memory = memory_mutex.as_ref().unwrap();
    let latest_file = memory.latest_positions_file();
    let mime_type = mime_guess::from_path(&latest_file).first_or_text_plain();

    match File::open(&latest_file).await {
        Err(_) => Ok(Response::builder()
            .status(StatusCode::NOT_FOUND)
            .body(body::boxed(Empty::new()))
            .unwrap()),
        Ok(mut file) => {
            let mut contents = vec![];
            match file.read_to_end(&mut contents).await {
                Err(_) => Ok(Response::builder()
                    .status(StatusCode::NOT_FOUND)
                    .body(body::boxed(Empty::new()))
                    .unwrap()),
                Ok(_) => {
                    drop(memory_mutex);
                    Ok(Response::builder()
                        .status(StatusCode::OK)
                        .header(
                            header::CONTENT_TYPE,
                            HeaderValue::from_str(mime_type.as_ref()).unwrap(),
                        )
                        .header(
                            header::CONTENT_DISPOSITION,
                            HeaderValue::from_str(&format!(
                                "attachment; filename=\"{latest_file}\""
                            ))
                            .unwrap(),
                        )
                        .body(body::boxed(Full::from(contents)))
                        .unwrap())
                }
            }
        }
    }
}
