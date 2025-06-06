---
applyTo: '**'
---
# Piece and state encoding Instructions

Each board position is encoded using 7 bits, each line from the board is encoded on a 64bit unsigned integer. A whole is board encoded in 9 x u64.
Note: Some stacks are not permitted:
- King+*, *+King: They don't appear in the encoding because the king as a special code.
- Jester+Jester, Commander+Commander: Can be encoded but will never appear in a game.

When describing a piece/stack, either by using their full name or their short notation, the first one is always the one on top of the stack so: Jester+Paladin = J+P: Jester is on top of the stack with the Palading below.

The 7 bits for a piece are interpreted as `C UUU LLL`:
- `C` (1 bit): Color. `0` for Black, `1` for White.
- `UUU` (3 bits): **Top Piece Code**. This represents the piece physically on top in a stack. If there's only a single piece on the square (i.e., no stack), these bits are `0b000`.
- `LLL` (3 bits): **Bottom Piece Code**. This represents the piece physically at the bottom of a stack. If there's only a single piece on the square, these bits encode the type of that single piece.

This structure means:
- For a single piece (e.g., a Guard), its type code is placed in `LLL`, and `UUU` is `0b000`. (Example: Black Guard `0 000 101`)
- For a stacked piece (e.g., Jester on Paladin, written J+P), the Jester's code (top piece) is in `UUU` and the Paladin's code (bottom piece) is in `LLL`. (Example: White Jester+Paladin `1 010 100`)

### Special cases:
- 0b0000000: Empty case
- 0b_111000: King (This encoding implies the King is effectively a single piece, so `UUU` would be `111` and `LLL` would be `000` for the payload part, combined with the color bit `C`).

#### Base pieces
(Codes for `UUU` or `LLL` when they represent a piece type)
- 0b001: Soldier
- 0b010: Jester
- 0b011: Commander
- 0b100: Paladins
- 0b101: Guards
- 0b110: Dragons
- 0b111: Ballista

### Examples
- White Soldier+Commander (Soldier on top, Commander at bottom): `1 001 011` (C=1, UUU=Soldier, LLL=Commander)
- Black Guard (single piece): `0 000 101` (C=0, UUU=000, LLL=Guard)
- Black King: `0 111000` (C=0, payload=111000)
- White Soldier+Soldier (Soldier on top, Soldier at bottom): `1 001 001` (C=1, UUU=Soldier, LLL=Soldier)

### Notes
Note that using this encoding is not using the following codes (these refer to the 6-bit payload `UUULLL`):
- `0bUUU000` where `UUU` is `0b001` through `0b110`. These would imply a stack with a "null" bottom piece, which is invalid. A single piece is `0b000LLL`.
An exception must be thrown if any of these invalid codes are encountered.
