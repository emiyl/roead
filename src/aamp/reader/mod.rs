//! Zero-copy AAMP reader API
//!
//! This module provides high-performance, zero-copy readers for AAMP documents.
//! Unlike the owned API which parses the entire document into heap-allocated
//! structures, the reader API provides lazy access to the data with minimal
//! allocations.

pub mod document;
pub mod list;
pub mod object;
pub mod iterators;

pub use document::ParameterIOReader;
pub use list::ParameterListReader;
pub use object::ParameterObjectReader;

/// Error type for AAMP reader operations
#[derive(Debug, thiserror::Error)]
pub enum ReaderError {
    #[error("Invalid binary format: {0}")]
    InvalidFormat(String),
    #[error("Unexpected end of data at offset {0:#x}")]
    UnexpectedEnd(u32),
    #[error("Invalid offset: {0:#x}")]
    InvalidOffset(u32),
    #[error("Invalid parameter type: {0:?}")]
    InvalidParameterType(u8),
    #[error("String encoding error: {0}")]
    StringEncoding(#[from] std::str::Utf8Error),
    #[error("BinRW error: {0}")]
    BinRw(#[from] binrw::Error),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

/// Result type for AAMP reader operations
pub type ReaderResult<T> = std::result::Result<T, ReaderError>;

/// Zero-copy parameter value reader
#[derive(Debug, Clone, Copy)]
pub enum ParameterReader<'a> {
    Bool(bool),
    F32(f32),
    I32(i32),
    Vec2(&'a [f32; 2]),
    Vec3(&'a [f32; 3]),
    Vec4(&'a [f32; 4]),
    Color(&'a [f32; 4]),
    String(&'a str),           // Zero-copy string access
    Binary(&'a [u8]),         // Zero-copy binary access
    Curve1(&'a [[f32; 30]; 1]), // Zero-copy curve access
    Curve2(&'a [[f32; 30]; 2]),
    Curve3(&'a [[f32; 30]; 3]),
    Curve4(&'a [[f32; 30]; 4]),
    BufferInt(&'a [i32]),     // Zero-copy buffer access
    BufferF32(&'a [f32]),     // Zero-copy buffer access
    BufferU32(&'a [u32]),     // Zero-copy buffer access
    BufferBinary(&'a [u8]),   // Zero-copy buffer access
    Quat(&'a [f32; 4]),
    U32(u32),
}

impl<'a> ParameterReader<'a> {
    /// Get the inner bool value.
    pub fn as_bool(&self) -> ReaderResult<bool> {
        match self {
            ParameterReader::Bool(b) => Ok(*b),
            _ => Err(ReaderError::InvalidFormat(
                "Parameter is not a bool".to_string(),
            )),
        }
    }

    /// Get the inner f32 value.
    pub fn as_f32(&self) -> ReaderResult<f32> {
        match self {
            ParameterReader::F32(f) => Ok(*f),
            _ => Err(ReaderError::InvalidFormat(
                "Parameter is not an f32".to_string(),
            )),
        }
    }

    /// Get the inner i32 value.
    pub fn as_i32(&self) -> ReaderResult<i32> {
        match self {
            ParameterReader::I32(i) => Ok(*i),
            _ => Err(ReaderError::InvalidFormat(
                "Parameter is not an i32".to_string(),
            )),
        }
    }

    /// Get the inner u32 value.
    pub fn as_u32(&self) -> ReaderResult<u32> {
        match self {
            ParameterReader::U32(u) => Ok(*u),
            _ => Err(ReaderError::InvalidFormat(
                "Parameter is not a u32".to_string(),
            )),
        }
    }

    /// Get the inner string value (zero-copy).
    pub fn as_str(&self) -> ReaderResult<&'a str> {
        match self {
            ParameterReader::String(s) => Ok(s),
            _ => Err(ReaderError::InvalidFormat(
                "Parameter is not a string".to_string(),
            )),
        }
    }

    /// Get the inner binary data (zero-copy).
    pub fn as_binary(&self) -> ReaderResult<&'a [u8]> {
        match self {
            ParameterReader::Binary(b) => Ok(b),
            ParameterReader::BufferBinary(b) => Ok(b),
            _ => Err(ReaderError::InvalidFormat(
                "Parameter is not binary data".to_string(),
            )),
        }
    }

    /// Get the inner Vec2 value.
    pub fn as_vec2(&self) -> ReaderResult<&'a [f32; 2]> {
        match self {
            ParameterReader::Vec2(v) => Ok(v),
            _ => Err(ReaderError::InvalidFormat(
                "Parameter is not a Vec2".to_string(),
            )),
        }
    }

    /// Get the inner Vec3 value.
    pub fn as_vec3(&self) -> ReaderResult<&'a [f32; 3]> {
        match self {
            ParameterReader::Vec3(v) => Ok(v),
            _ => Err(ReaderError::InvalidFormat(
                "Parameter is not a Vec3".to_string(),
            )),
        }
    }

    /// Get the inner Vec4 value.
    pub fn as_vec4(&self) -> ReaderResult<&'a [f32; 4]> {
        match self {
            ParameterReader::Vec4(v) => Ok(v),
            _ => Err(ReaderError::InvalidFormat(
                "Parameter is not a Vec4".to_string(),
            )),
        }
    }

    /// Get the inner Color value.
    pub fn as_color(&self) -> ReaderResult<&'a [f32; 4]> {
        match self {
            ParameterReader::Color(c) => Ok(c),
            _ => Err(ReaderError::InvalidFormat(
                "Parameter is not a Color".to_string(),
            )),
        }
    }

    /// Get the inner Quat value.
    pub fn as_quat(&self) -> ReaderResult<&'a [f32; 4]> {
        match self {
            ParameterReader::Quat(q) => Ok(q),
            _ => Err(ReaderError::InvalidFormat(
                "Parameter is not a Quat".to_string(),
            )),
        }
    }

    /// Get the inner BufferF32 value (zero-copy).
    pub fn as_buffer_f32(&self) -> ReaderResult<&'a [f32]> {
        match self {
            ParameterReader::BufferF32(b) => Ok(b),
            _ => Err(ReaderError::InvalidFormat(
                "Parameter is not a BufferF32".to_string(),
            )),
        }
    }

    /// Get the inner BufferI32 value (zero-copy).
    pub fn as_buffer_int(&self) -> ReaderResult<&'a [i32]> {
        match self {
            ParameterReader::BufferInt(b) => Ok(b),
            _ => Err(ReaderError::InvalidFormat(
                "Parameter is not a BufferI32".to_string(),
            )),
        }
    }

    /// Get the inner BufferU32 value (zero-copy).
    pub fn as_buffer_u32(&self) -> ReaderResult<&'a [u32]> {
        match self {
            ParameterReader::BufferU32(b) => Ok(b),
            _ => Err(ReaderError::InvalidFormat(
                "Parameter is not a BufferU32".to_string(),
            )),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parameter_reader_accessors() {
        // Test primitive accessors
        let bool_param = ParameterReader::Bool(true);
        assert_eq!(bool_param.as_bool().unwrap(), true);
        assert!(bool_param.as_f32().is_err());

        let f32_param = ParameterReader::F32(3.14);
        assert_eq!(f32_param.as_f32().unwrap(), 3.14);
        assert!(f32_param.as_bool().is_err());

        let string_param = ParameterReader::String("test");
        assert_eq!(string_param.as_str().unwrap(), "test");
        assert!(string_param.as_f32().is_err());

        let binary_data = &[1u8, 2, 3, 4];
        let binary_param = ParameterReader::Binary(binary_data);
        assert_eq!(binary_param.as_binary().unwrap(), binary_data);
        assert!(binary_param.as_str().is_err());
    }
}