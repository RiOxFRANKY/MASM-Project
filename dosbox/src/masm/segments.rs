pub type SegmentId = usize;
pub type GroupId = usize;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Frame {
    Segment(SegmentId),
    Group(GroupId),
}

pub struct Segment {
    pub name: String,
    pub class: String,
    pub align: u32,
    pub stack: bool,
    pub group: Option<GroupId>,
    pub rank: u32,
    pub data: Vec<u8>,
    pub relocations: Vec<(u32, Frame)>,
    pub location: u32,
    pub size: u32,
}

#[derive(Default)]
pub struct Segments {
    pub list: Vec<Segment>,
    pub groups: Vec<String>,
}

#[derive(Clone, Default, PartialEq)]
pub struct Layout {
    pub bases: Vec<u32>,
    pub group_bases: Vec<u32>,
}

impl Segments {
    pub fn find(&self, name: &str) -> Option<SegmentId> {
        self.list.iter().position(|segment| segment.name == name)
    }

    pub fn find_group(&self, name: &str) -> Option<GroupId> {
        self.groups.iter().position(|group| group == name)
    }

    pub fn open(&mut self, name: &str, class: &str, align: u32, stack: bool, rank: u32) -> SegmentId {
        if let Some(id) = self.find(name) {
            return id;
        }
        self.list.push(Segment {
            name: name.to_string(),
            class: class.to_string(),
            align,
            stack,
            group: None,
            rank,
            data: Vec::new(),
            relocations: Vec::new(),
            location: 0,
            size: 0,
        });
        self.list.len() - 1
    }

    pub fn group(&mut self, name: &str) -> GroupId {
        if let Some(id) = self.find_group(name) {
            return id;
        }
        self.groups.push(name.to_string());
        self.groups.len() - 1
    }

    pub fn join(&mut self, segment: SegmentId, group: GroupId) -> Result<(), String> {
        match self.list[segment].group {
            Some(existing) if existing != group => Err(format!("segment {} is already in a group", self.list[segment].name)),
            _ => {
                self.list[segment].group = Some(group);
                Ok(())
            }
        }
    }

    pub fn emit(&mut self, id: SegmentId, bytes: &[u8]) {
        let segment = &mut self.list[id];
        let start = segment.location as usize;
        let end = start + bytes.len();
        if segment.data.len() < end {
            segment.data.resize(end, 0);
        }
        segment.data[start..end].copy_from_slice(bytes);
        segment.location = end as u32;
        segment.size = segment.size.max(segment.location);
    }

    pub fn reserve(&mut self, id: SegmentId, count: u32, fill: u8) {
        let bytes = vec![fill; count as usize];
        self.emit(id, &bytes);
    }

    pub fn frame_of(&self, segment: SegmentId) -> Frame {
        match self.list[segment].group {
            Some(group) => Frame::Group(group),
            None => Frame::Segment(segment),
        }
    }

    pub fn layout(&self) -> Layout {
        let mut order: Vec<SegmentId> = (0..self.list.len()).collect();
        order.sort_by_key(|id| (self.list[*id].rank, *id));
        let mut bases = vec![0u32; self.list.len()];
        let mut cursor = 0u32;
        for id in order {
            let align = self.list[id].align.max(16);
            cursor = cursor.div_ceil(align) * align;
            bases[id] = cursor;
            cursor += self.list[id].size;
        }
        let group_bases = (0..self.groups.len())
            .map(|group| {
                (0..self.list.len())
                    .filter(|id| self.list[*id].group == Some(group))
                    .map(|id| bases[id])
                    .min()
                    .unwrap_or(0)
            })
            .collect();
        Layout { bases, group_bases }
    }

    pub fn ordered(&self) -> Vec<SegmentId> {
        let mut order: Vec<SegmentId> = (0..self.list.len()).collect();
        order.sort_by_key(|id| (self.list[*id].rank, *id));
        order
    }
}

impl Layout {
    pub fn frame_base(&self, frame: Frame) -> u32 {
        match frame {
            Frame::Segment(id) => self.bases.get(id).copied().unwrap_or(0),
            Frame::Group(id) => self.group_bases.get(id).copied().unwrap_or(0),
        }
    }

    pub fn paragraph(&self, frame: Frame) -> u16 {
        (self.frame_base(frame) >> 4) as u16
    }

    pub fn frame_offset(&self, segments: &Segments, segment: SegmentId, local: u32) -> i64 {
        let base = self.bases.get(segment).copied().unwrap_or(0) as i64;
        let frame_base = self.frame_base(segments.frame_of(segment)) as i64;
        base - frame_base + local as i64
    }
}
