use super::*;
use axum::body::{Body, Bytes};

pub async fn server_latest_delegated_positions_as_csv(
    Extension(memory): Extension<Arc<Mutex<Option<Memory>>>>,
) -> impl IntoResponse {
    let memory_mutex = memory.lock().await;
    if memory_mutex.is_none() {
        return Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            DATA_NOT_INIT_MSG.to_string(),
        ));
    }
    let memory = memory_mutex.as_ref().unwrap();
    let latest_file = memory.latest_delegated_positions_file();
    let mime_type = mime_guess::from_path(&latest_file).first_or_text_plain();

    match File::open(&latest_file).await {
        Err(_) => Ok(Response::builder()
            .status(StatusCode::NOT_FOUND)
            .body(Body::empty())
            .unwrap()),
        Ok(mut file) => {
            let mut contents = vec![];
            match file.read_to_end(&mut contents).await {
                Err(_) => Ok(Response::builder()
                    .status(StatusCode::NOT_FOUND)
                    .body(Body::empty())
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
                        .body(Body::from(Bytes::from(contents)))
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
            DATA_NOT_INIT_MSG.to_string(),
        ));
    }
    let memory = memory_mutex.as_ref().unwrap();
    let latest_file = memory.latest_positions_file();
    let mime_type = mime_guess::from_path(&latest_file).first_or_text_plain();

    match File::open(&latest_file).await {
        Err(_) => Ok(Response::builder()
            .status(StatusCode::NOT_FOUND)
            .body(Body::empty())
            .unwrap()),
        Ok(mut file) => {
            let mut contents = vec![];
            match file.read_to_end(&mut contents).await {
                Err(_) => Ok(Response::builder()
                    .status(StatusCode::NOT_FOUND)
                    .body(Body::empty())
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
                        .body(Body::from(Bytes::from(contents)))
                        .unwrap())
                }
            }
        }
    }
}
