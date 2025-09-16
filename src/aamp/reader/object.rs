//! AAMP parameter object reader implementation

use super::{ReaderError, ReaderResult, ParameterReader};
use crate::aamp::{Name, ResParameter, Type};
use crate::types::*;
use binrw::BinRead;
use std::io::Cursor;

/// Zero-copy AAMP parameter object reader
#[derive(Debug, Clone)]
pub struct ParameterObjectReader<'a> {
    data: &'a [u8],
    params_offset: u32,
    param_count: u16,
}

impl<'a> ParameterObjectReader<'a> {
    /// Create a new parameter object reader
    pub(super) fn new(
        data: &'a [u8],
        params_offset: u32,
        param_count: u16,
    ) -> ReaderResult<Self> {
        Ok(Self {
            data,
            params_offset,
            param_count,
        })
    }

    /// Get the number of parameters in this object
    pub fn param_count(&self) -> usize {
        self.param_count as usize
    }

    /// Check if this object is empty
    pub fn is_empty(&self) -> bool {
        self.param_count == 0
    }

    /// Get a parameter by name
    pub fn get(&self, name: impl Into<Name>) -> Option<ParameterReader<'a>> {
        let name = name.into();
        // Find parameter by iterating through all parameters
        for i in 0..self.param_count() {
            if let Ok(Some((param_name, param_reader))) = self.get_parameter_at_index(i) {
                if param_name == name {
                    return Some(param_reader);
                }
            }
        }
        None
    }

    /// Get a parameter at a specific index
    pub fn get_parameter_at_index(&self, index: usize) -> ReaderResult<Option<(Name, ParameterReader<'a>)>> {
        if index >= self.param_count() {
            return Ok(None);
        }

        // Calculate offset to the parameter header
        let param_header_offset = self.params_offset + (index * 8) as u32; // Each parameter header is 8 bytes

        if param_header_offset as usize + 8 > self.data.len() {
            return Err(ReaderError::UnexpectedEnd(param_header_offset));
        }

        // Parse the parameter header
        let mut cursor = Cursor::new(&self.data[param_header_offset as usize..]);
        let param_header = ResParameter::read(&mut cursor).map_err(ReaderError::BinRw)?;

        // Calculate the actual data offset (relative to the parameter header position)
        let data_offset = param_header_offset + (param_header.data_rel_offset.as_u32() * 4);

        // Parse the parameter value based on its type
        let param_value = self.parse_parameter_value(&param_header.type_, data_offset)?;

        Ok(Some((param_header.name, param_value)))
    }

    /// Parse a parameter value from the data
    fn parse_parameter_value(&self, param_type: &Type, data_offset: u32) -> ReaderResult<ParameterReader<'a>> {
        if data_offset as usize >= self.data.len() {
            return Err(ReaderError::InvalidOffset(data_offset));
        }

        match param_type {
            Type::Bool => {
                let value = u32::from_le_bytes(
                    self.data[data_offset as usize..data_offset as usize + 4]
                        .try_into()
                        .map_err(|_| ReaderError::UnexpectedEnd(data_offset))?
                ) != 0;
                Ok(ParameterReader::Bool(value))
            }
            Type::F32 => {
                let bytes = self.data[data_offset as usize..data_offset as usize + 4]
                    .try_into()
                    .map_err(|_| ReaderError::UnexpectedEnd(data_offset))?;
                let value = f32::from_le_bytes(bytes);
                Ok(ParameterReader::F32(value))
            }
            Type::Int => {
                let bytes = self.data[data_offset as usize..data_offset as usize + 4]
                    .try_into()
                    .map_err(|_| ReaderError::UnexpectedEnd(data_offset))?;
                let value = i32::from_le_bytes(bytes);
                Ok(ParameterReader::I32(value))
            }
            Type::U32 => {
                let bytes = self.data[data_offset as usize..data_offset as usize + 4]
                    .try_into()
                    .map_err(|_| ReaderError::UnexpectedEnd(data_offset))?;
                let value = u32::from_le_bytes(bytes);
                Ok(ParameterReader::U32(value))
            }
            Type::Vec2 => {
                let bytes: &[u8] = self.data[data_offset as usize..data_offset as usize + 8]
                    .try_into()
                    .map_err(|_| ReaderError::UnexpectedEnd(data_offset))?;
                // SAFETY: Vec2 is just [f32; 2] which has the same layout as 8 bytes
                let vec_ref: &[f32; 2] = unsafe { &*(bytes.as_ptr() as *const [f32; 2]) };
                Ok(ParameterReader::Vec2(vec_ref))
            }
            Type::Vec3 => {
                let bytes: &[u8] = self.data[data_offset as usize..data_offset as usize + 12]
                    .try_into()
                    .map_err(|_| ReaderError::UnexpectedEnd(data_offset))?;
                // SAFETY: Vec3 is just [f32; 3] which has the same layout as 12 bytes
                let vec_ref: &[f32; 3] = unsafe { &*(bytes.as_ptr() as *const [f32; 3]) };
                Ok(ParameterReader::Vec3(vec_ref))
            }
            Type::Vec4 => {
                let bytes: &[u8] = self.data[data_offset as usize..data_offset as usize + 16]
                    .try_into()
                    .map_err(|_| ReaderError::UnexpectedEnd(data_offset))?;
                // SAFETY: Vec4 is just [f32; 4] which has the same layout as 16 bytes
                let vec_ref: &[f32; 4] = unsafe { &*(bytes.as_ptr() as *const [f32; 4]) };
                Ok(ParameterReader::Vec4(vec_ref))
            }
            Type::Color => {
                let bytes: &[u8] = self.data[data_offset as usize..data_offset as usize + 16]
                    .try_into()
                    .map_err(|_| ReaderError::UnexpectedEnd(data_offset))?;
                // SAFETY: Color is just [f32; 4] which has the same layout as 16 bytes
                let color_ref: &[f32; 4] = unsafe { &*(bytes.as_ptr() as *const [f32; 4]) };
                Ok(ParameterReader::Color(color_ref))
            }
            Type::Quat => {
                let bytes: &[u8] = self.data[data_offset as usize..data_offset as usize + 16]
                    .try_into()
                    .map_err(|_| ReaderError::UnexpectedEnd(data_offset))?;
                // SAFETY: Quat is just [f32; 4] which has the same layout as 16 bytes
                let quat_ref: &[f32; 4] = unsafe { &*(bytes.as_ptr() as *const [f32; 4]) };
                Ok(ParameterReader::Quat(quat_ref))
            }
            Type::String32 => {
                self.parse_fixed_string(data_offset, 32)
            }
            Type::String64 => {
                self.parse_fixed_string(data_offset, 64)
            }
            Type::String256 => {
                self.parse_fixed_string(data_offset, 256)
            }
            Type::StringRef => {
                self.parse_string_ref(data_offset)
            }
            Type::BufferInt => {
                self.parse_buffer::<i32>(data_offset)
                    .map(ParameterReader::BufferInt)
            }
            Type::BufferF32 => {
                self.parse_buffer::<f32>(data_offset)
                    .map(ParameterReader::BufferF32)
            }
            Type::BufferU32 => {
                self.parse_buffer::<u32>(data_offset)
                    .map(ParameterReader::BufferU32)
            }
            Type::BufferBinary => {
                self.parse_binary_buffer(data_offset)
                    .map(ParameterReader::BufferBinary)
            }
            _ => Err(ReaderError::InvalidParameterType(*param_type as u8)),
        }
    }

    /// Parse a fixed-length string parameter
    fn parse_fixed_string(&self, data_offset: u32, max_len: usize) -> ReaderResult<ParameterReader<'a>> {
        if data_offset as usize + max_len > self.data.len() {
            return Err(ReaderError::UnexpectedEnd(data_offset));
        }

        let string_data = &self.data[data_offset as usize..data_offset as usize + max_len];
        
        // Find the null terminator
        let null_pos = string_data
            .iter()
            .position(|&b| b == 0)
            .unwrap_or(max_len);

        let string_slice = std::str::from_utf8(&string_data[..null_pos])
            .map_err(ReaderError::StringEncoding)?;

        Ok(ParameterReader::String(string_slice))
    }

    /// Parse a string reference parameter
    fn parse_string_ref(&self, data_offset: u32) -> ReaderResult<ParameterReader<'a>> {
        // String references are stored as a 4-byte offset in the string section
        let string_offset_bytes = self.data[data_offset as usize..data_offset as usize + 4]
            .try_into()
            .map_err(|_| ReaderError::UnexpectedEnd(data_offset))?;
        let string_offset = u32::from_le_bytes(string_offset_bytes);

        // Parse the header to get the correct string section offset
        let mut cursor = std::io::Cursor::new(&self.data[..]);
        let header = crate::aamp::ResHeader::read(&mut cursor).map_err(ReaderError::BinRw)?;
        let string_section_start = 0x30 + header.data_section_size;
        let actual_string_offset = string_section_start + string_offset;

        if actual_string_offset as usize >= self.data.len() {
            return Err(ReaderError::InvalidOffset(actual_string_offset));
        }

        // Find the null terminator
        let string_data = &self.data[actual_string_offset as usize..];
        let null_pos = string_data
            .iter()
            .position(|&b| b == 0)
            .ok_or_else(|| ReaderError::InvalidFormat("No null terminator in string ref".to_string()))?;

        let string_slice = std::str::from_utf8(&string_data[..null_pos])
            .map_err(ReaderError::StringEncoding)?;

        Ok(ParameterReader::String(string_slice))
    }

    /// Parse a typed buffer (int, float, u32)
    fn parse_buffer<T>(&self, data_offset: u32) -> ReaderResult<&'a [T]> 
    where
        T: Copy + 'static,
    {
        // Buffer format: [u32 size][data...]
        let size_bytes = self.data[data_offset as usize..data_offset as usize + 4]
            .try_into()
            .map_err(|_| ReaderError::UnexpectedEnd(data_offset))?;
        let size = u32::from_le_bytes(size_bytes) as usize;

        let element_size = std::mem::size_of::<T>();
        let total_size = size * element_size;
        let buffer_start = data_offset as usize + 4;

        if buffer_start + total_size > self.data.len() {
            return Err(ReaderError::UnexpectedEnd(data_offset + 4 + total_size as u32));
        }

        let buffer_data = &self.data[buffer_start..buffer_start + total_size];
        
        // SAFETY: We've validated the size and alignment
        let buffer_slice = unsafe {
            std::slice::from_raw_parts(
                buffer_data.as_ptr() as *const T,
                size,
            )
        };

        Ok(buffer_slice)
    }

    /// Parse a binary buffer
    fn parse_binary_buffer(&self, data_offset: u32) -> ReaderResult<&'a [u8]> {
        // Binary buffer format: [u32 size][data...]
        let size_bytes = self.data[data_offset as usize..data_offset as usize + 4]
            .try_into()
            .map_err(|_| ReaderError::UnexpectedEnd(data_offset))?;
        let size = u32::from_le_bytes(size_bytes) as usize;

        let buffer_start = data_offset as usize + 4;

        if buffer_start + size > self.data.len() {
            return Err(ReaderError::UnexpectedEnd(data_offset + 4 + size as u32));
        }

        Ok(&self.data[buffer_start..buffer_start + size])
    }

    /// Get an iterator over all parameters
    pub fn iter(&'a self) -> ParameterIterator<'a> {
        ParameterIterator {
            object_reader: self,
            current_index: 0,
        }
    }

    /// Convenience method to get a string parameter
    pub fn get_str(&self, name: impl Into<Name>) -> Option<&'a str> {
        self.get(name.into())?.as_str().ok()
    }

    /// Convenience method to get a binary parameter
    pub fn get_binary(&self, name: impl Into<Name>) -> Option<&'a [u8]> {
        self.get(name.into())?.as_binary().ok()
    }

    /// Convenience method to get a bool parameter
    pub fn get_bool(&self, name: impl Into<Name>) -> Option<bool> {
        self.get(name.into())?.as_bool().ok()
    }

    /// Convenience method to get an f32 parameter
    pub fn get_f32(&self, name: impl Into<Name>) -> Option<f32> {
        self.get(name.into())?.as_f32().ok()
    }

    /// Convenience method to get an i32 parameter
    pub fn get_i32(&self, name: impl Into<Name>) -> Option<i32> {
        self.get(name.into())?.as_i32().ok()
    }

    /// Convenience method to get a u32 parameter
    pub fn get_u32(&self, name: impl Into<Name>) -> Option<u32> {
        self.get(name.into())?.as_u32().ok()
    }

    /// Convert this parameter object reader to an owned ParameterObject
    pub fn to_owned(&self) -> crate::Result<crate::aamp::ParameterObject> {
        use crate::aamp::ParameterObject;

        let mut params = crate::aamp::ParameterStructureMap::default();

        for result in self.iter() {
            match result {
                Ok((name, param_reader)) => {
                    let owned_param = self.convert_parameter_to_owned(param_reader)?;
                    params.insert(name, owned_param);
                }
                Err(e) => return Err(crate::Error::Aamp(format!("Error converting parameter: {:?}", e))),
            }
        }

        Ok(ParameterObject(params))
    }

    /// Convert a parameter reader to an owned parameter
    fn convert_parameter_to_owned(&self, param: ParameterReader<'a>) -> crate::Result<crate::aamp::Parameter> {
        use crate::aamp::Parameter;

        match param {
            ParameterReader::Bool(b) => Ok(Parameter::Bool(b)),
            ParameterReader::F32(f) => Ok(Parameter::F32(f)),
            ParameterReader::I32(i) => Ok(Parameter::I32(i)),
            ParameterReader::U32(u) => Ok(Parameter::U32(u)),
            ParameterReader::Vec2(v) => Ok(Parameter::Vec2(Vector2f { x: v[0], y: v[1] })),
            ParameterReader::Vec3(v) => Ok(Parameter::Vec3(Vector3f { x: v[0], y: v[1], z: v[2] })),
            ParameterReader::Vec4(v) => Ok(Parameter::Vec4(Vector4f { x: v[0], y: v[1], z: v[2], t: v[3] })),
            ParameterReader::Color(c) => Ok(Parameter::Color(Color { r: c[0], g: c[1], b: c[2], a: c[3] })),
            ParameterReader::Quat(q) => Ok(Parameter::Quat(Quat { a: q[0], b: q[1], c: q[2], d: q[3] })),
            ParameterReader::String(s) => {
                // Convert to appropriate string type based on length
                if s.len() <= 32 {
                    Ok(Parameter::String32(s.into()))
                } else if s.len() <= 64 {
                    Ok(Parameter::String64(Box::new(s.into())))
                } else if s.len() <= 256 {
                    Ok(Parameter::String256(Box::new(s.into())))
                } else {
                    Ok(Parameter::StringRef(s.into()))
                }
            }
            ParameterReader::BufferInt(buf) => Ok(Parameter::BufferInt(buf.to_vec())),
            ParameterReader::BufferF32(buf) => Ok(Parameter::BufferF32(buf.to_vec())),
            ParameterReader::BufferU32(buf) => Ok(Parameter::BufferU32(buf.to_vec())),
            ParameterReader::BufferBinary(buf) => Ok(Parameter::BufferBinary(buf.to_vec())),
            ParameterReader::Binary(bin) => Ok(Parameter::BufferBinary(bin.to_vec())),
            _ => Err(crate::Error::Aamp("Unsupported parameter type for conversion".to_string())),
        }
    }
}

