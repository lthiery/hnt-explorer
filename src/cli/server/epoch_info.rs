use super::super::epoch_info;
use super::*;
use crate::cli::epoch_info::EpochSummary;
use chrono::{Datelike, Utc};

#[derive(Debug)]
pub struct Memory {
    pub latest_data: Arc<Vec<EpochSummary>>,
}

impl Memory {
    pub async fn new(rpc_client: &Arc<RpcClient>) -> Result<Memory> {
        let latest_data = Arc::new(Self::pull_latest_data(rpc_client).await?);
        Ok(Memory { latest_data })
    }

    async fn pull_latest_data(rpc_client: &Arc<RpcClient>) -> Result<Vec<EpochSummary>> {
        let mut latest_data = epoch_info::get_epoch_summaries(rpc_client).await?;
        latest_data.iter_mut().for_each(|x| x.scale_down());
        Ok(latest_data)
    }

    async fn update_data(&mut self, latest_data: Vec<EpochSummary>) -> Result {
        self.latest_data = Arc::new(latest_data);
        Ok(())
    }
}

/// Only updates the epoch info when the date rolls over
pub async fn get_epoch_info(rpc_client: Arc<RpcClient>, memory: Arc<Mutex<Memory>>) -> Result {
    let mut last_pull_day = Utc::now().day();
    loop {
        time::sleep(time::Duration::from_secs(60 * 5)).await;
        let day = Utc::now().day();
        if day != last_pull_day {
            last_pull_day = day;
            let mut latest_data = Memory::pull_latest_data(&rpc_client).await;
            while latest_data.is_err() {
                time::sleep(time::Duration::from_secs(60)).await;
                latest_data = Memory::pull_latest_data(&rpc_client).await;
            }
            let mut memory = memory.lock().await;
            memory.update_data(latest_data.unwrap()).await?;
        }
    }
}

pub async fn get(
    Extension(memory): Extension<Arc<Mutex<Memory>>>,
    Extension(stakes_memory): Extension<Arc<Mutex<Option<positions::Memory>>>>,
) -> HandlerResult {
    let (mobile_vehnt, iot_vehnt, ts) = {
        let stakes_memory = stakes_memory.lock().await;
        if stakes_memory.is_none() {
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                DATA_NOT_INIT_MSG.to_string(),
            ));
        }
        let stakes_memory = stakes_memory.as_ref().unwrap();
        (
            stakes_memory.latest_data.vehnt.mobile.total.vehnt,
            stakes_memory.latest_data.vehnt.iot.total.vehnt,
            stakes_memory.latest_data.vehnt.timestamp,
        )
    };
    let mut data: Vec<EpochSummary> = {
        // we do a deep copy because we will be mutating the data
        let memory = memory.lock().await;
        memory.latest_data.to_vec()
    };

    // we take most recent delegated_stakes data and make a future epoch out of it
    let last_epoch = data[data.len() - 1].epoch + 1;
    let current_stats =
        EpochSummary::from_partial_data(last_epoch, mobile_vehnt, iot_vehnt, ts).unwrap();
    data.push(current_stats);

    Ok(response::Json(json!(data)))
}
