//! AAMP parameter list reader implementation

use super::{ReaderError, ReaderResult, ParameterObjectReader};
use crate::aamp::{Name, ResParameterList, ResParameterObj};
use binrw::BinRead;
use std::io::Cursor;

/// Zero-copy AAMP parameter list reader
#[derive(Debug, Clone)]
pub struct ParameterListReader<'a> {
    data: &'a [u8],
    list_header_offset: u32,  // Offset where this list's header starts
    header: ResParameterList,
}

impl<'a> ParameterListReader<'a> {
    /// Create a new parameter list reader
    pub(super) fn new(
        data: &'a [u8],
        list_header_offset: u32,
        header: &ResParameterList,
    ) -> ReaderResult<Self> {
        Ok(Self {
            data,
            list_header_offset,
            header: header.clone(),
        })
    }

    /// Get the number of child parameter lists
    pub fn list_count(&self) -> usize {
        self.header.list_count as usize
    }

    /// Get the number of child parameter objects
    pub fn object_count(&self) -> usize {
        self.header.object_count as usize
    }

    /// Check if this list is empty
    pub fn is_empty(&self) -> bool {
        self.list_count() == 0 && self.object_count() == 0
    }

    /// Get a child parameter list by name
    pub fn get_list(&self, name: impl Into<Name>) -> Option<ParameterListReader<'a>> {
        let name = name.into();
        // Find the list by iterating through all lists
        for i in 0..self.list_count() {
            if let Ok(Some((list_name, list_reader))) = self.get_list_at_index(i) {
                if list_name == name {
                    return Some(list_reader);
                }
            }
        }
        None
    }

    /// Get a child parameter object by name
    pub fn get_object(&self, name: impl Into<Name>) -> Option<ParameterObjectReader<'a>> {
        let name = name.into();
        // Find the object by iterating through all objects
        for i in 0..self.object_count() {
            if let Ok(Some((obj_name, obj_reader))) = self.get_object_at_index(i) {
                if obj_name == name {
                    return Some(obj_reader);
                }
            }
        }
        None
    }

    /// Get a parameter list at a specific index
    pub fn get_list_at_index(&self, index: usize) -> ReaderResult<Option<(Name, ParameterListReader<'a>)>> {
        if index >= self.list_count() {
            return Ok(None);
        }

        // Calculate offset to the list headers (relative to this list's header)
        let lists_offset = self.list_header_offset + (self.header.lists_rel_offset as u32 * 4);
        let list_header_offset = lists_offset + (index * 12) as u32; // Each list header is 12 bytes (0xC)

        if list_header_offset as usize + 8 > self.data.len() {
            return Err(ReaderError::UnexpectedEnd(list_header_offset));
        }

        // Parse the list header
        let mut cursor = Cursor::new(&self.data[list_header_offset as usize..]);
        let list_header = ResParameterList::read(&mut cursor).map_err(ReaderError::BinRw)?;

        // Create the child list reader
        let child_reader = ParameterListReader::new(
            self.data,
            list_header_offset, // The child list's header starts at this offset
            &list_header,
        )?;

        Ok(Some((list_header.name, child_reader)))
    }

    /// Get a parameter object at a specific index
    pub fn get_object_at_index(&self, index: usize) -> ReaderResult<Option<(Name, ParameterObjectReader<'a>)>> {
        if index >= self.object_count() {
            return Ok(None);
        }

        // Calculate offset to the object headers (relative to this list's header)
        let objects_offset = self.list_header_offset + (self.header.objects_rel_offset as u32 * 4);
        let object_header_offset = objects_offset + (index * 8) as u32; // Each object header is 8 bytes

        if object_header_offset as usize + 8 > self.data.len() {
            return Err(ReaderError::UnexpectedEnd(object_header_offset));
        }

        // Parse the object header
        let mut cursor = Cursor::new(&self.data[object_header_offset as usize..]);
        let object_header = ResParameterObj::read(&mut cursor).map_err(ReaderError::BinRw)?;

        // Create the object reader
        let obj_reader = ParameterObjectReader::new(
            self.data,
            objects_offset + (object_header.params_rel_offset as u32 * 4),
            object_header.param_count,
        )?;

        Ok(Some((object_header.name, obj_reader)))
    }

    /// Get an iterator over all child parameter lists
    pub fn lists(&'a self) -> ParameterListIterator<'a> {
        ParameterListIterator {
            list_reader: self,
            current_index: 0,
        }
    }

    /// Get an iterator over all child parameter objects
    pub fn objects(&'a self) -> ParameterObjectIterator<'a> {
        ParameterObjectIterator {
            list_reader: self,
            current_index: 0,
        }
    }

    /// Convert this parameter list reader to an owned ParameterList
    pub fn to_owned(&self) -> crate::Result<crate::aamp::ParameterList> {
        use crate::aamp::{ParameterList, ParameterListMap, ParameterObjectMap};

        let mut lists = ParameterListMap::default();
        let mut objects = ParameterObjectMap::default();

        // Convert child lists
        for result in self.lists() {
            match result {
                Ok((name, list_reader)) => {
                    let owned_list = list_reader.to_owned()?;
                    lists.insert(name, owned_list);
                }
                Err(e) => return Err(crate::Error::Aamp(format!("Error converting list: {:?}", e))),
            }
        }

        // Convert child objects
        for result in self.objects() {
            match result {
                Ok((name, obj_reader)) => {
                    let owned_obj = obj_reader.to_owned()?;
                    objects.insert(name, owned_obj);
                }
                Err(e) => return Err(crate::Error::Aamp(format!("Error converting object: {:?}", e))),
            }
        }

        Ok(ParameterList { lists, objects })
    }
}

/// Iterator over child parameter lists
pub struct ParameterListIterator<'a> {
    list_reader: &'a ParameterListReader<'a>,
    current_index: usize,
}

