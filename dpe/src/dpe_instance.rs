/*++
Licensed under the Apache-2.0 license.

Abstract:
    Defines an instance of DPE and all of its contexts.
--*/
use crate::{
    commands::InitCtxCmd,
    profile,
    response::{DpeErrorCode, GetProfileResp, InitCtxResp},
    MAX_HANDLES,
};

pub struct DpeInstance {
    contexts: [Context; MAX_HANDLES],
}

impl Default for DpeInstance {
    fn default() -> Self {
        Self::new()
    }
}

impl DpeInstance {
    pub fn new() -> DpeInstance {
        const CONTEXT_INITIALIZER: Context = Context::new();
        DpeInstance {
            contexts: [CONTEXT_INITIALIZER; MAX_HANDLES],
        }
    }

    pub fn get_profile(&mut self) -> Result<GetProfileResp, DpeErrorCode> {
        Ok(GetProfileResp { flags: 0 })
    }

    pub fn initialize_context(&mut self, _cmd: &InitCtxCmd) -> Result<InitCtxResp, DpeErrorCode> {
        Ok(InitCtxResp { handle: [0; 20] })
    }
}

#[repr(transparent)]
pub struct TciMeasurement([u8; profile::TCI_SIZE]);

impl Default for TciMeasurement {
    fn default() -> Self {
        Self([0; profile::TCI_SIZE])
    }
}

#[repr(C, align(4))]
#[derive(Default)]
struct TciNodeData {
    tci_type: u32,

    // Bits
    // 31: INTERNAL
    // 30-0: Reserved. Must be zero
    flags: u32,
    tci_cumulative: TciMeasurement,
    tci_current: TciMeasurement,
}

impl TciNodeData {
    const INTERNAL_FLAG_MASK: u32 = 1 << 31;

    const fn flag_is_internal(self) -> bool {
        self.flags & Self::INTERNAL_FLAG_MASK != 0
    }

    fn set_flag_is_internal(&mut self, value: bool) {
        set_flag(&mut self.flags, Self::INTERNAL_FLAG_MASK, value);
    }

    const fn new() -> TciNodeData {
        TciNodeData {
            tci_type: 0,
            flags: 0,
            tci_cumulative: TciMeasurement([0; profile::TCI_SIZE]),
            tci_current: TciMeasurement([0; profile::TCI_SIZE]),
        }
    }
}

#[repr(C, align(4))]
struct Context {
    tci: TciNodeData,
    // Bitmap of the node indices that are children of this node
    children: u32,
    // Index in DPE instance of the parent context. 0xFF if this node is the root
    parent_idx: u8,
    simulation: bool,
    is_active: bool,
}

impl Context {
    const fn new() -> Context {
        Context {
            tci: TciNodeData::new(),
            children: 0,
            parent_idx: 0xFF,
            simulation: false,
            is_active: false,
        }
    }
}

fn set_flag(field: &mut u32, mask: u32, value: bool) {
    *field = if value { *field | mask } else { *field & !mask };
}