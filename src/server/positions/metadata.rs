use super::*;

#[derive(Debug, Deserialize)]
pub struct StatsParams {
    timestamp: Option<i64>,
}

pub async fn vehnt_positions_metadata(
    Extension(memory): Extension<Arc<Mutex<Option<Memory>>>>,
    query: Query<StatsParams>,
) -> HandlerResult {
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

    Ok(response::Json(json!(data.stats)))
}
