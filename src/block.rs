#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BlockType {
    Cobblestone,
    Sandstone,
}

pub const NUM_BLOCK_TYPES: usize = 2;

impl std::fmt::Display for BlockType {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                BlockType::Cobblestone => "cobblestone",
                BlockType::Sandstone => "sandstone",
            }
        )
    }
}
