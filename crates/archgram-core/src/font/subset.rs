//! A TrueType subsetter: keeps the glyphs a diagram uses and nothing else
//! (ARCHITECTURE.md, Render).
//!
//! It handles what archgram embeds, static fonts with `glyf` outlines, and
//! refuses anything else. The output keeps the tables a browser's font
//! sanitiser requires (`OS/2`, `cmap`, `glyf`, `head`, `hhea`, `hmtx`,
//! `loca`, `maxp`, `name`, `post`), with:
//!
//! - glyphs renumbered densely: `.notdef` first, then the text's glyphs in
//!   character order, then the parts of composite glyphs as they are found;
//! - hinting removed (`fpgm`, `prep`, `cvt ` dropped, glyph instructions
//!   emptied), since the SVG is scaled freely;
//! - layout tables dropped (`GSUB`, `GPOS`, `GDEF`): archgram sets text
//!   without kerning or ligatures, matching how it measures;
//! - `name` reduced to the family, style, copyright and licence entries,
//!   which the SIL Open Font License asks to travel with the font;
//! - `post` reduced to version 3 (no glyph names).
//!
//! Table layouts follow the OpenType specification (learn.microsoft.com/typography/opentype/spec).

use std::collections::{BTreeMap, BTreeSet};

use skrifa::prelude::{FontRef, MetadataProvider};

/// Why a font cannot be subset.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SubsetError(pub String);

fn err<T>(message: impl Into<String>) -> Result<T, SubsetError> {
    Err(SubsetError(message.into()))
}

/// Big-endian reads that fail cleanly past the end of the data.
struct Reader<'a>(&'a [u8]);

impl<'a> Reader<'a> {
    fn u16(&self, at: usize) -> Result<u16, SubsetError> {
        match self.0.get(at..at + 2) {
            Some(b) => Ok(u16::from_be_bytes([b[0], b[1]])),
            None => err(format!("read past the end of a table at {at}")),
        }
    }

    fn i16(&self, at: usize) -> Result<i16, SubsetError> {
        self.u16(at).map(|v| i16::from_be_bytes(v.to_be_bytes()))
    }

    fn u32(&self, at: usize) -> Result<u32, SubsetError> {
        match self.0.get(at..at + 4) {
            Some(b) => Ok(u32::from_be_bytes([b[0], b[1], b[2], b[3]])),
            None => err(format!("read past the end of a table at {at}")),
        }
    }

    fn slice(&self, from: usize, to: usize) -> Result<&'a [u8], SubsetError> {
        self.0
            .get(from..to)
            .ok_or_else(|| SubsetError(format!("range {from}..{to} is outside the table")))
    }
}

/// The font's tables by tag.
fn tables(font: &[u8]) -> Result<BTreeMap<[u8; 4], &[u8]>, SubsetError> {
    let r = Reader(font);
    if r.u32(0)? != 0x0001_0000 {
        return err("not a TrueType font with glyf outlines (sfnt version is not 1.0)");
    }
    let count = usize::from(r.u16(4)?);
    let mut out = BTreeMap::new();
    for i in 0..count {
        let rec = 12 + 16 * i;
        let tag: [u8; 4] = r.slice(rec, rec + 4)?.try_into().unwrap_or_default();
        let (offset, length) = (r.u32(rec + 8)? as usize, r.u32(rec + 12)? as usize);
        out.insert(tag, r.slice(offset, offset + length)?);
    }
    for required in [
        *b"head", *b"hhea", *b"maxp", *b"hmtx", *b"loca", *b"glyf", *b"cmap", *b"name", *b"post",
        *b"OS/2",
    ] {
        if !out.contains_key(&required) {
            return err(format!(
                "the font has no `{}` table",
                String::from_utf8_lossy(&required)
            ));
        }
    }
    if out.contains_key(b"fvar") {
        return err("variable fonts are not supported; use a static instance");
    }
    Ok(out)
}

// Composite glyph flags (OpenType, glyf: Composite glyph description).
const ARG_1_AND_2_ARE_WORDS: u16 = 0x0001;
const WE_HAVE_A_SCALE: u16 = 0x0008;
const MORE_COMPONENTS: u16 = 0x0020;
const WE_HAVE_AN_X_AND_Y_SCALE: u16 = 0x0040;
const WE_HAVE_A_TWO_BY_TWO: u16 = 0x0080;
const WE_HAVE_INSTRUCTIONS: u16 = 0x0100;

