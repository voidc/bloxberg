use std::convert::TryInto;

#[derive(Debug, Copy, Clone, PartialEq)]
pub enum Format {
    Hex,
    Dec,
    SDec,
    Oct,
    Bin,
    Char,
}

impl Format {
    pub fn cols_per_byte(&self) -> usize {
        match &self {
            Format::Hex | Format::Char => 1,
            Format::Dec | Format::SDec | Format::Oct => 2,
            Format::Bin => 4,
        }
    }

    pub fn cycle(&self, rev: bool) -> Self {
        match self {
            Format::Hex if rev => Format::Char,
            Format::Hex => Format::Dec,
            Format::Dec if rev => Format::Hex,
            Format::Dec => Format::SDec,
            Format::SDec if rev => Format::Dec,
            Format::SDec => Format::Oct,
            Format::Oct if rev => Format::SDec,
            Format::Oct => Format::Bin,
            Format::Bin if rev => Format::Oct,
            Format::Bin => Format::Char,
            Format::Char if rev => Format::Bin,
            Format::Char => Format::Hex,
        }
    }
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub enum Width {
    //8, 16 hw, 32 w, 64 dw, 128 qw
    Byte8,
    HWord16,
    Word32,
    DWord64,
    QWord128,
}

impl Width {
    pub fn n_bytes(&self) -> usize {
        match &self {
            Width::Byte8 => 1,
            Width::HWord16 => 2,
            Width::Word32 => 4,
            Width::DWord64 => 8,
            Width::QWord128 => 16,
        }
    }

    pub fn align(&self, n: usize) -> usize {
        let shift = self.n_bytes() - 1;
        (n >> shift) << shift
    }

    pub fn inc(&self) -> Self {
        match self {
            Width::Byte8 => Width::HWord16,
            Width::HWord16 => Width::Word32,
            Width::Word32 => Width::DWord64,
            Width::DWord64 => Width::QWord128,
            Width::QWord128 => Width::QWord128,
        }
    }

    pub fn dec(&self) -> Self {
        match self {
            Width::Byte8 => Width::Byte8,
            Width::HWord16 => Width::Byte8,
            Width::Word32 => Width::HWord16,
            Width::DWord64 => Width::Word32,
            Width::QWord128 => Width::DWord64,
        }
    }
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub enum ByteOrder {
    BigEndian,
    LittleEndian,
}

impl ByteOrder {
    pub fn toggle(&self) -> Self {
        match self {
            ByteOrder::LittleEndian => ByteOrder::BigEndian,
            ByteOrder::BigEndian => ByteOrder::LittleEndian,
        }
    }
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub struct Cell {
    pub offset: usize,
    pub col: usize,
    pub format: Format,
    pub width: Width,
    pub byte_order: ByteOrder,
    pub selected: bool,
}

impl Cell {
    pub fn new(offset: usize, col: usize, format: Format, width: Width, byte_order: ByteOrder) -> Self {
        Cell {
            offset,
            col,
            format,
            width,
            byte_order,
            selected: false,
        }
    }

    pub fn new_hex(offset: usize, col: usize) -> Self {
        Self::new(offset, col, Format::Hex, Width::Byte8, ByteOrder::LittleEndian)
    }

    pub fn n_bytes(&self) -> usize {
        self.width.n_bytes()
    }

    pub fn n_cols(&self) -> usize {
        self.format.cols_per_byte() * self.n_bytes()
    }

    pub fn parse_value(&self, data: &[u8]) -> u128 {
        match self.byte_order {
            ByteOrder::LittleEndian => {
                match self.width {
                    Width::Byte8 => u8::from_le_bytes(data[..1].try_into().unwrap()).into(),
                    Width::HWord16 => u16::from_le_bytes(data[..2].try_into().unwrap()).into(),
                    Width::Word32 => u32::from_le_bytes(data[..4].try_into().unwrap()).into(),
                    Width::DWord64 => u64::from_le_bytes(data[..8].try_into().unwrap()).into(),
                    Width::QWord128 => u128::from_le_bytes(data[..16].try_into().unwrap()).into(),
                }
            },
            ByteOrder::BigEndian => {
                match self.width {
                    Width::Byte8 => u8::from_be_bytes(data[..1].try_into().unwrap()).into(),
                    Width::HWord16 => u16::from_be_bytes(data[..2].try_into().unwrap()).into(),
                    Width::Word32 => u32::from_be_bytes(data[..4].try_into().unwrap()).into(),
                    Width::DWord64 => u64::from_be_bytes(data[..8].try_into().unwrap()).into(),
                    Width::QWord128 => u128::from_be_bytes(data[..16].try_into().unwrap()).into(),
                }
            }
        }
    }

    pub fn parse_value_signed(&self, data: &[u8]) -> i128 {
        match self.byte_order {
            ByteOrder::LittleEndian => {
                match self.width {
                    Width::Byte8 => i8::from_le_bytes(data[..1].try_into().unwrap()).into(),
                    Width::HWord16 => i16::from_le_bytes(data[..2].try_into().unwrap()).into(),
                    Width::Word32 => i32::from_le_bytes(data[..4].try_into().unwrap()).into(),
                    Width::DWord64 => i64::from_le_bytes(data[..8].try_into().unwrap()).into(),
                    Width::QWord128 => i128::from_le_bytes(data[..16].try_into().unwrap()).into(),
                }
            },
            ByteOrder::BigEndian => {
                match self.width {
                    Width::Byte8 => i8::from_be_bytes(data[..1].try_into().unwrap()).into(),
                    Width::HWord16 => i16::from_be_bytes(data[..2].try_into().unwrap()).into(),
                    Width::Word32 => i32::from_be_bytes(data[..4].try_into().unwrap()).into(),
                    Width::DWord64 => i64::from_be_bytes(data[..8].try_into().unwrap()).into(),
                    Width::QWord128 => i128::from_be_bytes(data[..16].try_into().unwrap()).into(),
                }
            }
        }
    }
}