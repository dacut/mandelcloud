use {
    rug::{Complex, Float},
    serde::{Deserialize, Deserializer, Serialize, Serializer},
    std::{
        error::Error as StdError,
        fmt::{Display, Formatter, Result as FmtResult},
    },
};

/// A type for serializing an arbitrary-precision float.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct SerFloat {
    pub value: String,
    pub prec: u64,
}

impl From<&Float> for SerFloat {
    /// Create a SerFloat from a rug Float.
    fn from(f: &Float) -> Self {
        Self {
            value: f.to_string(),
            prec: f.prec_64(),
        }
    }
}

/// A type for serializing an arbitrary-precision complex number.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct SerComplex {
    pub real: SerFloat,
    pub imag: SerFloat,
}

impl From<&Complex> for SerComplex {
    /// Create a SerComplex from a rug Complex.
    fn from(c: &Complex) -> Self {
        Self {
            real: SerFloat::from(c.real()),
            imag: SerFloat::from(c.imag()),
        }
    }
}

/// The input message for a request to compute a point.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct ComputePointRequest {
    /// The starting point of the Mandelbrot set to compute.
    pub c: SerComplex,

    /// The maximum number of iterations to compute.
    pub iterations: u32,
}

/// Reponse for a computed point.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct ComputePointResponse {
    /// The result.
    pub computed_point_result: ComputePointResult,

    /// The request id.
    pub request_id: String,
}

/// The output message for a request to compute a point.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ComputePointResult {
    /// The starting point of the Mandelbrot set that was computed.
    pub c: SerComplex,

    /// The iteration at which the point escaped, if any.
    pub escape_iteration: Option<u32>,

    /// The number of iterations computed before the point escaped.
    pub computed_iterations: u32,

    /// The final value of the point after computation.
    pub final_value: SerComplex,
}

impl Display for SerFloat {
    fn fmt(&self, f: &mut Formatter) -> FmtResult {
        write!(f, "{}#{}", self.value, self.prec)
    }
}

impl Display for SerComplex {
    fn fmt(&self, f: &mut Formatter) -> FmtResult {
        write!(f, "({}, {})", self.real, self.imag)
    }
}

/// An error structure returned when a request fails.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct ErrorResponse {
    /// The underlying error.
    pub error: Error,

    /// The request id.
    pub request_id: String,
}

/// An error structure.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct Error {
    /// The type of error.
    #[serde(rename = "Type")]
    pub error_type: ErrorType,

    /// The error code.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub code: Option<String>,

    /// The error message.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

impl Display for Error {
    fn fmt(&self, f: &mut Formatter) -> FmtResult {
        serde_json::to_string(self).unwrap().fmt(f)
    }
}

impl StdError for Error {}

/// The type of error.
#[derive(Clone, Debug)]
pub enum ErrorType {
    Sender,
    Receiver,
}

impl Serialize for ErrorType {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self {
            ErrorType::Sender => serializer.serialize_str("Sender"),
            ErrorType::Receiver => serializer.serialize_str("Receiver"),
        }
    }
}

impl<'a> Deserialize<'a> for ErrorType {
    fn deserialize<D>(deserializer: D) -> Result<ErrorType, D::Error>
    where
        D: Deserializer<'a>,
    {
        let s = String::deserialize(deserializer)?;
        match s.as_str() {
            "Sender" => Ok(ErrorType::Sender),
            "Receiver" => Ok(ErrorType::Receiver),
            _ => Err(serde::de::Error::custom("invalid error type")),
        }
    }
}