/// A composite glyph's parts: each part's id with the offset of that id in
/// the glyph's data, the end of the records, and whether instructions follow.
type Components = (Vec<(usize, u16)>, usize, bool);

/// The glyph ids a composite glyph is built from, with the byte offset of
/// each id inside the glyph's data; the end of the component records; and
/// whether instructions follow them.
fn components(glyph: &[u8]) -> Result<Components, SubsetError> {
    let r = Reader(glyph);
    let mut at = 10;
    let mut out = Vec::new();
    let mut instructions = false;
    loop {
        let flags = r.u16(at)?;
        out.push((at + 2, r.u16(at + 2)?));
        instructions |= flags & WE_HAVE_INSTRUCTIONS != 0;
        at = next_record(glyph, at)?;
        if flags & MORE_COMPONENTS == 0 {
            return Ok((out, at, instructions));
        }
    }
}

/// Makes a subset of `font` holding the glyphs for `chars`.
///
/// # Errors
///
/// When the font is not a static TrueType font or its tables are malformed.
#[allow(clippy::too_many_lines)] // one pass over the tables, in the order they depend on each other
pub fn subset(font: &[u8], chars: &BTreeSet<char>) -> Result<Vec<u8>, SubsetError> {
    let t = tables(font)?;
    let head = Reader(t[b"head"]);
    let hhea = Reader(t[b"hhea"]);
    let maxp = Reader(t[b"maxp"]);
    let (loca, glyf, hmtx) = (Reader(t[b"loca"]), Reader(t[b"glyf"]), Reader(t[b"hmtx"]));
    let long_loca = head.i16(50)? == 1;
    let num_glyphs = maxp.u16(4)?;
    let num_h_metrics = hhea.u16(34)?;
    if num_h_metrics == 0 || num_h_metrics > num_glyphs {
        return err(format!(
            "hhea gives {num_h_metrics} horizontal metrics for {num_glyphs} glyphs"
        ));
    }

    // Which old glyph each character maps to, through the font's own cmap.
    let fref = FontRef::new(font).map_err(|e| SubsetError(e.to_string()))?;
    let charmap = fref.charmap();
    let mapped: Vec<(char, u16)> = chars
        .iter()
        .filter_map(|&c| {
            charmap
                .map(c)
                .map(|g| (c, u16::try_from(g.to_u32()).unwrap_or(0)))
        })
        .filter(|&(_, g)| g != 0)
        .collect();

    let glyph = |gid: u16| -> Result<&[u8], SubsetError> {
        let i = usize::from(gid);
        let (from, to) = if long_loca {
            (loca.u32(4 * i)? as usize, loca.u32(4 * i + 4)? as usize)
        } else {
            (
                usize::from(loca.u16(2 * i)?) * 2,
                usize::from(loca.u16(2 * i + 2)?) * 2,
            )
        };
        glyf.slice(from, to)
    };

    // New numbering: .notdef, the text's glyphs, then composite parts as found.
    let mut order: Vec<u16> = vec![0];
    let mut new_id: BTreeMap<u16, u16> = BTreeMap::from([(0, 0)]);
    let mut add = |gid: u16, order: &mut Vec<u16>| {
        new_id.entry(gid).or_insert_with(|| {
            order.push(gid);
            u16::try_from(order.len() - 1).unwrap_or(u16::MAX)
        });
    };
    for &(_, g) in &mapped {
        add(g, &mut order);
    }
    let mut i = 0;
    while i < order.len() {
        let data = glyph(order[i])?;
        if data.len() >= 10 && Reader(data).i16(0)? < 0 {
            for (_, part) in components(data)?.0 {
                if part >= num_glyphs {
                    return err(format!(
                        "glyph {} refers to glyph {part}, which does not exist",
                        order[i]
                    ));
                }
                add(part, &mut order);
            }
        }
        i += 1;
    }
    let count = u16::try_from(order.len()).map_err(|_| SubsetError("too many glyphs".into()))?;

    // glyf and loca: each glyph copied without instructions, padded to four bytes.
    let mut new_glyf = Vec::new();
    let mut new_loca = Vec::with_capacity(4 * (order.len() + 1));
    for &old in &order {
        new_loca.extend_from_slice(
            &u32::try_from(new_glyf.len())
                .unwrap_or(u32::MAX)
                .to_be_bytes(),
        );
        let data = glyph(old)?;
        if data.len() >= 10 {
            let r = Reader(data);
            let contours = r.i16(0)?;
            if contours >= 0 {
                // Simple glyph: header, end points, instruction length and instructions, then the rest.
                let end_points = 10 + 2 * usize::try_from(contours).unwrap_or(0);
                let instructions = usize::from(r.u16(end_points)?);
                new_glyf.extend_from_slice(r.slice(0, end_points)?);
                new_glyf.extend_from_slice(&0u16.to_be_bytes());
                new_glyf.extend_from_slice(r.slice(end_points + 2 + instructions, data.len())?);
            } else {
                // Composite glyph: renumber the parts, drop the trailing instructions.
                let (parts, end, has_instructions) = components(data)?;
                let mut copy = r.slice(0, end)?.to_vec();
                for (at, part) in parts {
                    copy[at..at + 2].copy_from_slice(&new_id[&part].to_be_bytes());
                }
                if has_instructions {
                    // The instructions after the records are already cut off; clear the flag that announced them.
                    let mut at = 10;
                    while at < end {
                        let flags = Reader(&copy).u16(at)?;
                        copy[at..at + 2]
                            .copy_from_slice(&(flags & !WE_HAVE_INSTRUCTIONS).to_be_bytes());
                        at = next_record(&copy, at)?;
                    }
                }
                new_glyf.extend_from_slice(&copy);
            }
        }
        while new_glyf.len() % 4 != 0 {
            new_glyf.push(0);
        }
    }
    new_loca.extend_from_slice(
        &u32::try_from(new_glyf.len())
            .unwrap_or(u32::MAX)
            .to_be_bytes(),
    );

    // hmtx: every glyph gets a full metric.
    let mut new_hmtx = Vec::with_capacity(4 * order.len());
    let last_advance = hmtx.u16(4 * (usize::from(num_h_metrics) - 1))?;
    for &old in &order {
        let (advance, lsb) = if old < num_h_metrics {
            (
                hmtx.u16(4 * usize::from(old))?,
                hmtx.u16(4 * usize::from(old) + 2)?,
            )
        } else {
            (
                last_advance,
                hmtx.u16(4 * usize::from(num_h_metrics) + 2 * usize::from(old - num_h_metrics))?,
            )
        };
        new_hmtx.extend_from_slice(&advance.to_be_bytes());
        new_hmtx.extend_from_slice(&lsb.to_be_bytes());
    }

    let new_mapped: Vec<(u32, u16)> = mapped
        .iter()
        .map(|&(c, g)| (u32::from(c), new_id[&g]))
        .collect();

    let mut out_tables: BTreeMap<[u8; 4], Vec<u8>> = BTreeMap::new();
    out_tables.insert(*b"OS/2", t[b"OS/2"].to_vec());
    out_tables.insert(*b"cmap", cmap(&new_mapped));
    out_tables.insert(*b"glyf", new_glyf);
    let mut new_head = t[b"head"].to_vec();
    new_head[8..12].copy_from_slice(&[0; 4]); // checkSumAdjustment, set last
    new_head[50..52].copy_from_slice(&1i16.to_be_bytes()); // indexToLocFormat: long
    out_tables.insert(*b"head", new_head);
    let mut new_hhea = t[b"hhea"].to_vec();
    new_hhea[34..36].copy_from_slice(&count.to_be_bytes());
    out_tables.insert(*b"hhea", new_hhea);
    out_tables.insert(*b"hmtx", new_hmtx);
    out_tables.insert(*b"loca", new_loca);
    let mut new_maxp = t[b"maxp"].to_vec();
    new_maxp[4..6].copy_from_slice(&count.to_be_bytes());
    out_tables.insert(*b"maxp", new_maxp);
    out_tables.insert(*b"name", name(t[b"name"])?);
    let mut post = Reader(t[b"post"]).slice(0, 32)?.to_vec();
    post[0..4].copy_from_slice(&0x0003_0000u32.to_be_bytes());
    out_tables.insert(*b"post", post);
    Ok(assemble(&out_tables))
}

