use super::*;
use crate::cli::positions;

#[derive(Debug, Deserialize)]
pub struct StatsParams {
    timestamp: Option<i64>,
}

pub async fn vehnt_positions_stats(
    Extension(memory): Extension<Arc<Mutex<Option<Memory>>>>,
    query: Query<StatsParams>,
) -> HandlerResult {
    const DEFAULT_LIMIT: usize = 500;
    let query = query.0;
    let data = {
        let memory = memory.lock().await;
        if memory.is_none() {
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                DATA_NOT_INIT_MSG.to_string(),
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

    #[derive(Default, Debug, serde::Serialize)]
    pub struct Stats {
        pub timestamp: i64,
        pub network: positions::Data,
        pub mobile: positions::Data,
        pub iot: positions::Data,
        pub undelegated: positions::Data,
    }

    let data = Stats {
        timestamp: data.vehnt.timestamp,
        network: data.vehnt.network,
        mobile: data.vehnt.mobile,
        iot: data.vehnt.iot,
        undelegated: data.vehnt.undelegated,
    };

    Ok(response::Json(json!(data)))
}
