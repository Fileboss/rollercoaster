//! Minimal re-implementation of the parts of MFC `CArchive` used by Looping.
//!
//! The same [`Archive`] value is used for loading and storing, mirroring the
//! MFC `Serialize(CArchive&)` idiom: every serializer is written once and
//! branches on [`Archive::is_loading`] only where Looping itself does.

use crate::Error;

/// MFC object tags (see `afx.h` / `arcobj.cpp`).
pub const NULL_TAG: u16 = 0;
pub const NEW_CLASS_TAG: u16 = 0xFFFF;
pub const CLASS_TAG: u16 = 0x8000;
pub const BIG_OBJECT_TAG: u16 = 0x7FFF;
pub const BIG_CLASS_TAG: u32 = 0x8000_0000;
pub const MAX_MAP_COUNT: u32 = 0x3FFF_FFFE;

pub type Result<T> = std::result::Result<T, Error>;

enum Mode<'a> {
    Load { data: &'a [u8], pos: usize },
    Store { buf: Vec<u8> },
}

pub struct Archive<'a> {
    mode: Mode<'a>,
    /// Document format version (the first `int` of the settings block,
    /// `CLoopingDoc+0x36c` in the original binary).
    pub version: i32,
    /// Loading: index -> class name (`None` for object slots).
    load_map: Vec<Option<String>>,
    /// Storing: class name -> map index.
    store_classes: Vec<(String, u32)>,
    store_count: u32,
}

impl<'a> Archive<'a> {
    pub fn loader(data: &'a [u8]) -> Self {
        Archive {
            mode: Mode::Load { data, pos: 0 },
            version: 0,
            // index 0 is the NULL object
            load_map: vec![None],
            store_classes: Vec::new(),
            store_count: 1,
        }
    }

