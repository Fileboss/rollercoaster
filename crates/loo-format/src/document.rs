//! `CLoopingDoc::Serialize` (0x140074c20).

use serde::Serialize;

use crate::archive::{Archive, ObjectHeader, Result};
use crate::objects::DrawObj;
use crate::settings::Settings;
use crate::Error;

/// Format version written by Looping 4.1.
pub const CURRENT_VERSION: i32 = 0x33;

/// `CUneVueUrbanisme` (0x14000e8b0): a named view.
#[derive(Debug, Clone, Default, PartialEq, Serialize)]
pub struct View {
    pub f8: i32,
    pub name: String,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Document {
    /// 0x1910 — drawing extent
    pub page_size: (i32, i32),
    /// 0x191c — background colour (`COLORREF`)
    pub background: u32,
    /// 0x334 — grid step of the magnetism (10)
    pub f334: i32,
    /// 0x338 / 0x33c — 0x33c: magnetism on (« Grille » / « Demi-grille »)
    pub f338: u16,
    pub f33c: u16,
    /// 0x350
    pub f350: [u8; 4],
    /// 0x36c..0xbf8, starts with the format version.
    pub settings: Settings,
    /// View +0x2f0 / +0x2f8: zoom in percent (twice), then a pair always
    /// 100 in the samples.
    pub frame_state: [[i32; 2]; 2],
    /// 0x340 (format > 0x1c) — magnetism on half the grid step
    pub f340: u16,
    /// 0x348 (format > 0x30)
    pub s348: String,
    /// 0x228 — `CTypedPtrList<CObList, CUneVueUrbanisme*>` (format > 0x2b)
    pub views: Vec<View>,
    /// 0xbf8 — 33 strings (format > 0xf)
    pub strings_bf8: Vec<String>,
    /// 0xd00 (format > 0x12)
    pub s_d00: String,
    /// 0xd10 — 50 strings (format > 0x12)
    pub strings_d10: Vec<String>,
    /// 0xea0 — 5 strings, see `ser_ea0`.
    pub strings_ea0: [String; 5],
    /// 0x17f8 — the drawing objects, in z-order.
    pub objects: Vec<DrawObj>,
    /// Version the document was loaded from.
    pub loaded_version: i32,
}

impl Default for Document {
    fn default() -> Self {
        Document {
            page_size: (10000, 10000),
            background: 0xF0F0F0,
            f334: 10,
            f338: 0,
            f33c: 1,
            f350: [0xc0, 0xc0, 0x80, 0x00],
            settings: Settings::new_default(),
            frame_state: [[100, 100], [100, 100]],
            f340: 0,
            s348: String::new(),
            views: Vec::new(),
            strings_bf8: vec![String::new(); 33],
            s_d00: String::new(),
            strings_d10: vec![String::new(); 50],
            strings_ea0: Default::default(),
            objects: Vec::new(),
            loaded_version: CURRENT_VERSION,
        }
    }
}

impl Document {
    pub fn load(data: &[u8]) -> std::result::Result<Self, Error> {
        let mut ar = Archive::loader(data);
        let mut doc = Document::default();
        doc.ser(&mut ar)?;
        if ar.remaining() != 0 {
            return Err(Error::TrailingData { at: ar.position(), len: ar.remaining() });
        }
        Ok(doc)
    }

    pub fn save(&self) -> std::result::Result<Vec<u8>, Error> {
        let mut doc = self.clone();
        doc.settings.set_version(CURRENT_VERSION);
        let mut ar = Archive::storer(CURRENT_VERSION);
        doc.ser(&mut ar)?;
        Ok(ar.into_bytes())
    }

