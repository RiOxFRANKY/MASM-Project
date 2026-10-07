#[derive(Clone, Copy, Debug, PartialEq)]
pub enum TypeKind {
    Byte,
    Word,
    Dword,
    Fword,
    Qword,
    Tbyte,
    Near,
    Far,
}

impl TypeKind {
    pub fn from_name(name: &str) -> Option<TypeKind> {
        Some(match name {
            "BYTE" | "SBYTE" | "DB" => TypeKind::Byte,
            "WORD" | "SWORD" | "DW" => TypeKind::Word,
            "DWORD" | "SDWORD" | "DD" => TypeKind::Dword,
            "FWORD" | "DF" => TypeKind::Fword,
            "QWORD" | "DQ" => TypeKind::Qword,
            "TBYTE" | "DT" => TypeKind::Tbyte,
            "NEAR" => TypeKind::Near,
            "FAR" => TypeKind::Far,
            _ => return None,
        })
    }

    pub fn size(self) -> u32 {
        match self {
            TypeKind::Byte => 1,
            TypeKind::Word | TypeKind::Near => 2,
            TypeKind::Dword | TypeKind::Far => 4,
            TypeKind::Fword => 6,
            TypeKind::Qword => 8,
            TypeKind::Tbyte => 10,
        }
    }

    pub fn type_value(self) -> i64 {
        match self {
            TypeKind::Near => 0xFF02,
            TypeKind::Far => 0xFF05,
            other => other.size() as i64,
        }
    }

    pub fn is_code(self) -> bool {
        matches!(self, TypeKind::Near | TypeKind::Far)
    }
}