    pub fn storer(version: i32) -> Archive<'static> {
        Archive {
            mode: Mode::Store { buf: Vec::new() },
            version,
            load_map: vec![None],
            store_classes: Vec::new(),
            store_count: 1,
        }
    }

    pub fn is_loading(&self) -> bool {
        matches!(self.mode, Mode::Load { .. })
    }

    pub fn position(&self) -> usize {
        match &self.mode {
            Mode::Load { pos, .. } => *pos,
            Mode::Store { buf } => buf.len(),
        }
    }

    pub fn remaining(&self) -> usize {
        match &self.mode {
            Mode::Load { data, pos } => data.len() - pos,
            Mode::Store { .. } => 0,
        }
    }

    pub fn into_bytes(self) -> Vec<u8> {
        match self.mode {
            Mode::Store { buf } => buf,
            Mode::Load { .. } => Vec::new(),
        }
    }

    fn take(&mut self, n: usize) -> Result<&'a [u8]> {
        match &mut self.mode {
            Mode::Load { data, pos } => {
                if *pos + n > data.len() {
                    return Err(Error::Eof { at: *pos, wanted: n });
                }
                let d: &'a [u8] = data;
                let s = &d[*pos..*pos + n];
                *pos += n;
                Ok(s)
            }
            Mode::Store { .. } => unreachable!("take() while storing"),
        }
    }

    fn put(&mut self, bytes: &[u8]) {
        match &mut self.mode {
            Mode::Store { buf } => buf.extend_from_slice(bytes),
            Mode::Load { .. } => unreachable!("put() while loading"),
        }
    }

    // ---------------------------------------------------------------- scalars

    pub fn bytes(&mut self, v: &mut [u8]) -> Result<()> {
        if self.is_loading() {
            let n = v.len();
            v.copy_from_slice(self.take(n)?);
        } else {
            self.put(v);
        }
        Ok(())
    }

    pub fn u8(&mut self, v: &mut u8) -> Result<()> {
        let mut b = [*v];
        self.bytes(&mut b)?;
        *v = b[0];
        Ok(())
    }

    pub fn u16(&mut self, v: &mut u16) -> Result<()> {
        let mut b = v.to_le_bytes();
        self.bytes(&mut b)?;
        *v = u16::from_le_bytes(b);
        Ok(())
    }

    pub fn i32(&mut self, v: &mut i32) -> Result<()> {
        let mut b = v.to_le_bytes();
        self.bytes(&mut b)?;
        *v = i32::from_le_bytes(b);
        Ok(())
    }

    pub fn u32(&mut self, v: &mut u32) -> Result<()> {
        let mut b = v.to_le_bytes();
        self.bytes(&mut b)?;
        *v = u32::from_le_bytes(b);
        Ok(())
    }

    pub fn u64(&mut self, v: &mut u64) -> Result<()> {
        let mut b = v.to_le_bytes();
        self.bytes(&mut b)?;
        *v = u64::from_le_bytes(b);
        Ok(())
    }

    pub fn f64(&mut self, v: &mut f64) -> Result<()> {
        let mut b = v.to_le_bytes();
        self.bytes(&mut b)?;
        *v = f64::from_le_bytes(b);
        Ok(())
    }

    /// An `int` the original code reads and discards, and writes as 0.
    pub fn skip_i32(&mut self) -> Result<()> {
        let mut x = 0;
        self.i32(&mut x)
    }

    // ----------------------------------------------------------------- counts

    /// `CArchive::ReadCount` / `WriteCount`.
    pub fn count(&mut self, v: &mut usize) -> Result<()> {
        if self.is_loading() {
            let mut w = 0u16;
            self.u16(&mut w)?;
            if w != 0xFFFF {
                *v = w as usize;
                return Ok(());
            }
            let mut dw = 0u32;
            self.u32(&mut dw)?;
            if dw != 0xFFFF_FFFF {
                *v = dw as usize;
                return Ok(());
            }
            let mut qw = 0u64;
            self.u64(&mut qw)?;
            *v = qw as usize;
        } else {
            let n = *v;
            if n < 0xFFFF {
                self.put(&(n as u16).to_le_bytes());
            } else {
                self.put(&0xFFFFu16.to_le_bytes());
                if n < 0xFFFF_FFFF {
                    self.put(&(n as u32).to_le_bytes());
                } else {
                    self.put(&0xFFFF_FFFFu32.to_le_bytes());
                    self.put(&(n as u64).to_le_bytes());
                }
            }
        }
        Ok(())
    }

    // ---------------------------------------------------------------- strings

    /// `AfxReadStringLength`: returns (length, char size).
    fn read_string_length(&mut self) -> Result<(usize, usize)> {
        let mut char_size = 1;
        let mut b = 0u8;
        self.u8(&mut b)?;
        if b < 0xFF {
            return Ok((b as usize, char_size));
        }
        let mut w = 0u16;
        self.u16(&mut w)?;
        if w == 0xFFFE {
            char_size = 2;
            self.u8(&mut b)?;
            if b < 0xFF {
                return Ok((b as usize, char_size));
            }
            self.u16(&mut w)?;
        }
        if w < 0xFFFF {
            return Ok((w as usize, char_size));
        }
        let mut dw = 0u32;
        self.u32(&mut dw)?;
        if dw < 0xFFFF_FFFF {
            return Ok((dw as usize, char_size));
        }
        let mut qw = 0u64;
        self.u64(&mut qw)?;
        Ok((qw as usize, char_size))
    }

    /// `AfxWriteStringLength(ar, len, TRUE)`.
    fn write_string_length(&mut self, n: usize) {
        self.put(&[0xFF]);
        self.put(&0xFFFEu16.to_le_bytes());
        if n < 0xFF {
            self.put(&[n as u8]);
        } else {
            self.put(&[0xFF]);
            if n < 0xFFFE {
                self.put(&(n as u16).to_le_bytes());
            } else {
                self.put(&0xFFFFu16.to_le_bytes());
                if n < 0xFFFF_FFFF {
                    self.put(&(n as u32).to_le_bytes());
                } else {
                    self.put(&0xFFFF_FFFFu32.to_le_bytes());
                    self.put(&(n as u64).to_le_bytes());
                }
            }
        }
    }

    /// `CString` (Unicode build): `operator>>` / `operator<<`.
    pub fn string(&mut self, v: &mut String) -> Result<()> {
        if self.is_loading() {
            let (n, cs) = self.read_string_length()?;
            let raw = self.take(n * cs)?;
            *v = if cs == 2 {
                let units: Vec<u16> = raw.chunks_exact(2).map(|c| u16::from_le_bytes([c[0], c[1]])).collect();
                String::from_utf16_lossy(&units)
            } else {
                // MultiByteToWideChar(CP_THREAD_ACP): Looping runs on Windows-1252
                // locales for all known files.
                raw.iter().map(|&b| cp1252(b)).collect()
            };
        } else {
            let units: Vec<u16> = v.encode_utf16().collect();
            self.write_string_length(units.len());
            for u in units {
                self.put(&u.to_le_bytes());
            }
        }
        Ok(())
    }

    /// `CStringList::Serialize`.
    pub fn string_list(&mut self, v: &mut Vec<String>) -> Result<()> {
        let mut n = v.len();
        self.count(&mut n)?;
        if self.is_loading() {
            v.clear();
            for _ in 0..n {
                let mut s = String::new();
                self.string(&mut s)?;
                v.push(s);
            }
        } else {
            for s in v.iter_mut() {
                self.string(s)?;
            }
        }
        Ok(())
    }

    // ------------------------------------------------------- object protocol

    /// Reads the tag preceding a serialized `CObject*` (`CArchive::ReadClass`).
    pub fn read_object_header(&mut self) -> Result<ObjectHeader> {
        let start = self.position();
        let mut tag = 0u16;
        self.u16(&mut tag)?;
        let (ob_tag, is_class) = if tag == BIG_OBJECT_TAG {
            let mut big = 0u32;
            self.u32(&mut big)?;
            (big & !BIG_CLASS_TAG, big & BIG_CLASS_TAG != 0)
        } else {
            ((tag & !CLASS_TAG) as u32, tag & CLASS_TAG != 0)
        };
        if tag == NULL_TAG {
            return Ok(ObjectHeader::Null);
        }
        let class = if tag == NEW_CLASS_TAG {
            let mut schema = 0u16;
            self.u16(&mut schema)?;
            let mut len = 0u16;
            self.u16(&mut len)?;
            let name = self.take(len as usize)?;
            let name = String::from_utf8_lossy(name).into_owned();
            self.load_map.push(Some(name.clone()));
            name
        } else if is_class {
            match self.load_map.get(ob_tag as usize) {
                Some(Some(name)) => name.clone(),
                _ => return Err(Error::BadClassTag { at: start, tag: ob_tag }),
            }
        } else {
            return Ok(ObjectHeader::Reference(ob_tag));
        };
        // the new object takes the next map slot
        self.load_map.push(None);
        Ok(ObjectHeader::New { class, index: (self.load_map.len() - 1) as u32 })
    }

    /// Writes the tag for a new object of `class` (`CArchive::WriteObject`).
    pub fn write_object_header(&mut self, class: &str, schema: u16) {
        if let Some(&(_, idx)) = self.store_classes.iter().find(|(c, _)| c == class) {
            if idx < BIG_OBJECT_TAG as u32 {
                self.put(&((CLASS_TAG as u32 | idx) as u16).to_le_bytes());
            } else {
                self.put(&BIG_OBJECT_TAG.to_le_bytes());
                self.put(&(BIG_CLASS_TAG | idx).to_le_bytes());
            }
        } else {
            self.put(&NEW_CLASS_TAG.to_le_bytes());
            self.put(&schema.to_le_bytes());
            self.put(&(class.len() as u16).to_le_bytes());
            self.put(class.as_bytes());
            self.store_classes.push((class.to_owned(), self.store_count));
            self.store_count += 1;
        }
        // object slot
        self.store_count += 1;
    }

    pub fn write_null_object(&mut self) {
        self.put(&NULL_TAG.to_le_bytes());
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ObjectHeader {
    Null,
    Reference(u32),
    New { class: String, index: u32 },
}

fn cp1252(b: u8) -> char {
    const HIGH: [u16; 32] = [
        0x20AC, 0x81, 0x201A, 0x0192, 0x201E, 0x2026, 0x2020, 0x2021, 0x02C6, 0x2030, 0x0160, 0x2039, 0x0152, 0x8D,
        0x017D, 0x8F, 0x90, 0x2018, 0x2019, 0x201C, 0x201D, 0x2022, 0x2013, 0x2014, 0x02DC, 0x2122, 0x0161, 0x203A,
        0x0153, 0x9D, 0x017E, 0x0178,
    ];
    match b {
        0x80..=0x9F => char::from_u32(HIGH[(b - 0x80) as usize] as u32).unwrap_or('?'),
        _ => b as char,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn string_roundtrip() {
        for s in ["", "Chien", &"x".repeat(300), "Propriétaire"] {
            let mut w = Archive::storer(51);
            let mut v = s.to_string();
            w.string(&mut v).unwrap();
            let bytes = w.into_bytes();
            let mut r = Archive::loader(&bytes);
            let mut out = String::new();
            r.string(&mut out).unwrap();
            assert_eq!(out, s);
            assert_eq!(r.remaining(), 0);
        }
    }

    #[test]
    fn object_tags() {
        let mut w = Archive::storer(51);
        w.write_object_header("CDrawArc", 0);
        w.write_object_header("CDrawArc", 0);
        w.write_object_header("CDrawTexte", 0);
        w.write_object_header("CDrawArc", 0);
        let bytes = w.into_bytes();
        let mut r = Archive::loader(&bytes);
        let h: Vec<_> = (0..4).map(|_| r.read_object_header().unwrap()).collect();
        assert_eq!(h[0], ObjectHeader::New { class: "CDrawArc".into(), index: 2 });
        assert_eq!(h[1], ObjectHeader::New { class: "CDrawArc".into(), index: 3 });
        assert_eq!(h[2], ObjectHeader::New { class: "CDrawTexte".into(), index: 5 });
        assert_eq!(h[3], ObjectHeader::New { class: "CDrawArc".into(), index: 6 });
    }
}
