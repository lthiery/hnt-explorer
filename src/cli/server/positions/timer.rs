use super::*;

pub async fn get_positions(
    rpc_client: Arc<RpcClient>,
    memory: Arc<Mutex<Option<Memory>>>,
    epoch_memory: Arc<Mutex<epoch_info::Memory>>,
) -> Result {
    let mut position_owner_map = PositionOwners::default();
    loop {
        println!("Pulling latest data");
        let mut latest_data =
            Memory::pull_latest_data(&rpc_client, epoch_memory.clone(), &mut position_owner_map)
                .await;
        // if the first pull fails, keep trying until it succeeds
        while let Err(e) = latest_data {
            println!("Error pulling data: {e:?}");
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
        time::sleep(time::Duration::from_secs(60 * 5)).await;
    }
}
