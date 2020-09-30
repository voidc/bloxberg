use std::collections::HashMap;
use std::convert::TryInto;
use std::ops::Range;

#[derive(Debug, Copy, Clone, PartialEq)]
pub enum Format {
    Hex,
    UDec,
    SDec,
    Oct,
    Bin,
    Char,
}

impl Format {
    pub const fn cols_per_byte(&self) -> usize {
        match &self {
            Format::Hex | Format::Char => 1,
            Format::UDec | Format::SDec | Format::Oct => 2,
            Format::Bin => 4,
        }
    }

    pub const fn cycle(&self, rev: bool) -> Self {
        match self {
            Format::Hex if rev => Format::Char,
            Format::Hex => Format::UDec,
            Format::UDec if rev => Format::Hex,
            Format::UDec => Format::SDec,
            Format::SDec if rev => Format::UDec,
            Format::SDec => Format::Oct,
            Format::Oct if rev => Format::SDec,
            Format::Oct => Format::Bin,
            Format::Bin if rev => Format::Oct,
            Format::Bin => Format::Char,
            Format::Char if rev => Format::Bin,
            Format::Char => Format::Hex,
        }
    }

    pub const fn chars_per_byte(&self) -> usize {
        match &self {
            Format::Hex => 2,
            Format::UDec | Format::SDec => 3,
            Format::Oct => 4,
            Format::Bin => 8,
            Format::Char => 1,
        }
    }

    pub fn parse_char(&self, c: char) -> Option<u8> {
        match &self {
            Format::Hex => c.to_digit(16),
            Format::UDec | Format::SDec => c.to_digit(10),
            Format::Oct => c.to_digit(8),
            Format::Bin => c.to_digit(2),
            Format::Char => Some(c as u32),
        }
        .map(|x| x as u8)
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
    #[cfg(target_pointer_width = "64")]
    pub const ADDRESS: Width = Width::DWord64;

    pub const fn n_bytes(&self) -> usize {
        match &self {
            Width::Byte8 => 1,
            Width::HWord16 => 2,
            Width::Word32 => 4,
            Width::DWord64 => 8,
            Width::QWord128 => 16,
        }
    }

    pub const fn align(&self, n: usize) -> usize {
        let shift = match &self {
            Width::Byte8 => 0,
            Width::HWord16 => 1,
            Width::Word32 => 2,
            Width::DWord64 => 3,
            Width::QWord128 => 4,
        };
        (n >> shift) << shift
    }

    pub const fn inc(&self) -> Self {
        match self {
            Width::Byte8 => Width::HWord16,
            Width::HWord16 => Width::Word32,
            Width::Word32 => Width::DWord64,
            Width::DWord64 => Width::QWord128,
            Width::QWord128 => Width::QWord128,
        }
    }

    pub const fn dec(&self) -> Self {
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
    pub const fn toggle(&self) -> Self {
        match self {
            ByteOrder::LittleEndian => ByteOrder::BigEndian,
            ByteOrder::BigEndian => ByteOrder::LittleEndian,
        }
    }
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub struct Cell {
    pub offset: usize,
    pub format: Format,
    pub width: Width,
    pub byte_order: ByteOrder,
}

impl Cell {
    pub const fn new(offset: usize, format: Format, width: Width, byte_order: ByteOrder) -> Self {
        Cell {
            offset,
            format,
            width,
            byte_order,
        }
    }

    pub const fn new_hex(offset: usize) -> Self {
        Self::new(offset, Format::Hex, Width::Byte8, ByteOrder::LittleEndian)
    }

    pub const fn n_bytes(&self) -> usize {
        self.width.n_bytes()
    }

    pub const fn byte_range(&self) -> Range<usize> {
        self.offset..(self.offset + self.n_bytes())
    }

    pub const fn n_cols(&self) -> usize {
        self.format.cols_per_byte() * self.n_bytes()
    }

    pub const fn base_offset(&self) -> usize {
        self.width.align(self.offset)
    }

    pub fn parse_value(&self, data: &[u8]) -> u128 {
        match self.byte_order {
            ByteOrder::LittleEndian => match self.width {
                Width::Byte8 => u8::from_le_bytes(data[..1].try_into().unwrap()).into(),
                Width::HWord16 => u16::from_le_bytes(data[..2].try_into().unwrap()).into(),
                Width::Word32 => u32::from_le_bytes(data[..4].try_into().unwrap()).into(),
                Width::DWord64 => u64::from_le_bytes(data[..8].try_into().unwrap()).into(),
                Width::QWord128 => u128::from_le_bytes(data[..16].try_into().unwrap()).into(),
            },
            ByteOrder::BigEndian => match self.width {
                Width::Byte8 => u8::from_be_bytes(data[..1].try_into().unwrap()).into(),
                Width::HWord16 => u16::from_be_bytes(data[..2].try_into().unwrap()).into(),
                Width::Word32 => u32::from_be_bytes(data[..4].try_into().unwrap()).into(),
                Width::DWord64 => u64::from_be_bytes(data[..8].try_into().unwrap()).into(),
                Width::QWord128 => u128::from_be_bytes(data[..16].try_into().unwrap()).into(),
            },
        }
    }

    pub fn parse_value_signed(&self, data: &[u8]) -> i128 {
        match self.byte_order {
            ByteOrder::LittleEndian => match self.width {
                Width::Byte8 => i8::from_le_bytes(data[..1].try_into().unwrap()).into(),
                Width::HWord16 => i16::from_le_bytes(data[..2].try_into().unwrap()).into(),
                Width::Word32 => i32::from_le_bytes(data[..4].try_into().unwrap()).into(),
                Width::DWord64 => i64::from_le_bytes(data[..8].try_into().unwrap()).into(),
                Width::QWord128 => i128::from_le_bytes(data[..16].try_into().unwrap()).into(),
            },
            ByteOrder::BigEndian => match self.width {
                Width::Byte8 => i8::from_be_bytes(data[..1].try_into().unwrap()).into(),
                Width::HWord16 => i16::from_be_bytes(data[..2].try_into().unwrap()).into(),
                Width::Word32 => i32::from_be_bytes(data[..4].try_into().unwrap()).into(),
                Width::DWord64 => i64::from_be_bytes(data[..8].try_into().unwrap()).into(),
                Width::QWord128 => i128::from_be_bytes(data[..16].try_into().unwrap()).into(),
            },
        }
    }
}

pub struct SparseCells {
    map: HashMap<usize, Cell>,
    len: usize,
}

impl SparseCells {
    pub fn new(len: usize) -> Self {
        SparseCells {
            map: HashMap::default(),
            len,
        }
    }

    pub fn get(&self, index: usize) -> Cell {
        assert!(index < self.len);
        self.map
            .get(&index)
            .cloned()
            .unwrap_or_else(|| Cell::new_hex(index))
    }

    pub fn get_mut(&mut self, index: usize) -> &mut Cell {
        assert!(index < self.len);
        self.map
            .entry(index)
            .or_insert_with(|| Cell::new_hex(index))
    }

    pub fn len(&self) -> usize {
        self.len
    }
}
