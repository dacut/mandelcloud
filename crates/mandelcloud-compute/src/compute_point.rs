use {
    crate::{
        http::bad_request,
        types::{
            BoxError, ComputePointResponse, ComputePointResult, SerComplex, SerFloat, S3Config,
        },
    },
    aws_lambda_events::event::lambda_function_urls::{LambdaFunctionUrlRequest, LambdaFunctionUrlResponse},
    aws_sdk_s3::{operation::get_object::GetObjectError, primitives::ByteStream},
    aws_smithy_runtime_api::client::result::SdkError,
    http::{status::StatusCode, HeaderMap},
    lambda_runtime::LambdaEvent,
    log::error,
    rug::{Complex, Float},
    sha3::{Digest, Sha3_512},
};

pub(crate) async fn handle_compute_point_request(
    event: LambdaEvent<LambdaFunctionUrlRequest>,
) -> Result<LambdaFunctionUrlResponse, BoxError> {
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

async fn compute_point(
    x: String,
    y: String,
    x_prec: u64,
    y_prec: u64,
    iterations: u32,
) -> Result<ComputePointResult, BoxError> {
    let s3_config = S3Config::from_env();
    let s3_bucket = s3_config.bucket.as_str();
    let s3_prefix = s3_config.points_prefix.as_str();
    let s3_key_raw = format!("x={},y={},x_prec={},y_prec={},iterations={}", x, y, x_prec, y_prec, iterations);
    let mut s3_key_hasher = Sha3_512::new();
    s3_key_hasher.update(s3_key_raw.as_bytes());
    let s3_key_hash = s3_key_hasher.finalize();
    let s3_key = format!("{}{}", s3_prefix, hex::encode(s3_key_hash));
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
                    error!("Unexpected error retrieving s3://{s3_bucket}/{s3_key}: {s3_error}");
                }
            } else {
                // This wasn't expected; log the error.
                error!("Unexpected error retrieving s3://{s3_bucket}/{s3_key}: {e}");
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
            prec: z.real().prec_64(),
        },
        imag: SerFloat {
            value: z.imag().to_string(),
            prec: z.imag().prec_64(),
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
