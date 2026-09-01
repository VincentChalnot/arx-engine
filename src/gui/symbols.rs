// Auto-generated 23x16 1-bit UI symbol masks.
// Regenerate with `scripts/gen_symbols.py` (run automatically by `make gui`)
// after editing the .xcf sources in assets/pixel/symbols/. Do not edit by hand.
pub const SYMBOL_W: usize = 23;

pub fn symbol_bits(name: Symbol) -> [u32; 16] {
    match name {
        Symbol::DownArrow => [
            0x020020, 0x0380E0, 0x03E3E0, 0x03FFE0, 0x03FFE0, 0x03FFE0, 0x03FFE0, 0x63FFE3,
            0x3FFFFE, 0x1FFFFC, 0x07FFF0, 0x03FFE0, 0x01FFC0, 0x007F00, 0x003E00, 0x000800,
        ],
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Symbol {
    DownArrow,
}
