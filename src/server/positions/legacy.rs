use super::*;
use crate::cli::positions;

pub async fn delegated_stakes(
    Extension(memory): Extension<Arc<Mutex<Option<Memory>>>>,
    query: Query<PositionParams>,
) -> HandlerResult {
    const DEFAULT_LIMIT: usize = 500;
    let query = query.0;
    let data = {
        let memory = memory.lock().await;
        if memory.is_none() {
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                "DATA_NOT_INIT_MSGData not initialized".to_string(),
            ));
        }
        let memory = memory.as_ref().unwrap();
        if let Some(timestamp) = query.timestamp {
            if let Some(data) = memory.data.get(&timestamp) {
                Ok(data.vehnt.clone())
            } else {
                Err((
                    StatusCode::NOT_FOUND,
                    format!("Data not found for timestamp = {timestamp}"),
                ))
            }
        } else {
            Ok(memory.latest_data.vehnt.clone())
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

    Ok(response::Json(json!(data)).into())
}
