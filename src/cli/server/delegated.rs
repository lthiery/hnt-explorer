use super::super::delegated;
use super::*;

#[derive(Debug)]
pub struct Memory {
    data: HashMap<i64, Arc<delegated::DelegatedData>>,
    latest_data: Arc<delegated::DelegatedData>,
}

impl Memory {
    pub async fn new(rpc_client: &Arc<RpcClient>) -> Result<Memory> {
        let latest_data = Arc::new(Self::pull_latest_data(rpc_client).await?);
        let mut data = HashMap::new();
        data.insert(latest_data.timestamp, latest_data.clone());
        Ok(Memory { data, latest_data })
    }

    async fn pull_latest_data(rpc_client: &Arc<RpcClient>) -> Result<delegated::DelegatedData> {
        let mut latest_data = delegated::get_data(rpc_client).await?;
        latest_data.scale_down();
        Ok(latest_data)
    }

    async fn update_data(&mut self, latest_data: delegated::DelegatedData) -> Result {
        use chrono::Utc;
        let latest_data = Arc::new(latest_data);
        self.latest_data = latest_data.clone();

        // start a new Hashmap
        let mut data = HashMap::new();
        data.insert(latest_data.timestamp, latest_data.clone());

        // Only keep data that is less than 16 minutes old
        let current_time = Utc::now().timestamp();
        for (key, value) in &self.data {
            if value.timestamp < current_time + 16 * 60 * 3 {
                data.insert(*key, value.clone());
            }
        }
        self.data = data;
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
    pub positions: Vec<delegated::PositionSaved>,
    pub positions_total_len: usize,
}

pub async fn delegated_stakes(
    Extension(memory): Extension<Arc<Mutex<Memory>>>,
    query: Query<QueryParams>,
) -> HandlerResult {
    const DEFAULT_LIMIT: usize = 500;
    let query = query.0;
    let data = {
        let memory = memory.lock().await;
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
    let max_data = data.positions.len() - start;
    let limit = query.limit.map_or(DEFAULT_LIMIT, |limit| {
        limit.min(DEFAULT_LIMIT).min(max_data)
    });

    let mut positions = Vec::with_capacity(limit);
    positions.resize(limit, delegated::PositionSaved::default());
    positions.clone_from_slice(&data.positions[start..start + limit]);

    let data = DelegatedData {
        positions_total_len: data.positions_total_len,
        positions,
        timestamp: data.timestamp,
    };

    Ok(response::Json(json!(data)))
}

#[derive(Debug, Deserialize)]
pub struct QueryParamsMetadata {
    timestamp: Option<i64>,
}

pub async fn delegated_stakes_metadata(
    Extension(memory): Extension<Arc<Mutex<Memory>>>,
    query: Query<QueryParamsMetadata>,
) -> HandlerResult {
    const DEFAULT_LIMIT: usize = 500;
    let query = query.0;
    let data = {
        let memory = memory.lock().await;
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
    };

    Ok(response::Json(json!(data)))
}

#[derive(Default, Debug, serde::Serialize)]
pub struct Metadata {
    pub timestamp: i64,
    pub network: delegated::Data,
    pub mobile: delegated::Data,
    pub iot: delegated::Data,
}

pub async fn get_delegated_stakes(
    rpc_client: Arc<RpcClient>,
    memory: Arc<Mutex<Memory>>,
) -> Result {
    loop {
        time::sleep(time::Duration::from_secs(60 * 5)).await;
        let latest_data = Memory::pull_latest_data(&rpc_client).await?;
        {
            let mut memory = memory.lock().await;
            memory.update_data(latest_data).await?;
        }
    }
}