impl<'a> Iterator for ParameterListIterator<'a> {
    type Item = ReaderResult<(Name, ParameterListReader<'a>)>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.current_index >= self.list_reader.list_count() {
            return None;
        }

        let result = self.list_reader.get_list_at_index(self.current_index);
        self.current_index += 1;

        match result {
            Ok(Some(item)) => Some(Ok(item)),
            Ok(None) => None,
            Err(e) => Some(Err(e)),
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let remaining = self.list_reader.list_count().saturating_sub(self.current_index);
        (remaining, Some(remaining))
    }
}

impl<'a> ExactSizeIterator for ParameterListIterator<'a> {
    fn len(&self) -> usize {
        self.list_reader.list_count().saturating_sub(self.current_index)
    }
}

/// Iterator over child parameter objects
pub struct ParameterObjectIterator<'a> {
    list_reader: &'a ParameterListReader<'a>,
    current_index: usize,
}

impl<'a> Iterator for ParameterObjectIterator<'a> {
    type Item = ReaderResult<(Name, ParameterObjectReader<'a>)>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.current_index >= self.list_reader.object_count() {
            return None;
        }

        let result = self.list_reader.get_object_at_index(self.current_index);
        self.current_index += 1;

        match result {
            Ok(Some(item)) => Some(Ok(item)),
            Ok(None) => None,
            Err(e) => Some(Err(e)),
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let remaining = self.list_reader.object_count().saturating_sub(self.current_index);
        (remaining, Some(remaining))
    }
}

impl<'a> ExactSizeIterator for ParameterObjectIterator<'a> {
    fn len(&self) -> usize {
        self.list_reader.object_count().saturating_sub(self.current_index)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parameter_list_reader_basic() {
        // Test basic functionality with available AAMP files
        let test_files = [
            "test/aamp/Lizalfos.bphysics",
            "test/aamp/ActorInfo.product.aamp",
        ];

        for file_path in test_files {
            if let Ok(data) = std::fs::read(file_path) {
                if let Ok(reader) = super::super::ParameterIOReader::new(&data) {
                    let root = reader.root();
                    
                    println!("Testing ParameterListReader with {}", file_path);
                    println!("  Lists: {}, Objects: {}", root.list_count(), root.object_count());
                    
                    // Test list iteration
                    for (i, result) in root.lists().enumerate() {
                        match result {
                            Ok((name, child_list)) => {
                                println!("  List {}: {} (lists: {}, objects: {})", 
                                    i, name, child_list.list_count(), child_list.object_count());
                            }
                            Err(e) => {
                                println!("  List {}: Error {:?}", i, e);
                                break;
                            }
                        }
                        if i >= 3 { break; } // Limit output
                    }
                    
                    // Test object iteration
                    for (i, result) in root.objects().enumerate() {
                        match result {
                            Ok((name, obj)) => {
                                println!("  Object {}: {} (params: {})", i, name, obj.param_count());
                            }
                            Err(e) => {
                                println!("  Object {}: Error {:?}", i, e);
                                break;
                            }
                        }
                        if i >= 3 { break; } // Limit output
                    }
                }
            }
        }
    }

    #[test]
    fn test_parameter_list_reader_conversion() {
        // Test conversion to owned format
        let test_files = ["test/aamp/Lizalfos.bphysics"];
        
        for file_path in test_files {
            if let Ok(data) = std::fs::read(file_path) {
                if let Ok(reader) = super::super::ParameterIOReader::new(&data) {
                    if let Ok(owned_root) = reader.root().to_owned() {
                        println!("✅ Successfully converted parameter list to owned format");
                        println!("  Lists: {}, Objects: {}", owned_root.lists.len(), owned_root.objects.len());
                    }
                }
            }
        }
    }
}