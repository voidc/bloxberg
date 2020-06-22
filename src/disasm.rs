use crate::util::cmp_range;
use capstone::prelude::*;
use std::convert::TryInto;
use std::ops::Range;

#[derive(Debug)]
struct Insn {
    byte_range: Range<usize>,
    asm: String,
}

pub struct DisasmView {
    cs: Capstone,
    insns: Vec<Insn>,
}

impl DisasmView {
    pub fn new() -> Self {
        let cs = Capstone::new()
            .x86()
            .mode(arch::x86::ArchMode::Mode64)
            .syntax(arch::x86::ArchSyntax::Intel)
            .detail(true)
            .build()
            .unwrap();
        DisasmView { cs, insns: vec![] }
    }

    pub fn is_enabled(&self) -> bool {
        !self.insns.is_empty()
    }

    pub fn disassemble(&mut self, addr: usize, count: usize, data: &[u8]) {
        self.insns = self
            .cs
            .disasm_count(&data[addr..], addr as u64, count)
            .unwrap()
            .iter()
            .map(|insn| {
                let addr = insn.address() as usize;
                Insn {
                    byte_range: addr..addr + insn.bytes().len(),
                    asm: insn.to_string(),
                }
            })
            .collect();
        eprintln!("{:?}", self.insns);
    }

    pub fn get(&self, cursor_offset: usize, relative_scroll: isize) -> Option<&str> {
        if relative_scroll.abs() as usize > self.insns.len() {
            return None;
        }

        let insn_idx = self
            .insns
            .binary_search_by(|insn| cmp_range(cursor_offset, insn.byte_range.clone()).reverse())
            .ok()?;
        let insn_idx = insn_idx as isize + relative_scroll;
        let insn_idx: usize = insn_idx.try_into().ok()?;
        let insn: &Insn = self.insns.get(insn_idx)?;
        return Some(&insn.asm);
    }
}