/// Iterator over parameters in an object
pub struct ParameterIterator<'a> {
    object_reader: &'a ParameterObjectReader<'a>,
    current_index: usize,
}

impl<'a> Iterator for ParameterIterator<'a> {
    type Item = ReaderResult<(Name, ParameterReader<'a>)>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.current_index >= self.object_reader.param_count() {
            return None;
        }

        let result = self.object_reader.get_parameter_at_index(self.current_index);
        self.current_index += 1;

        match result {
            Ok(Some(item)) => Some(Ok(item)),
            Ok(None) => None,
            Err(e) => Some(Err(e)),
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let remaining = self.object_reader.param_count().saturating_sub(self.current_index);
        (remaining, Some(remaining))
    }
}

impl<'a> ExactSizeIterator for ParameterIterator<'a> {
    fn len(&self) -> usize {
        self.object_reader.param_count().saturating_sub(self.current_index)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parameter_object_reader_basic() {
        // Test with available AAMP files
        let test_files = ["test/aamp/Lizalfos.bphysics"];

        for file_path in test_files {
            if let Ok(data) = std::fs::read(file_path) {
                if let Ok(reader) = super::super::ParameterIOReader::new(&data) {
                    let root = reader.root();
                    
                    println!("Testing ParameterObjectReader with {}", file_path);
                    
                    // Test object iteration
                    for result in root.objects().take(3) {
                        match result {
                            Ok((name, obj)) => {
                                println!("  Object {}: {} params", name, obj.param_count());
                                
                                // Test parameter iteration
                                for param_result in obj.iter().take(3) {
                                    match param_result {
                                        Ok((param_name, param_reader)) => {
                                            match param_reader {
                                                ParameterReader::Bool(b) => println!("    {}: Bool({})", param_name, b),
                                                ParameterReader::F32(f) => println!("    {}: F32({})", param_name, f),
                                                ParameterReader::I32(i) => println!("    {}: I32({})", param_name, i),
                                                ParameterReader::U32(u) => println!("    {}: U32({})", param_name, u),
                                                ParameterReader::String(s) => println!("    {}: String('{}')", param_name, s),
                                                _ => println!("    {}: {:?}", param_name, param_reader),
                                            }
                                        }
                                        Err(e) => {
                                            println!("    Parameter error: {:?}", e);
                                            break;
                                        }
                                    }
                                }
                            }
                            Err(e) => {
                                println!("  Object error: {:?}", e);
                                break;
                            }
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn test_parameter_object_reader_conversion() {
        // Test conversion to owned format
        let test_files = ["test/aamp/Lizalfos.bphysics"];
        
        for file_path in test_files {
            if let Ok(data) = std::fs::read(file_path) {
                if let Ok(reader) = super::super::ParameterIOReader::new(&data) {
                    let root = reader.root();
                    
                    for result in root.objects().take(1) {
                        if let Ok((name, obj)) = result {
                            if let Ok(_owned_obj) = obj.to_owned() {
                                println!("✅ Successfully converted parameter object '{}' to owned format", name);
                            }
                            break;
                        }
                    }
                }
            }
        }
    }
}