use {
    crate::types::{Error, ErrorResponse, ErrorType},
    aws_lambda_events::event::lambda_function_urls::LambdaFunctionUrlResponse,
    http::{status::StatusCode, HeaderMap, HeaderValue},
};

/// Returns a LambdaFunctionUrlResponse of 400: BadRequest.
pub(crate) fn bad_request(message: &str, request_id: &str) -> LambdaFunctionUrlResponse {
    http_error(StatusCode::BAD_REQUEST, "BadRequest", message, request_id)
}

/// Returns a LambdaFunctionUrlResponse of 404: NotFound.
pub(crate) fn not_found(request_id: &str) -> LambdaFunctionUrlResponse {
    http_error(StatusCode::NOT_FOUND, "NotFound", "Not found", request_id)
}

// Returns an LambdaFunctionUrlResponse error with the given status code, error code, and message.
pub(crate) fn http_error(
    status_code: StatusCode,
    error_code: &str,
    message: &str,
    request_id: &str,
) -> LambdaFunctionUrlResponse {
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
