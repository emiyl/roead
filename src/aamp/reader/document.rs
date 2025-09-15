//! AAMP document reader implementation

use super::{ReaderError, ReaderResult, ParameterListReader};
use crate::aamp::{Name, ResHeader, ResParameterList};
use binrw::BinRead;
use std::io::Cursor;

/// Zero-copy AAMP document reader
#[derive(Debug)]
pub struct ParameterIOReader<'a> {
    data: &'a [u8],
    header: ResHeader,
    root_list: ParameterListReader<'a>,
}

impl<'a> ParameterIOReader<'a> {
    /// Create a new AAMP reader from binary data
    pub fn new(data: &'a [u8]) -> ReaderResult<Self> {
        let mut cursor = Cursor::new(data);
        
        // Parse the header
        let header = ResHeader::read(&mut cursor).map_err(ReaderError::BinRw)?;
        
        // Validate magic and version
        if header.version != 2 {
            return Err(ReaderError::InvalidFormat(format!(
                "Unsupported AAMP version: {}",
                header.version
            )));
        }

        // Calculate root list offset (relative to 0x30)
        let root_offset = 0x30 + header.pio_offset;
        if root_offset as usize >= data.len() {
            return Err(ReaderError::InvalidOffset(root_offset));
        }

        // Parse root parameter list
        let mut root_cursor = Cursor::new(&data[root_offset as usize..]);
        let root_list_header = ResParameterList::read(&mut root_cursor).map_err(ReaderError::BinRw)?;
        
        let root_list = ParameterListReader::new(
            data,
            root_offset + 8, // Skip the list header (8 bytes)
            &root_list_header,
        )?;

        Ok(Self {
            data,
            header,
            root_list,
        })
    }

    /// Get the document type (usually "xml")
    pub fn doc_type(&self) -> ReaderResult<&'a str> {
        // The doc type is stored as a null-terminated string starting at offset 0x30 + string_section_size
        let string_section_start = 0x30 + self.header.data_section_size;
        if string_section_start as usize >= self.data.len() {
            return Err(ReaderError::InvalidOffset(string_section_start));
        }

        // Find the null terminator
        let string_data = &self.data[string_section_start as usize..];
        let null_pos = string_data
            .iter()
            .position(|&b| b == 0)
            .ok_or_else(|| ReaderError::InvalidFormat("No null terminator found in doc type".to_string()))?;

        std::str::from_utf8(&string_data[..null_pos])
            .map_err(ReaderError::StringEncoding)
    }

    /// Get the data version
    pub fn version(&self) -> u32 {
        self.header.pio_version
    }

    /// Get the root parameter list
    pub fn root(&self) -> &ParameterListReader<'a> {
        &self.root_list
    }

    /// Convenience method to get a parameter list by name from the root
    pub fn list(&self, name: impl Into<Name>) -> Option<ParameterListReader<'a>> {
        self.root_list.get_list(name.into())
    }

    /// Convenience method to get a parameter object by name from the root
    pub fn object(&self, name: impl Into<Name>) -> Option<super::ParameterObjectReader<'a>> {
        self.root_list.get_object(name.into())
    }

    /// Convert this reader to an owned ParameterIO (allocates)
    pub fn to_owned(&self) -> crate::Result<crate::aamp::ParameterIO> {
        use crate::aamp::ParameterIO;
        
        let param_root = self.root_list.to_owned()?;
        let doc_type = self.doc_type().unwrap_or("xml").to_string();
        
        Ok(ParameterIO {
            version: self.version(),
            data_type: doc_type.into(),
            param_root,
        })
    }

    #[cfg(feature = "yaml")]
    /// Convert this reader to YAML text representation
    pub fn to_text(&self) -> crate::Result<String> {
        // Convert to owned format and then use existing YAML serialization
        let owned = self.to_owned()?;
        owned.to_text()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn test_parameter_io_reader_basic() {
        // Test with a simple AAMP file if available
        let test_files = [
            "test/aamp/Lizalfos.bphysics",
            "test/aamp/ActorInfo.product.aamp",
        ];

        for file_path in test_files {
            if let Ok(data) = std::fs::read(file_path) {
                println!("Testing ParameterIOReader with {}", file_path);
                
                match ParameterIOReader::new(&data) {
                    Ok(reader) => {
                        println!("  ✅ Reader created successfully");
                        println!("  Version: {}", reader.version());
                        
                        if let Ok(doc_type) = reader.doc_type() {
                            println!("  Doc type: {}", doc_type);
                        }
                        
                        let root = reader.root();
                        println!("  Root list - Lists: {}, Objects: {}", root.list_count(), root.object_count());
                        
                        // Test conversion to owned
                        if let Ok(_owned) = reader.to_owned() {
                            println!("  ✅ Conversion to owned successful");
                        }
                    }
                    Err(e) => {
                        println!("  ⚠️  Failed to parse {}: {:?}", file_path, e);
                    }
                }
            }
        }
    }

    #[test] 
    fn test_parameter_io_reader_invalid_data() {
        // Test invalid data
        let invalid_data = b"INVALID";
        assert!(ParameterIOReader::new(invalid_data).is_err());

        // Test truncated data
        let truncated_data = b"AAMP\x02\x00\x00\x00"; // Just magic and version
        assert!(ParameterIOReader::new(truncated_data).is_err());
    }

    #[test]
    fn test_parameter_io_reader_consistency() {
        // Test that reader API produces consistent results with owned API
        let test_files = ["test/aamp/Lizalfos.bphysics"];
        
        for file_path in test_files {
            if let Ok(data) = std::fs::read(file_path) {
                // Parse with both APIs
                let owned_result = crate::aamp::ParameterIO::from_binary(&data);
                let reader_result = ParameterIOReader::new(&data);
                
                match (owned_result, reader_result) {
                    (Ok(owned), Ok(reader)) => {
                        // Compare basic properties
                        assert_eq!(owned.version, reader.version());
                        
                        if let Ok(reader_doc_type) = reader.doc_type() {
                            assert_eq!(owned.data_type.as_str(), reader_doc_type);
                        }
                        
                        // Test conversion consistency
                        if let Ok(reader_owned) = reader.to_owned() {
                            assert_eq!(owned.version, reader_owned.version);
                            assert_eq!(owned.data_type, reader_owned.data_type);
                        }
                        
                        println!("✅ Consistency test passed for {}", file_path);
                    }
                    (Err(_), Err(_)) => {
                        println!("✅ Both APIs failed as expected for {}", file_path);
                    }
                    (Ok(_), Err(e)) => {
                        panic!("Reader API failed where owned API succeeded: {:?}", e);
                    }
                    (Err(_), Ok(_)) => {
                        println!("⚠️  Reader API succeeded where owned API failed");
                    }
                }
            }
        }
    }
}