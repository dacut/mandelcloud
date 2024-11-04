//! Compute points from the Mandelbrot set, taking the point to compute from
//! an AWS SQS queue and storing the result into an AWS S3 bucket.
mod types;

use {
    aws_lambda_events::event::lambda_function_urls::{LambdaFunctionUrlRequest, LambdaFunctionUrlResponse},
    aws_sdk_s3::{operation::get_object::GetObjectError, primitives::ByteStream},
    aws_smithy_runtime_api::client::result::SdkError,
        lambda_runtime::{service_fn, tracing::init_default_subscriber, LambdaEvent},
    log::error,
    http::{HeaderMap, HeaderValue, status::StatusCode},
    rug::{Complex, Float},
    std::error::Error as StdError,
    types::{ComputePointResult, ComputePointResponse, SerComplex, SerFloat, Error, ErrorType, ErrorResponse},
};

type BoxError = Box<dyn StdError + Send + Sync + 'static>;

/// The S3 bucket and prefix to use for storing computed points.
#[derive(Debug)]
pub struct S3Config {
    bucket: String,
    prefix: String,
}

impl S3Config {
    /// Returns an S3 config from the Lambda environment.
    /// 
    /// # Panics
    /// If the S3_BUCKET or S3_PREFIX environment variables are not set, this function will panic.
    pub fn from_env() -> Self {
        let bucket = std::env::var("S3_BUCKET").expect("S3_BUCKET environment variable not set");
        let prefix = std::env::var("S3_PREFIX").expect("S3_PREFIX environment variable not set");
        Self { bucket, prefix }
    }
}

#[tokio::main]
#[allow(clippy::needless_return)] // Clippy is flagging this strangely.
async fn main() -> Result<(), BoxError> {
    init_default_subscriber();
    let func = service_fn(run);
    lambda_runtime::run(func).await?;
    Ok(())
}

/// Returns a LambdaFunctionUrlResponse of 400: BadRequest.
fn bad_request(message: &str, request_id: &str) -> LambdaFunctionUrlResponse {
    http_error(StatusCode::BAD_REQUEST, "BadRequest", message, request_id)
}

/// Returns a LambdaFunctionUrlResponse of 404: NotFound.
fn not_found(request_id: &str) -> LambdaFunctionUrlResponse {
    http_error(StatusCode::NOT_FOUND, "NotFound", "Not found", request_id)
}