/// The offset of the component record after the one at `at`.
fn next_record(glyph: &[u8], at: usize) -> Result<usize, SubsetError> {
    let flags = Reader(glyph).u16(at)?;
    let mut next = at
        + 4
        + if flags & ARG_1_AND_2_ARE_WORDS != 0 {
            4
        } else {
            2
        };
    next += if flags & WE_HAVE_A_SCALE != 0 {
        2
    } else if flags & WE_HAVE_AN_X_AND_Y_SCALE != 0 {
        4
    } else if flags & WE_HAVE_A_TWO_BY_TWO != 0 {
        8
    } else {
        0
    };
    Ok(next)
}

/// A `cmap` with a Windows Unicode BMP subtable (format 4) for the characters
/// up to U+FFFF and, when any lie beyond, a full-repertoire subtable (format 12).
fn cmap(mapped: &[(u32, u16)]) -> Vec<u8> {
    let bmp: Vec<(u16, u16)> = mapped
        .iter()
        .filter_map(|&(c, g)| u16::try_from(c).ok().map(|c| (c, g)))
        .collect();
    let beyond = mapped.iter().any(|&(c, _)| c > 0xFFFF);

    // Format 4, one segment per character, then the required final 0xFFFF segment.
    let seg_count = u16::try_from(bmp.len() + 1).unwrap_or(u16::MAX);
    // 2 × the largest power of two not above segCount (OpenType, cmap format 4).
    let search_range = 2 * (1u16 << seg_count.ilog2());
    let entry_selector = u16::try_from((search_range / 2).trailing_zeros()).unwrap_or(0);
    let mut f4 = Vec::new();
    let length = 16 + 8 * usize::from(seg_count);
    for v in [
        4,
        u16::try_from(length).unwrap_or(u16::MAX),
        0,
        seg_count * 2,
        search_range,
        entry_selector,
        seg_count * 2 - search_range,
    ] {
        f4.extend_from_slice(&v.to_be_bytes());
    }
    for &(c, _) in &bmp {
        f4.extend_from_slice(&c.to_be_bytes());
    }
    f4.extend_from_slice(&0xFFFFu16.to_be_bytes());
    f4.extend_from_slice(&0u16.to_be_bytes()); // reservedPad
    for &(c, _) in &bmp {
        f4.extend_from_slice(&c.to_be_bytes());
    }
    f4.extend_from_slice(&0xFFFFu16.to_be_bytes());
    for &(c, g) in &bmp {
        f4.extend_from_slice(&g.wrapping_sub(c).to_be_bytes()); // idDelta, modulo 65536
    }
    f4.extend_from_slice(&1u16.to_be_bytes());
    for _ in 0..seg_count {
        f4.extend_from_slice(&0u16.to_be_bytes()); // idRangeOffset
    }

    let mut f12 = Vec::new();
    if beyond {
        let groups = u32::try_from(mapped.len()).unwrap_or(u32::MAX);
        f12.extend_from_slice(&12u16.to_be_bytes());
        f12.extend_from_slice(&0u16.to_be_bytes());
        f12.extend_from_slice(&(16 + 12 * groups).to_be_bytes());
        f12.extend_from_slice(&0u32.to_be_bytes());
        f12.extend_from_slice(&groups.to_be_bytes());
        for &(c, g) in mapped {
            f12.extend_from_slice(&c.to_be_bytes());
            f12.extend_from_slice(&c.to_be_bytes());
            f12.extend_from_slice(&u32::from(g).to_be_bytes());
        }
    }

    let subtables: Vec<(u16, u16, &Vec<u8>)> = if beyond {
        vec![(3, 1, &f4), (3, 10, &f12)]
    } else {
        vec![(3, 1, &f4)]
    };
    let mut out = Vec::new();
    out.extend_from_slice(&0u16.to_be_bytes());
    out.extend_from_slice(&u16::try_from(subtables.len()).unwrap_or(0).to_be_bytes());
    let mut offset = 4 + 8 * subtables.len();
    for (platform, encoding, data) in &subtables {
        out.extend_from_slice(&platform.to_be_bytes());
        out.extend_from_slice(&encoding.to_be_bytes());
        out.extend_from_slice(&u32::try_from(offset).unwrap_or(0).to_be_bytes());
        offset += data.len();
    }
    for (_, _, data) in subtables {
        out.extend_from_slice(data);
    }
    out
}

