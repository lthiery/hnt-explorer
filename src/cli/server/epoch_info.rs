use super::super::epoch_info;
use super::*;
use chrono::{Datelike, Utc};

#[derive(Debug)]
pub struct Memory {
    latest_data: Arc<Vec<epoch_info::EpochSummary>>,
}

impl Memory {
    pub async fn new(rpc_client: &Arc<RpcClient>) -> Result<Memory> {
        let latest_data = Arc::new(Self::pull_latest_data(rpc_client).await?);
        Ok(Memory { latest_data })
    }

    async fn pull_latest_data(
        rpc_client: &Arc<RpcClient>,
    ) -> Result<Vec<epoch_info::EpochSummary>> {
        let mut latest_data = epoch_info::get_epoch_summaries(rpc_client).await?;
        latest_data.iter_mut().for_each(|x| x.scale_down());
        Ok(latest_data)
    }

    async fn update_data(&mut self, latest_data: Vec<epoch_info::EpochSummary>) -> Result {
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
            time::sleep(time::Duration::from_secs(60 * 2)).await;
            let latest_data = Memory::pull_latest_data(&rpc_client).await?;
            {
                let mut memory = memory.lock().await;
                memory.update_data(latest_data).await?;
            }
        }
    }
}

pub async fn get(Extension(memory): Extension<Arc<Mutex<Memory>>>) -> HandlerResult {
    let data = {
        let memory = memory.lock().await;
        memory.latest_data.clone()
    };
    Ok(response::Json(json!(data)))
}