// Returns an LambdaFunctionUrlResponse error with the given status code, error code, and message.
fn http_error(status_code: StatusCode, error_code: &str, message: &str, request_id: &str) -> LambdaFunctionUrlResponse {
    let mut headers = HeaderMap::new();
    headers.insert("Content-Type", HeaderValue::from_static("application/json"));

    let e = ErrorResponse {
        error: Error {
            error_type: ErrorType::Sender,
            code: Some(error_code.into()),
            message: Some(message.into()),
        },
        request_id: request_id.to_string(),
    };

    let body = serde_json::to_string(&e).unwrap();

    LambdaFunctionUrlResponse {
        status_code: status_code.as_u16() as i64,
        body: Some(body),
        headers,
        cookies: vec![],
        is_base64_encoded: false,
    }
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

async fn handle_health_request(_event: LambdaEvent<LambdaFunctionUrlRequest>) -> Result<LambdaFunctionUrlResponse, BoxError> {
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

async fn handle_compute_point_request(event: LambdaEvent<LambdaFunctionUrlRequest>) -> Result<LambdaFunctionUrlResponse, BoxError> {
    let request_id = event.context.request_id.as_str();
    let Some(x) = event.payload.query_string_parameters.get("x").map(String::as_str) else {
        return Ok(bad_request("Missing x parameter", request_id));
    };
    let Some(y) = event.payload.query_string_parameters.get("y").map(String::as_str) else {
        return Ok(bad_request("Missing y parameter", request_id));
    };
    let Some(iterations) = event.payload.query_string_parameters.get("iterations").map(String::as_str) else {
        return Ok(bad_request("Missing iterations parameter", request_id));
    };
    let Some(x_prec) = event.payload.query_string_parameters.get("x_prec").map(String::as_str) else {
        return Ok(bad_request("Missing x_prec parameter", request_id));
    };
    let Some(y_prec) = event.payload.query_string_parameters.get("y_prec").map(String::as_str) else {
        return Ok(bad_request("Missing y_prec parameter", request_id));
    };

    let Ok(iterations) = str::parse(iterations) else {
        return Ok(bad_request("Invalid iterations parameter", request_id));
    };

    let Ok(x_prec) = str::parse(x_prec) else {
        return Ok(bad_request("Invalid x_prec parameter", request_id));
    };

    let Ok(y_prec) = str::parse(y_prec) else {
        return Ok(bad_request("Invalid y_prec parameter", request_id));
    };

    let computed_point_result = compute_point(x.to_string(), y.to_string(), x_prec, y_prec, iterations).await?;
    let response = ComputePointResponse {
        computed_point_result,
        request_id: request_id.to_string(),
    };
    let body = serde_json::to_string(&response).unwrap();
    let headers = HeaderMap::new();
    let cookies = vec![];
    Ok(LambdaFunctionUrlResponse {
        status_code: StatusCode::OK.as_u16() as i64,
        body: Some(body),
        headers,
        cookies,
        is_base64_encoded: false,
    })
}

async fn compute_point(x: String, y: String, x_prec: u64, y_prec: u64,iterations: u32) -> Result<ComputePointResult, BoxError> {
    let s3_config = S3Config::from_env();
    let s3_bucket = s3_config.bucket.as_str();
    let s3_prefix = s3_config.prefix.as_str();
    let s3_key = format!("{}{}_{}/{}_{}/{}.json", s3_prefix, x, x_prec, y, y_prec, iterations);
    let x = Float::parse(&x).unwrap();
    let y = Float::parse(&y).unwrap();
    let c = Complex::with_val_64((x_prec, y_prec), (x, y));
    let mut z = c.clone();
    let limit = Float::new(53) + 4u32;

    let config = aws_config::load_from_env().await;
    let s3_client = aws_sdk_s3::Client::new(&config);

    // Do we have a result for this already? If so, just return it.
    match s3_client.get_object().bucket(s3_bucket).key(&s3_key).send().await {
        Ok(response) => {
            let bytes = response.body.collect().await?.into_bytes();
            let response = serde_json::from_slice::<ComputePointResult>(&bytes)?;
            return Ok(response);
        }
        Err(e) => {
            if let SdkError::ServiceError(s3_error) = e {
                let s3_error = s3_error.into_err();
                if !matches!(s3_error, GetObjectError::NoSuchKey(_)) {
                    // This wasn't expected; log the error.
                    error!("Unexpected error retrieving s3://{s3_bucket}/{s3_prefix}: {s3_error}");
                }
            } else {
                // This wasn't expected; log the error.
                error!("Unexpected error retrieving s3://{s3_bucket}/{s3_prefix}: {e}");
            }
        }
    }

    let mut iteration = 0;
    let mut escape_iteration = None;
    while iteration < iterations {
        z = z.square() + &c;

        let bound = Float::with_val(53, z.norm_ref());
        if bound > limit {
            escape_iteration = Some(iteration);
            break;
        }

        iteration += 1;
    }

    let final_value = SerComplex {
        real: SerFloat {
            value: z.real().to_string(),
            prec: 53,
        },
        imag: SerFloat {
            value: z.imag().to_string(),
            prec: 53,
        },
    };

    let computed_point = ComputePointResult {
        c: SerComplex::from(&c),
        escape_iteration,
        computed_iterations: iteration,
        final_value,
    };

    // Store the result in S3.
    let body = serde_json::to_string(&computed_point).unwrap();
    let body_stream = ByteStream::from(body.into_bytes());
    if let Err(e) = s3_client.put_object().bucket(s3_bucket).key(&s3_key).body(body_stream).send().await {
        error!("Error storing result in s3://{s3_bucket}/{s3_key}: {e}");
    }

    Ok(computed_point)
}