/// The `name` records to keep: copyright (0), family (1), subfamily (2),
/// unique id (3), full name (4), version (5), PostScript name (6), licence
/// description (13) and licence URL (14).
const KEPT_NAMES: [u16; 9] = [0, 1, 2, 3, 4, 5, 6, 13, 14];

/// A version 0 `name` table with only the kept records.
fn name(table: &[u8]) -> Result<Vec<u8>, SubsetError> {
    let r = Reader(table);
    let count = usize::from(r.u16(2)?);
    let storage = usize::from(r.u16(4)?);
    let mut kept = Vec::new();
    for i in 0..count {
        let rec = 6 + 12 * i;
        let name_id = r.u16(rec + 6)?;
        if KEPT_NAMES.contains(&name_id) {
            let (len, off) = (usize::from(r.u16(rec + 8)?), usize::from(r.u16(rec + 10)?));
            let text = r.slice(storage + off, storage + off + len)?;
            kept.push((
                [r.u16(rec)?, r.u16(rec + 2)?, r.u16(rec + 4)?, name_id],
                text,
            ));
        }
    }
    let mut out = Vec::new();
    let header = 6 + 12 * kept.len();
    for v in [
        0,
        u16::try_from(kept.len()).unwrap_or(0),
        u16::try_from(header).unwrap_or(0),
    ] {
        out.extend_from_slice(&v.to_be_bytes());
    }
    let mut strings = Vec::new();
    for (ids, text) in &kept {
        for v in ids {
            out.extend_from_slice(&v.to_be_bytes());
        }
        out.extend_from_slice(&u16::try_from(text.len()).unwrap_or(0).to_be_bytes());
        out.extend_from_slice(&u16::try_from(strings.len()).unwrap_or(0).to_be_bytes());
        strings.extend_from_slice(text);
    }
    out.extend_from_slice(&strings);
    Ok(out)
}

