//! Compute points from the Mandelbrot set, taking the point to compute from
//! an AWS SQS queue and storing the result into an AWS S3 bucket.
mod compute_point;
mod http;
mod types;

use {
    crate::http::{bad_request, not_found},
    ::http::{status::StatusCode, HeaderMap, HeaderValue},
    aws_lambda_events::event::lambda_function_urls::{LambdaFunctionUrlRequest, LambdaFunctionUrlResponse},
    compute_point::handle_compute_point_request,
    lambda_runtime::{service_fn, tracing::init_default_subscriber, LambdaEvent},
    types::BoxError,
};

#[tokio::main]
#[allow(clippy::needless_return)] // Clippy is flagging this strangely.
async fn main() -> Result<(), BoxError> {
    init_default_subscriber();
    let func = service_fn(run);
    lambda_runtime::run(func).await?;
    Ok(())
}

async fn run(event: LambdaEvent<LambdaFunctionUrlRequest>) -> Result<LambdaFunctionUrlResponse, BoxError> {
    let request_id = event.context.request_id.as_str();
    let Some(raw_path) = event.payload.raw_path.as_deref() else {
        return Ok(bad_request("No path found in request", request_id));
    };

    match raw_path {
        "/point" => handle_compute_point_request(event).await,
        "/health" => handle_health_request(event).await,
        _ => Ok(not_found(request_id)),
    }
}

async fn handle_health_request(
    _event: LambdaEvent<LambdaFunctionUrlRequest>,
) -> Result<LambdaFunctionUrlResponse, BoxError> {
    let mut headers = HeaderMap::new();
    headers.insert("Content-Type", HeaderValue::from_static("text/plain"));
    let cookies = vec![];
    Ok(LambdaFunctionUrlResponse {
        status_code: StatusCode::OK.as_u16() as i64,
        body: Some("OK".to_string()),
        headers,
        cookies,
        is_base64_encoded: false,
    })
}
