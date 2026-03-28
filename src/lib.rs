//! Rust-native runtime types for tree-sitter generated parsers.
//!
//! This crate provides the type definitions that generated Rust parsers depend on.
//! It mirrors the C `parser.h` ABI as idiomatic Rust types.

#![no_std]

pub type Symbol = u16;
pub type StateId = u16;
pub type FieldId = u16;

pub const BUILTIN_SYM_END: Symbol = 0;
pub const BUILTIN_SYM_ERROR: Symbol = u16::MAX;
pub const SERIALIZATION_BUFFER_SIZE: usize = 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SymbolMetadata {
    pub visible: bool,
    pub named: bool,
    pub supertype: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FieldMapEntry {
    pub field_id: FieldId,
    pub child_index: u8,
    pub inherited: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MapSlice {
    pub index: u16,
    pub length: u16,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CharacterRange {
    pub start: i32,
    pub end: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LexMode {
    pub lex_state: StateId,
    pub external_lex_state: StateId,
    pub reserved_word_set_id: u16,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum ParseActionType {
    Shift = 0,
    Reduce = 1,
    Accept = 2,
    Recover = 3,
}

#[derive(Debug, Clone, Copy)]
pub struct ShiftAction {
    pub state: StateId,
    pub extra: bool,
    pub repetition: bool,
}

#[derive(Debug, Clone, Copy)]
pub struct ReduceAction {
    pub child_count: u8,
    pub symbol: Symbol,
    pub dynamic_precedence: i16,
    pub production_id: u16,
}

#[derive(Debug, Clone, Copy)]
pub enum ParseAction {
    Shift(ShiftAction),
    Reduce(ReduceAction),
    Accept,
    Recover,
}

#[derive(Debug, Clone, Copy)]
pub struct ParseActionEntry {
    pub count: u8,
    pub reusable: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LanguageMetadata {
    pub major_version: u8,
    pub minor_version: u8,
    pub patch_version: u8,
}

/// The interface that generated lex functions use to interact with the lexer.
pub trait Lexer {
    fn lookahead(&self) -> i32;
    fn result_symbol(&self) -> Symbol;
    fn set_result_symbol(&mut self, symbol: Symbol);
    fn advance(&mut self, skip: bool);
    fn mark_end(&mut self);
    fn get_column(&self) -> u32;
    fn is_at_included_range_start(&self) -> bool;
    fn eof(&self) -> bool;
}

/// Type alias for lex functions.
pub type LexFn = fn(&mut dyn Lexer, StateId) -> bool;

/// External scanner vtable for grammars that define custom tokenization.
#[derive(Debug, Clone, Copy)]
pub struct ExternalScanner {
    pub states: &'static [&'static [bool]],
    pub symbol_map: &'static [Symbol],
    pub create: fn() -> *mut u8,
    pub destroy: fn(*mut u8),
    pub scan: fn(*mut u8, &mut dyn Lexer, &[bool]) -> bool,
    pub serialize: fn(*mut u8, &mut [u8; SERIALIZATION_BUFFER_SIZE]) -> u32,
    pub deserialize: fn(*mut u8, &[u8]),
}

/// The main language definition struct, equivalent to the C `TSLanguage`.
///
/// Generated parsers produce a `&'static Language` that contains all
/// parse tables, lex functions, and metadata needed to drive parsing.
pub struct Language {
    pub abi_version: u32,
    pub symbol_count: u32,
    pub alias_count: u32,
    pub token_count: u32,
    pub external_token_count: u32,
    pub state_count: u32,
    pub large_state_count: u32,
    pub production_id_count: u32,
    pub field_count: u32,
    pub max_alias_sequence_length: u16,
    pub max_reserved_word_set_size: u16,
    pub supertype_count: u32,

    /// Flattened parse table: `[LARGE_STATE_COUNT][SYMBOL_COUNT]` of `u16` action indices.
    pub parse_table: &'static [u16],
    /// Compact representation for states with few entries.
    pub small_parse_table: &'static [u16],
    /// Maps `state - large_state_count` to an index into `small_parse_table`.
    pub small_parse_table_map: &'static [u32],
    /// Flat array of `ParseActionEntry` headers followed by `ParseAction` data,
    /// encoded as `u32` values for compact storage.
    pub parse_actions: &'static [u32],

    pub symbol_names: &'static [&'static str],
    pub field_names: &'static [Option<&'static str>],
    pub field_map_slices: &'static [MapSlice],
    pub field_map_entries: &'static [FieldMapEntry],
    pub symbol_metadata: &'static [SymbolMetadata],
    pub public_symbol_map: &'static [Symbol],
    pub alias_map: &'static [u16],
    /// Flattened alias sequences: `[PRODUCTION_ID_COUNT][MAX_ALIAS_SEQUENCE_LENGTH]`.
    pub alias_sequences: &'static [Symbol],

    pub lex_modes: &'static [LexMode],
    pub lex_fn: LexFn,
    pub keyword_lex_fn: Option<LexFn>,
    pub keyword_capture_token: Symbol,

    pub external_scanner: Option<&'static ExternalScanner>,

    pub primary_state_ids: &'static [StateId],
    pub name: &'static str,

    pub reserved_words: &'static [Symbol],

    pub supertype_symbols: &'static [Symbol],
    pub supertype_map_slices: &'static [MapSlice],
    pub supertype_map_entries: &'static [Symbol],

    pub metadata: Option<LanguageMetadata>,
}

/// Binary search over a sorted slice of `CharacterRange` to check membership.
///
/// This is the Rust equivalent of the C `set_contains` inline function from `parser.h`.
#[must_use]
pub fn set_contains(ranges: &[CharacterRange], lookahead: i32) -> bool {
    let mut index = 0usize;
    let mut size = ranges.len();
    while size > 1 {
        let half_size = size / 2;
        let mid_index = index + half_size;
        let range = &ranges[mid_index];
        if lookahead >= range.start && lookahead <= range.end {
            return true;
        } else if lookahead > range.end {
            index = mid_index;
        }
        size -= half_size;
    }
    if let Some(range) = ranges.get(index) {
        lookahead >= range.start && lookahead <= range.end
    } else {
        false
    }
}

// Ensure Language is Sync+Send since it's always 'static data.
unsafe impl Sync for Language {}
unsafe impl Send for Language {}
