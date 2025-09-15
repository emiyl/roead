//! Iterator implementations for BYML readers
//!
//! Provides iterator support for arrays and maps to enable idiomatic Rust
//! iteration over BYML containers.

use super::node::{BymlArrayReader, BymlMapReader, BymlNodeReader};

/// Iterator over BYML array elements
pub struct BymlArrayIterator<'a> {
    reader: &'a BymlArrayReader<'a>,
    index: usize,
}

impl<'a> BymlArrayReader<'a> {
    /// Create an iterator over array elements
    pub fn iter(&'a self) -> BymlArrayIterator<'a> {
        BymlArrayIterator {
            reader: self,
            index: 0,
        }
    }
}

impl<'a> Iterator for BymlArrayIterator<'a> {
    type Item = BymlNodeReader<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.index < self.reader.len() {
            let node = self.reader.get(self.index);
            self.index += 1;
            node
        } else {
            None
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let remaining = self.reader.len() - self.index;
        (remaining, Some(remaining))
    }
}

impl<'a> ExactSizeIterator for BymlArrayIterator<'a> {
    fn len(&self) -> usize {
        self.reader.len() - self.index
    }
}

/// Iterator over BYML map keys
pub struct BymlMapKeysIterator<'a> {
    reader: &'a BymlMapReader<'a>,
    index: usize,
}

impl<'a> BymlMapReader<'a> {
    /// Create an iterator over map keys
    pub fn keys(&'a self) -> BymlMapKeysIterator<'a> {
        BymlMapKeysIterator {
            reader: self,
            index: 0,
        }
    }
}

impl<'a> Iterator for BymlMapKeysIterator<'a> {
    type Item = Result<&'a str, super::ReaderError>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.index < self.reader.len() {
            let result = self.reader.get_key_at_index(self.index);
            self.index += 1;
            Some(result)
        } else {
            None
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let remaining = self.reader.len() - self.index;
        (remaining, Some(remaining))
    }
}

/// Iterator over BYML map values
pub struct BymlMapValuesIterator<'a> {
    reader: &'a BymlMapReader<'a>,
    index: usize,
}

impl<'a> BymlMapReader<'a> {
    /// Create an iterator over map values
    pub fn values(&'a self) -> BymlMapValuesIterator<'a> {
        BymlMapValuesIterator {
            reader: self,
            index: 0,
        }
    }
}

impl<'a> Iterator for BymlMapValuesIterator<'a> {
    type Item = BymlNodeReader<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.index < self.reader.len() {
            let node = self.reader.get_value_at_index(self.index);
            self.index += 1;
            node
        } else {
            None
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let remaining = self.reader.len() - self.index;
        (remaining, Some(remaining))
    }
}

/// Iterator over BYML map key-value pairs
pub struct BymlMapIterator<'a> {
    reader: &'a BymlMapReader<'a>,
    index: usize,
}

impl<'a> BymlMapReader<'a> {
    /// Create an iterator over map key-value pairs
    pub fn iter(&'a self) -> BymlMapIterator<'a> {
        BymlMapIterator {
            reader: self,
            index: 0,
        }
    }
}

impl<'a> Iterator for BymlMapIterator<'a> {
    type Item = Result<(&'a str, BymlNodeReader<'a>), super::ReaderError>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.index < self.reader.len() {
            let key_result = self.reader.get_key_at_index(self.index);
            let value = self.reader.get_value_at_index(self.index);
            self.index += 1;

            match (key_result, value) {
                (Ok(key), Some(value)) => Some(Ok((key, value))),
                (Err(e), _) => Some(Err(e)),
                (Ok(_), None) => {
                    // This shouldn't happen if the map is well-formed
                    Some(Err(super::ReaderError::InvalidFormat(
                        "Map value missing for existing key".to_string(),
                    )))
                }
            }
        } else {
            None
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let remaining = self.reader.len() - self.index;
        (remaining, Some(remaining))
    }
}

#[cfg(test)]
mod tests {
    // Note: These tests would need actual BYML data to test properly
    // For now, they serve as compilation checks

    #[test]
    fn test_iterator_compilation() {
        // This test just ensures the iterator types compile correctly
        // Real tests would need valid BYML data
    }
}
