use super::assembler::Assembler;
use super::segments::Frame;
use crate::formats::object::{ObjectFile, SegmentInfo, SymbolInfo};

const SEGMENT_LIMIT: u32 = 0x10000;

impl Assembler {
    pub fn build_object(&self, source_name: &str) -> Result<ObjectFile, Vec<String>> {
        let mut problems = Vec::new();
        let layout = &self.layout;
        let segments = &self.segments;
        let mut image_size = 0u32;
        for (id, segment) in segments.list.iter().enumerate() {
            if segment.size > SEGMENT_LIMIT {
                problems.push(format!("segment {} is larger than 64K", segment.name));
            }
            let span = layout.frame_offset(segments, id, segment.size);
            if span > SEGMENT_LIMIT as i64 {
                problems.push(format!("group containing {} is larger than 64K", segment.name));
            }
            image_size = image_size.max(layout.bases[id] + segment.size);
        }
        let mut image = vec![0u8; image_size as usize];
        let mut relocations = Vec::new();
        for (id, segment) in segments.list.iter().enumerate() {
            let base = layout.bases[id] as usize;
            image[base..base + segment.data.len()].copy_from_slice(&segment.data);
            relocations.extend(segment.relocations.iter().map(|(offset, _)| base as u32 + offset));
        }
        relocations.sort_unstable();
        let entry = match &self.entry {
            Some(value) => match value.label {
                Some(label) => Some((label.paragraph, value.number as u16)),
                None => {
                    problems.push("END must name a code label as the entry point".to_string());
                    None
                }
            },
            None => None,
        };
        let stack = segments.list.iter().enumerate().find(|(_, segment)| segment.stack).map(|(id, segment)| {
            let frame = segments.frame_of(id);
            (layout.paragraph(frame), layout.frame_offset(segments, id, segment.size) as u16)
        });
        let group_name = |id: usize| match segments.frame_of(id) {
            Frame::Group(group) => segments.groups[group].clone(),
            Frame::Segment(_) => String::new(),
        };
        let segment_infos = segments
            .ordered()
            .into_iter()
            .map(|id| SegmentInfo {
                name: segments.list[id].name.clone(),
                class: segments.list[id].class.clone(),
                group: group_name(id),
                start: layout.bases[id],
                length: segments.list[id].size,
            })
            .collect();
        let symbols = self
            .symbols
            .labels()
            .into_iter()
            .map(|(name, segment, offset)| {
                let frame = segments.frame_of(segment);
                SymbolInfo {
                    name,
                    segment: layout.paragraph(frame),
                    offset: layout.frame_offset(segments, segment, offset) as u16,
                }
            })
            .collect();
        if !problems.is_empty() {
            return Err(problems);
        }
        Ok(ObjectFile {
            source: source_name.to_string(),
            image,
            relocations,
            entry,
            stack,
            segments: segment_infos,
            symbols,
        })
    }
}
