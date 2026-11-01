//! Extract typed values from loosely structured JSON using paths.
//!
//! ```
//! let json = r#"{ "current_condition": [{ "temp_C": "11",
//!                 "weatherDesc": [{ "value": "Overcast " }] }] }"#;
//! let root = pluck::parse(json).unwrap();
//!
//! // `[x]` wrappers and numeric strings are handled for you.
//! let temp: i32 = pluck::get(&root, "current_condition.temp_C").unwrap();
//! let desc: String = pluck::get(&root, "current_condition.weatherDesc").unwrap();
//! assert_eq!((temp, desc.as_str()), (11, "Overcast "));
//! ```

mod error;
mod extract;
mod path;
mod syntax;
mod transform;

pub use error::{Error, ErrorKind};
pub use extract::{Extract, unwrap};
pub use path::{Match, Path, Segment};
#[cfg(feature = "derive")]
pub use pluck_derive::Extract;
pub use serde_json::Value;
pub use transform::{Opts, field, field_opts, with};

/// Parse a JSON string.
pub fn parse(json: &str) -> Result<Value, Error> {
    serde_json::from_str(json).map_err(|e| Error::new("", ErrorKind::Json(e.to_string())))
}

/// Extract `T` from the whole document (`T` is usually a schema struct).
pub fn from_value<T: Extract>(value: &Value) -> Result<T, Error> {
    T::extract(value, "")
}

/// Parse JSON bytes and extract `T` from them.
pub fn from_slice<T: Extract>(json: &[u8]) -> Result<T, Error> {
    let value =
        serde_json::from_slice(json).map_err(|e| Error::new("", ErrorKind::Json(e.to_string())))?;
    from_value(&value)
}

/// Parse `json` and extract `T` from it.
pub fn from_str<T: Extract>(json: &str) -> Result<T, Error> {
    from_value(&parse(json)?)
}

/// One-off lookup with a path string. Prefer a pre-parsed [`Path`] in hot code.
pub fn get<T: Extract>(root: &Value, path: &str) -> Result<T, Error> {
    field(root, "", &Path::parse(path)?)
}