    fn ser(&mut self, ar: &mut Archive) -> Result<()> {
        let loading = ar.is_loading();
        let mut size = [self.page_size.0, self.page_size.1];
        ar.i32(&mut size[0])?;
        ar.i32(&mut size[1])?;
        self.page_size = (size[0], size[1]);
        ar.u32(&mut self.background)?;
        ar.i32(&mut self.f334)?;
        ar.u16(&mut self.f338)?;
        ar.u16(&mut self.f33c)?;
        ar.bytes(&mut self.f350)?;
        ar.bytes(self.settings.raw_mut())?;
        if loading {
            ar.version = self.settings.version();
            self.loaded_version = ar.version;
            if ar.version > CURRENT_VERSION {
                // Looping warns "enregistré par une version plus récente" and continues.
                eprintln!("warning: file format {:#x} is newer than {:#x}", ar.version, CURRENT_VERSION);
            }
        }
        let v = ar.version;
        let all = !loading;
        for pair in &mut self.frame_state {
            ar.i32(&mut pair[0])?;
            ar.i32(&mut pair[1])?;
        }
        if all || v > 0x1c {
            ar.u16(&mut self.f340)?;
        } else {
            self.f340 = 0;
        }
        if all || v > 0x30 {
            ar.string(&mut self.s348)?;
        }
        if all || v > 0x2b {
            self.ser_views(ar)?;
        }
        if loading {
            self.settings.upgrade_from(v);
        }
        if all || v > 0xf {
            self.strings_bf8.resize(33, String::new());
            for s in &mut self.strings_bf8 {
                ar.string(s)?;
            }
        }
        if all || v > 0x12 {
            ar.string(&mut self.s_d00)?;
            self.strings_d10.resize(50, String::new());
            for s in &mut self.strings_d10 {
                ar.string(s)?;
            }
        }
        let e = &mut self.strings_ea0;
        if all || v > 0x12 {
            ar.string(&mut e[0])?;
            ar.string(&mut e[1])?;
            ar.string(&mut e[4])?;
        }
        if all || v > 0x23 {
            ar.string(&mut e[2])?;
            ar.string(&mut e[3])?;
        }
        if loading {
            self.settings.fix_after_strings();
            if ar.remaining() == 0 {
                return Ok(());
            }
        }
        self.ser_objects(ar)?;
        for o in &mut self.objects {
            o.ser_links(ar)?;
        }
        if loading {
            self.settings.clamp();
        }
        Ok(())
    }

    fn ser_views(&mut self, ar: &mut Archive) -> Result<()> {
        let mut n = self.views.len();
        ar.count(&mut n)?;
        if ar.is_loading() {
            self.views.clear();
            for _ in 0..n {
                let at = ar.position();
                match ar.read_object_header()? {
                    ObjectHeader::New { class, .. } if class == "CUneVueUrbanisme" => {}
                    other => return Err(Error::Unexpected { at, what: format!("{other:?} in view list") }),
                }
                let mut view = View::default();
                ar.i32(&mut view.f8)?;
                ar.string(&mut view.name)?;
                self.views.push(view);
            }
        } else {
            for view in &mut self.views {
                ar.write_object_header("CUneVueUrbanisme", 0);
                ar.i32(&mut view.f8)?;
                ar.string(&mut view.name)?;
            }
        }
        Ok(())
    }

    fn ser_objects(&mut self, ar: &mut Archive) -> Result<()> {
        let mut n = self.objects.len();
        ar.count(&mut n)?;
        if ar.is_loading() {
            self.objects.clear();
            for _ in 0..n {
                let at = ar.position();
                match ar.read_object_header()? {
                    ObjectHeader::New { class, .. } => {
                        let mut o =
                            DrawObj::empty(&class).ok_or_else(|| Error::UnknownClass { at, class: class.clone() })?;
                        o.ser(ar)?;
                        self.objects.push(o);
                    }
                    other => return Err(Error::Unexpected { at, what: format!("{other:?} in object list") }),
                }
            }
        } else {
            for o in &mut self.objects {
                ar.write_object_header(&o.class, 0);
                o.ser(ar)?;
            }
        }
        Ok(())
    }

    pub fn object(&self, id: i32) -> Option<&DrawObj> {
        if id == 0 {
            return None;
        }
        self.objects.iter().find(|o| o.base.id == id)
    }
}