/// The sum of a table's bytes as big-endian 32-bit words, zero-padded.
fn checksum(data: &[u8]) -> u32 {
    data.chunks(4).fold(0u32, |sum, c| {
        let mut w = [0u8; 4];
        w[..c.len()].copy_from_slice(c);
        sum.wrapping_add(u32::from_be_bytes(w))
    })
}

/// The font file: the table directory, then each table padded to four bytes,
/// with the table checksums and `head`'s whole-font adjustment filled in.
fn assemble(tables: &BTreeMap<[u8; 4], Vec<u8>>) -> Vec<u8> {
    let n = u16::try_from(tables.len()).unwrap_or(0);
    // log2 of the largest power of two not above the table count (OpenType, table directory).
    let entry_selector = u16::try_from(n.max(1).ilog2()).unwrap_or(0);
    let search_range = (1u16 << entry_selector) * 16;
    let mut out = Vec::new();
    for v in [
        0x0001u16,
        0x0000,
        n,
        search_range,
        entry_selector,
        n * 16 - search_range,
    ] {
        out.extend_from_slice(&v.to_be_bytes());
    }
    let mut offset = 12 + 16 * tables.len();
    let mut head_at = 0;
    for (tag, data) in tables {
        out.extend_from_slice(tag);
        out.extend_from_slice(&checksum(data).to_be_bytes());
        out.extend_from_slice(&u32::try_from(offset).unwrap_or(0).to_be_bytes());
        out.extend_from_slice(&u32::try_from(data.len()).unwrap_or(0).to_be_bytes());
        if tag == b"head" {
            head_at = offset;
        }
        offset += data.len().div_ceil(4) * 4;
    }
    for data in tables.values() {
        out.extend_from_slice(data);
        while out.len() % 4 != 0 {
            out.push(0);
        }
    }
    let adjustment = 0xB1B0_AFBAu32.wrapping_sub(checksum(&out));
    out[head_at + 8..head_at + 12].copy_from_slice(&adjustment.to_be_bytes());
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::font::{MEDIUM, REGULAR};
    use skrifa::prelude::{GlyphId, LocationRef, Size};

    fn chars(s: &str) -> BTreeSet<char> {
        s.chars().collect()
    }

    #[test]
    fn the_subset_maps_every_character_with_the_same_advance() {
        let text = "Postgres API, 3 replicas: queue";
        for font in [REGULAR, MEDIUM] {
            let sub = subset(font, &chars(text)).unwrap();
            let (a, b) = (FontRef::new(font).unwrap(), FontRef::new(&sub).unwrap());
            let (ma, mb) = (
                a.glyph_metrics(Size::unscaled(), LocationRef::default()),
                b.glyph_metrics(Size::unscaled(), LocationRef::default()),
            );
            for c in text.chars() {
                let (ga, gb) = (
                    a.charmap().map(c).unwrap(),
                    b.charmap()
                        .map(c)
                        .unwrap_or_else(|| panic!("{c:?} unmapped")),
                );
                assert_ne!(gb, GlyphId::new(0), "{c:?}");
                assert_eq!(ma.advance_width(ga), mb.advance_width(gb), "{c:?}");
            }
        }
    }

    #[test]
    fn the_subset_is_small_and_whole() {
        let sub = subset(
            REGULAR,
            &chars("Service Database Queue Cache Storage Users"),
        )
        .unwrap();
        assert!(sub.len() < 12_000, "{} bytes", sub.len());
        assert_eq!(checksum(&sub), 0xB1B0_AFBA, "the whole-font checksum");
        let t = tables(&sub).unwrap();
        assert!(!t.contains_key(b"GPOS") && !t.contains_key(b"fpgm"));
        let glyphs = Reader(t[b"maxp"]).u16(4).unwrap();
        // .notdef, one glyph per character, and the parts of any composites among them.
        let distinct = chars("Service Database Queue Cache Storage Users").len();
        assert!(
            (1 + distinct..=2 * distinct).contains(&usize::from(glyphs)),
            "{glyphs} glyphs for {distinct} characters"
        );
    }

    #[test]
    fn composite_glyphs_bring_their_parts_renumbered() {
        // Accented letters are composites in Geist: a base letter and a mark.
        let text = "éüğşə";
        let sub = subset(REGULAR, &chars(text)).unwrap();
        let t = tables(&sub).unwrap();
        let glyphs = Reader(t[b"maxp"]).u16(4).unwrap();
        let (loca, glyf) = (Reader(t[b"loca"]), Reader(t[b"glyf"]));
        let mut composites = 0;
        for g in 0..usize::from(glyphs) {
            let (from, to) = (
                loca.u32(4 * g).unwrap() as usize,
                loca.u32(4 * g + 4).unwrap() as usize,
            );
            let data = glyf.slice(from, to).unwrap();
            if data.len() >= 10 && Reader(data).i16(0).unwrap() < 0 {
                composites += 1;
                let (parts, _, instructions) = components(data).unwrap();
                assert!(!instructions);
                for (_, part) in parts {
                    assert!(part < glyphs, "part {part} of {glyphs}");
                }
            }
        }
        assert!(composites > 0, "expected composite glyphs among {text}");
        let b = FontRef::new(&sub).unwrap();
        for c in text.chars() {
            assert!(b.charmap().map(c).is_some(), "{c:?}");
        }
    }

    /// Damaged fonts are refused, never a panic: truncated at many points, and
    /// with bytes changed in the tables the subsetter reads. The positions and
    /// values come from a fixed xorshift sequence, so a failure reproduces.
    #[test]
    fn damaged_fonts_are_refused_without_panicking() {
        let text = chars("Postgres API é ş ə");
        for cut in (0..REGULAR.len()).step_by(97) {
            assert!(
                std::panic::catch_unwind(|| subset(&REGULAR[..cut], &text)).is_ok(),
                "truncated at {cut}"
            );
        }
        let t = tables(REGULAR).unwrap();
        let base = REGULAR.as_ptr() as usize;
        let ranges: Vec<(usize, usize)> = [
            *b"head", *b"hhea", *b"maxp", *b"loca", *b"cmap", *b"name", *b"post",
        ]
        .iter()
        .map(|tag| (t[tag].as_ptr() as usize - base, t[tag].len().min(4096)))
        .chain([(0, 12 + 16 * t.len())])
        .collect();
        let mut s: u64 = 0x9E37_79B9_7F4A_7C15;
        for round in 0..4000 {
            let mut data = REGULAR.to_vec();
            for _ in 0..3 {
                s ^= s << 13;
                s ^= s >> 7;
                s ^= s << 17;
                let (off, len) = ranges[usize::try_from(s % ranges.len() as u64).unwrap()];
                let at = off + usize::try_from((s >> 16) % len.max(1) as u64).unwrap();
                data[at] = s.to_be_bytes()[2];
            }
            assert!(
                std::panic::catch_unwind(|| subset(&data, &text)).is_ok(),
                "round {round}"
            );
        }
    }

    #[test]
    fn the_same_text_gives_the_same_bytes() {
        let s = chars("archgram");
        assert_eq!(subset(MEDIUM, &s).unwrap(), subset(MEDIUM, &s).unwrap());
    }

    #[test]
    fn the_licence_travels_with_the_subset() {
        let sub = subset(REGULAR, &chars("a")).unwrap();
        let t = tables(&sub).unwrap();
        let r = Reader(t[b"name"]);
        let ids: BTreeSet<u16> = (0..usize::from(r.u16(2).unwrap()))
            .map(|i| r.u16(6 + 12 * i + 6).unwrap())
            .collect();
        assert!(
            ids.contains(&0) && ids.contains(&13) && ids.contains(&14),
            "{ids:?}"
        );
    }
}
