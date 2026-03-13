/// An instruction format
///
/// Every opcode has a corresponding instruction format
/// which is represented by both the `InstructionFormat`
/// and the `InstructionData` enums.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum InstructionFormat {
    /// AtomicCas(imms=(flags: ir::MemFlags), vals=3, blocks=0, raw_blocks=0)
    AtomicCas, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:32
    /// AtomicRmw(imms=(flags: ir::MemFlags, op: ir::AtomicRmwOp), vals=2, blocks=0, raw_blocks=0)
    AtomicRmw, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:32
    /// Binary(imms=(), vals=2, blocks=0, raw_blocks=0)
    Binary, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:32
    /// BinaryImm64(imms=(imm: ir::immediates::Imm64), vals=1, blocks=0, raw_blocks=0)
    BinaryImm64, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:32
    /// BinaryImm8(imms=(imm: ir::immediates::Uimm8), vals=1, blocks=0, raw_blocks=0)
    BinaryImm8, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:32
    /// BranchTable(imms=(table: ir::JumpTable), vals=1, blocks=0, raw_blocks=0)
    BranchTable, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:32
    /// Brif(imms=(), vals=1, blocks=2, raw_blocks=0)
    Brif, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:32
    /// Call(imms=(func_ref: ir::FuncRef), vals=0, blocks=0, raw_blocks=0)
    Call, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:32
    /// CallIndirect(imms=(sig_ref: ir::SigRef), vals=1, blocks=0, raw_blocks=0)
    CallIndirect, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:32
    /// CondTrap(imms=(code: ir::TrapCode), vals=1, blocks=0, raw_blocks=0)
    CondTrap, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:32
    /// DynamicStackLoad(imms=(dynamic_stack_slot: ir::DynamicStackSlot), vals=0, blocks=0, raw_blocks=0)
    DynamicStackLoad, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:32
    /// DynamicStackStore(imms=(dynamic_stack_slot: ir::DynamicStackSlot), vals=1, blocks=0, raw_blocks=0)
    DynamicStackStore, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:32
    /// ExceptionHandlerAddress(imms=(imm: ir::immediates::Imm64), vals=0, blocks=0, raw_blocks=1)
    ExceptionHandlerAddress, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:32
    /// FloatCompare(imms=(cond: ir::condcodes::FloatCC), vals=2, blocks=0, raw_blocks=0)
    FloatCompare, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:32
    /// FuncAddr(imms=(func_ref: ir::FuncRef), vals=0, blocks=0, raw_blocks=0)
    FuncAddr, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:32
    /// IntAddTrap(imms=(code: ir::TrapCode), vals=2, blocks=0, raw_blocks=0)
    IntAddTrap, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:32
    /// IntCompare(imms=(cond: ir::condcodes::IntCC), vals=2, blocks=0, raw_blocks=0)
    IntCompare, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:32
    /// IntCompareImm(imms=(cond: ir::condcodes::IntCC, imm: ir::immediates::Imm64), vals=1, blocks=0, raw_blocks=0)
    IntCompareImm, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:32
    /// Jump(imms=(), vals=0, blocks=1, raw_blocks=0)
    Jump, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:32
    /// Load(imms=(flags: ir::MemFlags, offset: ir::immediates::Offset32), vals=1, blocks=0, raw_blocks=0)
    Load, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:32
    /// LoadNoOffset(imms=(flags: ir::MemFlags), vals=1, blocks=0, raw_blocks=0)
    LoadNoOffset, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:32
    /// MultiAry(imms=(), vals=0, blocks=0, raw_blocks=0)
    MultiAry, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:32
    /// NullAry(imms=(), vals=0, blocks=0, raw_blocks=0)
    NullAry, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:32
    /// Shuffle(imms=(imm: ir::Immediate), vals=2, blocks=0, raw_blocks=0)
    Shuffle, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:32
    /// StackLoad(imms=(stack_slot: ir::StackSlot, offset: ir::immediates::Offset32), vals=0, blocks=0, raw_blocks=0)
    StackLoad, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:32
    /// StackStore(imms=(stack_slot: ir::StackSlot, offset: ir::immediates::Offset32), vals=1, blocks=0, raw_blocks=0)
    StackStore, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:32
    /// Store(imms=(flags: ir::MemFlags, offset: ir::immediates::Offset32), vals=2, blocks=0, raw_blocks=0)
    Store, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:32
    /// StoreNoOffset(imms=(flags: ir::MemFlags), vals=2, blocks=0, raw_blocks=0)
    StoreNoOffset, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:32
    /// Ternary(imms=(), vals=3, blocks=0, raw_blocks=0)
    Ternary, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:32
    /// TernaryImm8(imms=(imm: ir::immediates::Uimm8), vals=2, blocks=0, raw_blocks=0)
    TernaryImm8, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:32
    /// Trap(imms=(code: ir::TrapCode), vals=0, blocks=0, raw_blocks=0)
    Trap, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:32
    /// TryCall(imms=(func_ref: ir::FuncRef, exception: ir::ExceptionTable), vals=0, blocks=0, raw_blocks=0)
    TryCall, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:32
    /// TryCallIndirect(imms=(exception: ir::ExceptionTable), vals=1, blocks=0, raw_blocks=0)
    TryCallIndirect, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:32
    /// Unary(imms=(), vals=1, blocks=0, raw_blocks=0)
    Unary, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:32
    /// UnaryConst(imms=(constant_handle: ir::Constant), vals=0, blocks=0, raw_blocks=0)
    UnaryConst, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:32
    /// UnaryGlobalValue(imms=(global_value: ir::GlobalValue), vals=0, blocks=0, raw_blocks=0)
    UnaryGlobalValue, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:32
    /// UnaryIeee16(imms=(imm: ir::immediates::Ieee16), vals=0, blocks=0, raw_blocks=0)
    UnaryIeee16, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:32
    /// UnaryIeee32(imms=(imm: ir::immediates::Ieee32), vals=0, blocks=0, raw_blocks=0)
    UnaryIeee32, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:32
    /// UnaryIeee64(imms=(imm: ir::immediates::Ieee64), vals=0, blocks=0, raw_blocks=0)
    UnaryIeee64, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:32
    /// UnaryImm(imms=(imm: ir::immediates::Imm64), vals=0, blocks=0, raw_blocks=0)
    UnaryImm, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:32
}

impl<'a> From<&'a InstructionData> for InstructionFormat {
    fn from(inst: &'a InstructionData) -> Self {
        match *inst {
            InstructionData::AtomicCas { .. } => {
                Self::AtomicCas
            }
            InstructionData::AtomicRmw { .. } => {
                Self::AtomicRmw
            }
            InstructionData::Binary { .. } => {
                Self::Binary
            }
            InstructionData::BinaryImm64 { .. } => {
                Self::BinaryImm64
            }
            InstructionData::BinaryImm8 { .. } => {
                Self::BinaryImm8
            }
            InstructionData::BranchTable { .. } => {
                Self::BranchTable
            }
            InstructionData::Brif { .. } => {
                Self::Brif
            }
            InstructionData::Call { .. } => {
                Self::Call
            }
            InstructionData::CallIndirect { .. } => {
                Self::CallIndirect
            }
            InstructionData::CondTrap { .. } => {
                Self::CondTrap
            }
            InstructionData::DynamicStackLoad { .. } => {
                Self::DynamicStackLoad
            }
            InstructionData::DynamicStackStore { .. } => {
                Self::DynamicStackStore
            }
            InstructionData::ExceptionHandlerAddress { .. } => {
                Self::ExceptionHandlerAddress
            }
            InstructionData::FloatCompare { .. } => {
                Self::FloatCompare
            }
            InstructionData::FuncAddr { .. } => {
                Self::FuncAddr
            }
            InstructionData::IntAddTrap { .. } => {
                Self::IntAddTrap
            }
            InstructionData::IntCompare { .. } => {
                Self::IntCompare
            }
            InstructionData::IntCompareImm { .. } => {
                Self::IntCompareImm
            }
            InstructionData::Jump { .. } => {
                Self::Jump
            }
            InstructionData::Load { .. } => {
                Self::Load
            }
            InstructionData::LoadNoOffset { .. } => {
                Self::LoadNoOffset
            }
            InstructionData::MultiAry { .. } => {
                Self::MultiAry
            }
            InstructionData::NullAry { .. } => {
                Self::NullAry
            }
            InstructionData::Shuffle { .. } => {
                Self::Shuffle
            }
            InstructionData::StackLoad { .. } => {
                Self::StackLoad
            }
            InstructionData::StackStore { .. } => {
                Self::StackStore
            }
            InstructionData::Store { .. } => {
                Self::Store
            }
            InstructionData::StoreNoOffset { .. } => {
                Self::StoreNoOffset
            }
            InstructionData::Ternary { .. } => {
                Self::Ternary
            }
            InstructionData::TernaryImm8 { .. } => {
                Self::TernaryImm8
            }
            InstructionData::Trap { .. } => {
                Self::Trap
            }
            InstructionData::TryCall { .. } => {
                Self::TryCall
            }
            InstructionData::TryCallIndirect { .. } => {
                Self::TryCallIndirect
            }
            InstructionData::Unary { .. } => {
                Self::Unary
            }
            InstructionData::UnaryConst { .. } => {
                Self::UnaryConst
            }
            InstructionData::UnaryGlobalValue { .. } => {
                Self::UnaryGlobalValue
            }
            InstructionData::UnaryIeee16 { .. } => {
                Self::UnaryIeee16
            }
            InstructionData::UnaryIeee32 { .. } => {
                Self::UnaryIeee32
            }
            InstructionData::UnaryIeee64 { .. } => {
                Self::UnaryIeee64
            }
            InstructionData::UnaryImm { .. } => {
                Self::UnaryImm
            }
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "enable-serde", derive(Serialize, Deserialize))]
#[allow(missing_docs, reason = "generated code")]
pub enum InstructionData {
    AtomicCas {
        opcode: Opcode,
        args: [Value; 3], // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:76
        flags: ir::MemFlags, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:97
    }
    , // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:100
    AtomicRmw {
        opcode: Opcode,
        args: [Value; 2], // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:76
        flags: ir::MemFlags, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:97
        op: ir::AtomicRmwOp, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:97
    }
    , // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:100
    Binary {
        opcode: Opcode,
        args: [Value; 2], // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:76
    }
    , // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:100
    BinaryImm64 {
        opcode: Opcode,
        arg: Value,
        imm: ir::immediates::Imm64, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:97
    }
    , // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:100
    BinaryImm8 {
        opcode: Opcode,
        arg: Value,
        imm: ir::immediates::Uimm8, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:97
    }
    , // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:100
    BranchTable {
        opcode: Opcode,
        arg: Value,
        table: ir::JumpTable, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:97
    }
    , // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:100
    Brif {
        opcode: Opcode,
        arg: Value,
        blocks: [ir::BlockCall; 2], // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:82
    }
    , // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:100
    Call {
        opcode: Opcode,
        args: ValueList,
        func_ref: ir::FuncRef, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:97
    }
    , // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:100
    CallIndirect {
        opcode: Opcode,
        args: ValueList,
        sig_ref: ir::SigRef, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:97
    }
    , // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:100
    CondTrap {
        opcode: Opcode,
        arg: Value,
        code: ir::TrapCode, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:97
    }
    , // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:100
    DynamicStackLoad {
        opcode: Opcode,
        dynamic_stack_slot: ir::DynamicStackSlot, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:97
    }
    , // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:100
    DynamicStackStore {
        opcode: Opcode,
        arg: Value,
        dynamic_stack_slot: ir::DynamicStackSlot, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:97
    }
    , // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:100
    ExceptionHandlerAddress {
        opcode: Opcode,
        block: ir::Block,
        imm: ir::immediates::Imm64, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:97
    }
    , // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:100
    FloatCompare {
        opcode: Opcode,
        args: [Value; 2], // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:76
        cond: ir::condcodes::FloatCC, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:97
    }
    , // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:100
    FuncAddr {
        opcode: Opcode,
        func_ref: ir::FuncRef, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:97
    }
    , // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:100
    IntAddTrap {
        opcode: Opcode,
        args: [Value; 2], // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:76
        code: ir::TrapCode, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:97
    }
    , // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:100
    IntCompare {
        opcode: Opcode,
        args: [Value; 2], // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:76
        cond: ir::condcodes::IntCC, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:97
    }
    , // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:100
    IntCompareImm {
        opcode: Opcode,
        arg: Value,
        cond: ir::condcodes::IntCC, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:97
        imm: ir::immediates::Imm64, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:97
    }
    , // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:100
    Jump {
        opcode: Opcode,
        destination: ir::BlockCall,
    }
    , // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:100
    Load {
        opcode: Opcode,
        arg: Value,
        flags: ir::MemFlags, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:97
        offset: ir::immediates::Offset32, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:97
    }
    , // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:100
    LoadNoOffset {
        opcode: Opcode,
        arg: Value,
        flags: ir::MemFlags, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:97
    }
    , // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:100
    MultiAry {
        opcode: Opcode,
        args: ValueList,
    }
    , // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:100
    NullAry {
        opcode: Opcode,
    }
    , // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:100
    Shuffle {
        opcode: Opcode,
        args: [Value; 2], // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:76
        imm: ir::Immediate, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:97
    }
    , // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:100
    StackLoad {
        opcode: Opcode,
        stack_slot: ir::StackSlot, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:97
        offset: ir::immediates::Offset32, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:97
    }
    , // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:100
    StackStore {
        opcode: Opcode,
        arg: Value,
        stack_slot: ir::StackSlot, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:97
        offset: ir::immediates::Offset32, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:97
    }
    , // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:100
    Store {
        opcode: Opcode,
        args: [Value; 2], // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:76
        flags: ir::MemFlags, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:97
        offset: ir::immediates::Offset32, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:97
    }
    , // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:100
    StoreNoOffset {
        opcode: Opcode,
        args: [Value; 2], // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:76
        flags: ir::MemFlags, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:97
    }
    , // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:100
    Ternary {
        opcode: Opcode,
        args: [Value; 3], // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:76
    }
    , // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:100
    TernaryImm8 {
        opcode: Opcode,
        args: [Value; 2], // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:76
        imm: ir::immediates::Uimm8, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:97
    }
    , // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:100
    Trap {
        opcode: Opcode,
        code: ir::TrapCode, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:97
    }
    , // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:100
    TryCall {
        opcode: Opcode,
        args: ValueList,
        func_ref: ir::FuncRef, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:97
        exception: ir::ExceptionTable, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:97
    }
    , // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:100
    TryCallIndirect {
        opcode: Opcode,
        args: ValueList,
        exception: ir::ExceptionTable, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:97
    }
    , // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:100
    Unary {
        opcode: Opcode,
        arg: Value,
    }
    , // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:100
    UnaryConst {
        opcode: Opcode,
        constant_handle: ir::Constant, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:97
    }
    , // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:100
    UnaryGlobalValue {
        opcode: Opcode,
        global_value: ir::GlobalValue, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:97
    }
    , // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:100
    UnaryIeee16 {
        opcode: Opcode,
        imm: ir::immediates::Ieee16, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:97
    }
    , // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:100
    UnaryIeee32 {
        opcode: Opcode,
        imm: ir::immediates::Ieee32, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:97
    }
    , // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:100
    UnaryIeee64 {
        opcode: Opcode,
        imm: ir::immediates::Ieee64, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:97
    }
    , // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:100
    UnaryImm {
        opcode: Opcode,
        imm: ir::immediates::Imm64, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:97
    }
    , // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:100
}

impl InstructionData {
    /// Get the opcode of this instruction.
    pub fn opcode(&self) -> Opcode {
        match *self {
            Self::AtomicCas { opcode, .. } |
            Self::AtomicRmw { opcode, .. } |
            Self::Binary { opcode, .. } |
            Self::BinaryImm64 { opcode, .. } |
            Self::BinaryImm8 { opcode, .. } |
            Self::BranchTable { opcode, .. } |
            Self::Brif { opcode, .. } |
            Self::Call { opcode, .. } |
            Self::CallIndirect { opcode, .. } |
            Self::CondTrap { opcode, .. } |
            Self::DynamicStackLoad { opcode, .. } |
            Self::DynamicStackStore { opcode, .. } |
            Self::ExceptionHandlerAddress { opcode, .. } |
            Self::FloatCompare { opcode, .. } |
            Self::FuncAddr { opcode, .. } |
            Self::IntAddTrap { opcode, .. } |
            Self::IntCompare { opcode, .. } |
            Self::IntCompareImm { opcode, .. } |
            Self::Jump { opcode, .. } |
            Self::Load { opcode, .. } |
            Self::LoadNoOffset { opcode, .. } |
            Self::MultiAry { opcode, .. } |
            Self::NullAry { opcode, .. } |
            Self::Shuffle { opcode, .. } |
            Self::StackLoad { opcode, .. } |
            Self::StackStore { opcode, .. } |
            Self::Store { opcode, .. } |
            Self::StoreNoOffset { opcode, .. } |
            Self::Ternary { opcode, .. } |
            Self::TernaryImm8 { opcode, .. } |
            Self::Trap { opcode, .. } |
            Self::TryCall { opcode, .. } |
            Self::TryCallIndirect { opcode, .. } |
            Self::Unary { opcode, .. } |
            Self::UnaryConst { opcode, .. } |
            Self::UnaryGlobalValue { opcode, .. } |
            Self::UnaryIeee16 { opcode, .. } |
            Self::UnaryIeee32 { opcode, .. } |
            Self::UnaryIeee64 { opcode, .. } |
            Self::UnaryImm { opcode, .. } => {
                opcode
            }
        }
    }

    /// Get the controlling type variable operand.
    pub fn typevar_operand(&self, pool: &ir::ValueListPool) -> Option<Value> {
        match *self {
            Self::Call { .. } |
            Self::DynamicStackLoad { .. } |
            Self::ExceptionHandlerAddress { .. } |
            Self::FuncAddr { .. } |
            Self::Jump { .. } |
            Self::MultiAry { .. } |
            Self::NullAry { .. } |
            Self::StackLoad { .. } |
            Self::Trap { .. } |
            Self::TryCall { .. } |
            Self::UnaryConst { .. } |
            Self::UnaryGlobalValue { .. } |
            Self::UnaryIeee16 { .. } |
            Self::UnaryIeee32 { .. } |
            Self::UnaryIeee64 { .. } |
            Self::UnaryImm { .. } => {
                None
            }
            Self::BinaryImm64 { arg, .. } |
            Self::BinaryImm8 { arg, .. } |
            Self::BranchTable { arg, .. } |
            Self::Brif { arg, .. } |
            Self::CondTrap { arg, .. } |
            Self::DynamicStackStore { arg, .. } |
            Self::IntCompareImm { arg, .. } |
            Self::Load { arg, .. } |
            Self::LoadNoOffset { arg, .. } |
            Self::StackStore { arg, .. } |
            Self::Unary { arg, .. } => {
                Some(arg)
            }
            Self::AtomicRmw { args: ref args_arity2, .. } |
            Self::Binary { args: ref args_arity2, .. } |
            Self::FloatCompare { args: ref args_arity2, .. } |
            Self::IntAddTrap { args: ref args_arity2, .. } |
            Self::IntCompare { args: ref args_arity2, .. } |
            Self::Shuffle { args: ref args_arity2, .. } |
            Self::Store { args: ref args_arity2, .. } |
            Self::StoreNoOffset { args: ref args_arity2, .. } |
            Self::TernaryImm8 { args: ref args_arity2, .. } => {
                Some(args_arity2[0])
            }
            Self::Ternary { args: ref args_arity3, .. } => {
                Some(args_arity3[1])
            }
            Self::AtomicCas { args: ref args_arity3, .. } => {
                Some(args_arity3[2])
            }
            Self::CallIndirect { ref args, .. } |
            Self::TryCallIndirect { ref args, .. } => {
                args.get(0, pool)
            }
        }
    }

    /// Get the value arguments to this instruction.
    pub fn arguments<'a>(&'a self, pool: &'a ir::ValueListPool) -> &'a [Value] {
        match *self {
            Self::DynamicStackLoad { .. } |
            Self::ExceptionHandlerAddress { .. } |
            Self::FuncAddr { .. } |
            Self::Jump { .. } |
            Self::NullAry { .. } |
            Self::StackLoad { .. } |
            Self::Trap { .. } |
            Self::UnaryConst { .. } |
            Self::UnaryGlobalValue { .. } |
            Self::UnaryIeee16 { .. } |
            Self::UnaryIeee32 { .. } |
            Self::UnaryIeee64 { .. } |
            Self::UnaryImm { .. } => {
                &[]
            }
            Self::AtomicRmw { args: ref args_arity2, .. } |
            Self::Binary { args: ref args_arity2, .. } |
            Self::FloatCompare { args: ref args_arity2, .. } |
            Self::IntAddTrap { args: ref args_arity2, .. } |
            Self::IntCompare { args: ref args_arity2, .. } |
            Self::Shuffle { args: ref args_arity2, .. } |
            Self::Store { args: ref args_arity2, .. } |
            Self::StoreNoOffset { args: ref args_arity2, .. } |
            Self::TernaryImm8 { args: ref args_arity2, .. } => {
                args_arity2
            }
            Self::AtomicCas { args: ref args_arity3, .. } |
            Self::Ternary { args: ref args_arity3, .. } => {
                args_arity3
            }
            Self::BinaryImm64 { ref arg, .. } |
            Self::BinaryImm8 { ref arg, .. } |
            Self::BranchTable { ref arg, .. } |
            Self::Brif { ref arg, .. } |
            Self::CondTrap { ref arg, .. } |
            Self::DynamicStackStore { ref arg, .. } |
            Self::IntCompareImm { ref arg, .. } |
            Self::Load { ref arg, .. } |
            Self::LoadNoOffset { ref arg, .. } |
            Self::StackStore { ref arg, .. } |
            Self::Unary { ref arg, .. } => {
                core::slice::from_ref(arg)
            }
            Self::Call { ref args, .. } |
            Self::CallIndirect { ref args, .. } |
            Self::MultiAry { ref args, .. } |
            Self::TryCall { ref args, .. } |
            Self::TryCallIndirect { ref args, .. } => {
                args.as_slice(pool)
            }
        }
    }

    /// Get mutable references to the value arguments to this
    /// instruction.
    pub fn arguments_mut<'a>(&'a mut self, pool: &'a mut ir::ValueListPool) -> &'a mut [Value] {
        match *self {
            Self::DynamicStackLoad { .. } |
            Self::ExceptionHandlerAddress { .. } |
            Self::FuncAddr { .. } |
            Self::Jump { .. } |
            Self::NullAry { .. } |
            Self::StackLoad { .. } |
            Self::Trap { .. } |
            Self::UnaryConst { .. } |
            Self::UnaryGlobalValue { .. } |
            Self::UnaryIeee16 { .. } |
            Self::UnaryIeee32 { .. } |
            Self::UnaryIeee64 { .. } |
            Self::UnaryImm { .. } => {
                &mut []
            }
            Self::AtomicRmw { args: ref mut args_arity2, .. } |
            Self::Binary { args: ref mut args_arity2, .. } |
            Self::FloatCompare { args: ref mut args_arity2, .. } |
            Self::IntAddTrap { args: ref mut args_arity2, .. } |
            Self::IntCompare { args: ref mut args_arity2, .. } |
            Self::Shuffle { args: ref mut args_arity2, .. } |
            Self::Store { args: ref mut args_arity2, .. } |
            Self::StoreNoOffset { args: ref mut args_arity2, .. } |
            Self::TernaryImm8 { args: ref mut args_arity2, .. } => {
                args_arity2
            }
            Self::AtomicCas { args: ref mut args_arity3, .. } |
            Self::Ternary { args: ref mut args_arity3, .. } => {
                args_arity3
            }
            Self::BinaryImm64 { ref mut arg, .. } |
            Self::BinaryImm8 { ref mut arg, .. } |
            Self::BranchTable { ref mut arg, .. } |
            Self::Brif { ref mut arg, .. } |
            Self::CondTrap { ref mut arg, .. } |
            Self::DynamicStackStore { ref mut arg, .. } |
            Self::IntCompareImm { ref mut arg, .. } |
            Self::Load { ref mut arg, .. } |
            Self::LoadNoOffset { ref mut arg, .. } |
            Self::StackStore { ref mut arg, .. } |
            Self::Unary { ref mut arg, .. } => {
                core::slice::from_mut(arg)
            }
            Self::Call { ref mut args, .. } |
            Self::CallIndirect { ref mut args, .. } |
            Self::MultiAry { ref mut args, .. } |
            Self::TryCall { ref mut args, .. } |
            Self::TryCallIndirect { ref mut args, .. } => {
                args.as_mut_slice(pool)
            }
        }
    }

    /// Compare two `InstructionData` for equality.
    ///
    /// This operation requires a reference to a `ValueListPool` to
    /// determine if the contents of any `ValueLists` are equal.
    ///
    /// This operation takes a closure that is allowed to map each
    /// argument value to some other value before the instructions
    /// are compared. This allows various forms of canonicalization.
    pub fn eq(&self, other: &Self, pool: &ir::ValueListPool) -> bool {
        if ::core::mem::discriminant(self) != ::core::mem::discriminant(other) {
            return false;
        }
        match (self, other) {
            (&Self::AtomicCas { opcode: ref opcode1, args: ref args1, flags: ref flags1 }, &Self::AtomicCas { opcode: ref opcode2, args: ref args2, flags: ref flags2 }) =>  {
                opcode1 == opcode2
                && flags1 == flags2 // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:276
                && args1.iter().zip(args2.iter()).all(|(a, b)| a == b) // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:279
            }
            (&Self::AtomicRmw { opcode: ref opcode1, args: ref args1, flags: ref flags1, op: ref op1 }, &Self::AtomicRmw { opcode: ref opcode2, args: ref args2, flags: ref flags2, op: ref op2 }) =>  {
                opcode1 == opcode2
                && flags1 == flags2 // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:276
                && op1 == op2 // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:276
                && args1.iter().zip(args2.iter()).all(|(a, b)| a == b) // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:279
            }
            (&Self::Binary { opcode: ref opcode1, args: ref args1 }, &Self::Binary { opcode: ref opcode2, args: ref args2 }) =>  {
                opcode1 == opcode2
                && args1.iter().zip(args2.iter()).all(|(a, b)| a == b) // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:279
            }
            (&Self::BinaryImm64 { opcode: ref opcode1, arg: ref arg1, imm: ref imm1 }, &Self::BinaryImm64 { opcode: ref opcode2, arg: ref arg2, imm: ref imm2 }) =>  {
                opcode1 == opcode2
                && imm1 == imm2 // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:276
                && arg1 == arg2 // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:279
            }
            (&Self::BinaryImm8 { opcode: ref opcode1, arg: ref arg1, imm: ref imm1 }, &Self::BinaryImm8 { opcode: ref opcode2, arg: ref arg2, imm: ref imm2 }) =>  {
                opcode1 == opcode2
                && imm1 == imm2 // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:276
                && arg1 == arg2 // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:279
            }
            (&Self::BranchTable { opcode: ref opcode1, arg: ref arg1, table: ref table1 }, &Self::BranchTable { opcode: ref opcode2, arg: ref arg2, table: ref table2 }) =>  {
                opcode1 == opcode2
                && table1 == table2 // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:276
                && arg1 == arg2 // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:279
            }
            (&Self::Brif { opcode: ref opcode1, arg: ref arg1, blocks: ref blocks1 }, &Self::Brif { opcode: ref opcode2, arg: ref arg2, blocks: ref blocks2 }) =>  {
                opcode1 == opcode2
                && arg1 == arg2 // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:279
                && blocks1.iter().zip(blocks2.iter()).all(|(a, b)| a.block(pool) == b.block(pool)) // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:282
            }
            (&Self::Call { opcode: ref opcode1, args: ref args1, func_ref: ref func_ref1 }, &Self::Call { opcode: ref opcode2, args: ref args2, func_ref: ref func_ref2 }) =>  {
                opcode1 == opcode2
                && func_ref1 == func_ref2 // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:276
                && args1.as_slice(pool).iter().zip(args2.as_slice(pool).iter()).all(|(a, b)| a == b) // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:279
            }
            (&Self::CallIndirect { opcode: ref opcode1, args: ref args1, sig_ref: ref sig_ref1 }, &Self::CallIndirect { opcode: ref opcode2, args: ref args2, sig_ref: ref sig_ref2 }) =>  {
                opcode1 == opcode2
                && sig_ref1 == sig_ref2 // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:276
                && args1.as_slice(pool).iter().zip(args2.as_slice(pool).iter()).all(|(a, b)| a == b) // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:279
            }
            (&Self::CondTrap { opcode: ref opcode1, arg: ref arg1, code: ref code1 }, &Self::CondTrap { opcode: ref opcode2, arg: ref arg2, code: ref code2 }) =>  {
                opcode1 == opcode2
                && code1 == code2 // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:276
                && arg1 == arg2 // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:279
            }
            (&Self::DynamicStackLoad { opcode: ref opcode1, dynamic_stack_slot: ref dynamic_stack_slot1 }, &Self::DynamicStackLoad { opcode: ref opcode2, dynamic_stack_slot: ref dynamic_stack_slot2 }) =>  {
                opcode1 == opcode2
                && dynamic_stack_slot1 == dynamic_stack_slot2 // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:276
            }
            (&Self::DynamicStackStore { opcode: ref opcode1, arg: ref arg1, dynamic_stack_slot: ref dynamic_stack_slot1 }, &Self::DynamicStackStore { opcode: ref opcode2, arg: ref arg2, dynamic_stack_slot: ref dynamic_stack_slot2 }) =>  {
                opcode1 == opcode2
                && dynamic_stack_slot1 == dynamic_stack_slot2 // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:276
                && arg1 == arg2 // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:279
            }
            (&Self::ExceptionHandlerAddress { opcode: ref opcode1, block: ref block1, imm: ref imm1 }, &Self::ExceptionHandlerAddress { opcode: ref opcode2, block: ref block2, imm: ref imm2 }) =>  {
                opcode1 == opcode2
                && imm1 == imm2 // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:276
                && block1 == block2 // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:285
            }
            (&Self::FloatCompare { opcode: ref opcode1, args: ref args1, cond: ref cond1 }, &Self::FloatCompare { opcode: ref opcode2, args: ref args2, cond: ref cond2 }) =>  {
                opcode1 == opcode2
                && cond1 == cond2 // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:276
                && args1.iter().zip(args2.iter()).all(|(a, b)| a == b) // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:279
            }
            (&Self::FuncAddr { opcode: ref opcode1, func_ref: ref func_ref1 }, &Self::FuncAddr { opcode: ref opcode2, func_ref: ref func_ref2 }) =>  {
                opcode1 == opcode2
                && func_ref1 == func_ref2 // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:276
            }
            (&Self::IntAddTrap { opcode: ref opcode1, args: ref args1, code: ref code1 }, &Self::IntAddTrap { opcode: ref opcode2, args: ref args2, code: ref code2 }) =>  {
                opcode1 == opcode2
                && code1 == code2 // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:276
                && args1.iter().zip(args2.iter()).all(|(a, b)| a == b) // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:279
            }
            (&Self::IntCompare { opcode: ref opcode1, args: ref args1, cond: ref cond1 }, &Self::IntCompare { opcode: ref opcode2, args: ref args2, cond: ref cond2 }) =>  {
                opcode1 == opcode2
                && cond1 == cond2 // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:276
                && args1.iter().zip(args2.iter()).all(|(a, b)| a == b) // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:279
            }
            (&Self::IntCompareImm { opcode: ref opcode1, arg: ref arg1, cond: ref cond1, imm: ref imm1 }, &Self::IntCompareImm { opcode: ref opcode2, arg: ref arg2, cond: ref cond2, imm: ref imm2 }) =>  {
                opcode1 == opcode2
                && cond1 == cond2 // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:276
                && imm1 == imm2 // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:276
                && arg1 == arg2 // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:279
            }
            (&Self::Jump { opcode: ref opcode1, destination: ref destination1 }, &Self::Jump { opcode: ref opcode2, destination: ref destination2 }) =>  {
                opcode1 == opcode2
                && destination1 == destination2 // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:282
            }
            (&Self::Load { opcode: ref opcode1, arg: ref arg1, flags: ref flags1, offset: ref offset1 }, &Self::Load { opcode: ref opcode2, arg: ref arg2, flags: ref flags2, offset: ref offset2 }) =>  {
                opcode1 == opcode2
                && flags1 == flags2 // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:276
                && offset1 == offset2 // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:276
                && arg1 == arg2 // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:279
            }
            (&Self::LoadNoOffset { opcode: ref opcode1, arg: ref arg1, flags: ref flags1 }, &Self::LoadNoOffset { opcode: ref opcode2, arg: ref arg2, flags: ref flags2 }) =>  {
                opcode1 == opcode2
                && flags1 == flags2 // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:276
                && arg1 == arg2 // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:279
            }
            (&Self::MultiAry { opcode: ref opcode1, args: ref args1 }, &Self::MultiAry { opcode: ref opcode2, args: ref args2 }) =>  {
                opcode1 == opcode2
                && args1.as_slice(pool).iter().zip(args2.as_slice(pool).iter()).all(|(a, b)| a == b) // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:279
            }
            (&Self::NullAry { opcode: ref opcode1 }, &Self::NullAry { opcode: ref opcode2 }) =>  {
                opcode1 == opcode2
            }
            (&Self::Shuffle { opcode: ref opcode1, args: ref args1, imm: ref imm1 }, &Self::Shuffle { opcode: ref opcode2, args: ref args2, imm: ref imm2 }) =>  {
                opcode1 == opcode2
                && imm1 == imm2 // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:276
                && args1.iter().zip(args2.iter()).all(|(a, b)| a == b) // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:279
            }
            (&Self::StackLoad { opcode: ref opcode1, stack_slot: ref stack_slot1, offset: ref offset1 }, &Self::StackLoad { opcode: ref opcode2, stack_slot: ref stack_slot2, offset: ref offset2 }) =>  {
                opcode1 == opcode2
                && stack_slot1 == stack_slot2 // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:276
                && offset1 == offset2 // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:276
            }
            (&Self::StackStore { opcode: ref opcode1, arg: ref arg1, stack_slot: ref stack_slot1, offset: ref offset1 }, &Self::StackStore { opcode: ref opcode2, arg: ref arg2, stack_slot: ref stack_slot2, offset: ref offset2 }) =>  {
                opcode1 == opcode2
                && stack_slot1 == stack_slot2 // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:276
                && offset1 == offset2 // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:276
                && arg1 == arg2 // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:279
            }
            (&Self::Store { opcode: ref opcode1, args: ref args1, flags: ref flags1, offset: ref offset1 }, &Self::Store { opcode: ref opcode2, args: ref args2, flags: ref flags2, offset: ref offset2 }) =>  {
                opcode1 == opcode2
                && flags1 == flags2 // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:276
                && offset1 == offset2 // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:276
                && args1.iter().zip(args2.iter()).all(|(a, b)| a == b) // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:279
            }
            (&Self::StoreNoOffset { opcode: ref opcode1, args: ref args1, flags: ref flags1 }, &Self::StoreNoOffset { opcode: ref opcode2, args: ref args2, flags: ref flags2 }) =>  {
                opcode1 == opcode2
                && flags1 == flags2 // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:276
                && args1.iter().zip(args2.iter()).all(|(a, b)| a == b) // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:279
            }
            (&Self::Ternary { opcode: ref opcode1, args: ref args1 }, &Self::Ternary { opcode: ref opcode2, args: ref args2 }) =>  {
                opcode1 == opcode2
                && args1.iter().zip(args2.iter()).all(|(a, b)| a == b) // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:279
            }
            (&Self::TernaryImm8 { opcode: ref opcode1, args: ref args1, imm: ref imm1 }, &Self::TernaryImm8 { opcode: ref opcode2, args: ref args2, imm: ref imm2 }) =>  {
                opcode1 == opcode2
                && imm1 == imm2 // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:276
                && args1.iter().zip(args2.iter()).all(|(a, b)| a == b) // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:279
            }
            (&Self::Trap { opcode: ref opcode1, code: ref code1 }, &Self::Trap { opcode: ref opcode2, code: ref code2 }) =>  {
                opcode1 == opcode2
                && code1 == code2 // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:276
            }
            (&Self::TryCall { opcode: ref opcode1, args: ref args1, func_ref: ref func_ref1, exception: ref exception1 }, &Self::TryCall { opcode: ref opcode2, args: ref args2, func_ref: ref func_ref2, exception: ref exception2 }) =>  {
                opcode1 == opcode2
                && func_ref1 == func_ref2 // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:276
                && exception1 == exception2 // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:276
                && args1.as_slice(pool).iter().zip(args2.as_slice(pool).iter()).all(|(a, b)| a == b) // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:279
            }
            (&Self::TryCallIndirect { opcode: ref opcode1, args: ref args1, exception: ref exception1 }, &Self::TryCallIndirect { opcode: ref opcode2, args: ref args2, exception: ref exception2 }) =>  {
                opcode1 == opcode2
                && exception1 == exception2 // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:276
                && args1.as_slice(pool).iter().zip(args2.as_slice(pool).iter()).all(|(a, b)| a == b) // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:279
            }
            (&Self::Unary { opcode: ref opcode1, arg: ref arg1 }, &Self::Unary { opcode: ref opcode2, arg: ref arg2 }) =>  {
                opcode1 == opcode2
                && arg1 == arg2 // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:279
            }
            (&Self::UnaryConst { opcode: ref opcode1, constant_handle: ref constant_handle1 }, &Self::UnaryConst { opcode: ref opcode2, constant_handle: ref constant_handle2 }) =>  {
                opcode1 == opcode2
                && constant_handle1 == constant_handle2 // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:276
            }
            (&Self::UnaryGlobalValue { opcode: ref opcode1, global_value: ref global_value1 }, &Self::UnaryGlobalValue { opcode: ref opcode2, global_value: ref global_value2 }) =>  {
                opcode1 == opcode2
                && global_value1 == global_value2 // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:276
            }
            (&Self::UnaryIeee16 { opcode: ref opcode1, imm: ref imm1 }, &Self::UnaryIeee16 { opcode: ref opcode2, imm: ref imm2 }) =>  {
                opcode1 == opcode2
                && imm1 == imm2 // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:276
            }
            (&Self::UnaryIeee32 { opcode: ref opcode1, imm: ref imm1 }, &Self::UnaryIeee32 { opcode: ref opcode2, imm: ref imm2 }) =>  {
                opcode1 == opcode2
                && imm1 == imm2 // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:276
            }
            (&Self::UnaryIeee64 { opcode: ref opcode1, imm: ref imm1 }, &Self::UnaryIeee64 { opcode: ref opcode2, imm: ref imm2 }) =>  {
                opcode1 == opcode2
                && imm1 == imm2 // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:276
            }
            (&Self::UnaryImm { opcode: ref opcode1, imm: ref imm1 }, &Self::UnaryImm { opcode: ref opcode2, imm: ref imm2 }) =>  {
                opcode1 == opcode2
                && imm1 == imm2 // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:276
            }
            _ => unreachable!()
        }
    }

    /// Hash an `InstructionData`.
    ///
    /// This operation requires a reference to a `ValueListPool` to
    /// hash the contents of any `ValueLists`.
    ///
    /// This operation takes a closure that is allowed to map each
    /// argument value to some other value before it is hashed. This
    /// allows various forms of canonicalization.
    pub fn hash<H: ::core::hash::Hasher>(&self, state: &mut H, pool: &ir::ValueListPool) {
        match *self {
            Self::AtomicCas{opcode, ref args, flags} =>  {
                ::core::hash::Hash::hash( &::core::mem::discriminant(self), state);
                ::core::hash::Hash::hash(&opcode, state);
                ::core::hash::Hash::hash(&flags, state); // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:353
                ::core::hash::Hash::hash(&args.len(), state); // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:355
                for &arg in args {
                    ::core::hash::Hash::hash(&arg, state); // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:358
                }
            }
            Self::AtomicRmw{opcode, ref args, flags, op} =>  {
                ::core::hash::Hash::hash( &::core::mem::discriminant(self), state);
                ::core::hash::Hash::hash(&opcode, state);
                ::core::hash::Hash::hash(&flags, state); // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:353
                ::core::hash::Hash::hash(&op, state); // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:353
                ::core::hash::Hash::hash(&args.len(), state); // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:355
                for &arg in args {
                    ::core::hash::Hash::hash(&arg, state); // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:358
                }
            }
            Self::Binary{opcode, ref args} =>  {
                ::core::hash::Hash::hash( &::core::mem::discriminant(self), state);
                ::core::hash::Hash::hash(&opcode, state);
                ::core::hash::Hash::hash(&args.len(), state); // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:355
                for &arg in args {
                    ::core::hash::Hash::hash(&arg, state); // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:358
                }
            }
            Self::BinaryImm64{opcode, ref arg, imm} =>  {
                ::core::hash::Hash::hash( &::core::mem::discriminant(self), state);
                ::core::hash::Hash::hash(&opcode, state);
                ::core::hash::Hash::hash(&imm, state); // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:353
                ::core::hash::Hash::hash(&1, state); // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:355
                for &arg in core::slice::from_ref(arg) {
                    ::core::hash::Hash::hash(&arg, state); // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:358
                }
            }
            Self::BinaryImm8{opcode, ref arg, imm} =>  {
                ::core::hash::Hash::hash( &::core::mem::discriminant(self), state);
                ::core::hash::Hash::hash(&opcode, state);
                ::core::hash::Hash::hash(&imm, state); // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:353
                ::core::hash::Hash::hash(&1, state); // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:355
                for &arg in core::slice::from_ref(arg) {
                    ::core::hash::Hash::hash(&arg, state); // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:358
                }
            }
            Self::BranchTable{opcode, ref arg, table} =>  {
                ::core::hash::Hash::hash( &::core::mem::discriminant(self), state);
                ::core::hash::Hash::hash(&opcode, state);
                ::core::hash::Hash::hash(&table, state); // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:353
                ::core::hash::Hash::hash(&1, state); // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:355
                for &arg in core::slice::from_ref(arg) {
                    ::core::hash::Hash::hash(&arg, state); // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:358
                }
            }
            Self::Brif{opcode, ref arg, ref blocks} =>  {
                ::core::hash::Hash::hash( &::core::mem::discriminant(self), state);
                ::core::hash::Hash::hash(&opcode, state);
                ::core::hash::Hash::hash(&1, state); // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:355
                for &arg in core::slice::from_ref(arg) {
                    ::core::hash::Hash::hash(&arg, state); // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:358
                }
                ::core::hash::Hash::hash(&blocks.len(), state); // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:363
                for &block in blocks {
                    ::core::hash::Hash::hash(&block.block(pool), state); // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:365
                    for arg in block.args(pool) {
                        ::core::hash::Hash::hash(&arg, state); // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:367
                    }
                }
            }
            Self::Call{opcode, ref args, func_ref} =>  {
                ::core::hash::Hash::hash( &::core::mem::discriminant(self), state);
                ::core::hash::Hash::hash(&opcode, state);
                ::core::hash::Hash::hash(&func_ref, state); // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:353
                ::core::hash::Hash::hash(&args.len(pool), state); // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:355
                for &arg in args.as_slice(pool) {
                    ::core::hash::Hash::hash(&arg, state); // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:358
                }
            }
            Self::CallIndirect{opcode, ref args, sig_ref} =>  {
                ::core::hash::Hash::hash( &::core::mem::discriminant(self), state);
                ::core::hash::Hash::hash(&opcode, state);
                ::core::hash::Hash::hash(&sig_ref, state); // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:353
                ::core::hash::Hash::hash(&args.len(pool), state); // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:355
                for &arg in args.as_slice(pool) {
                    ::core::hash::Hash::hash(&arg, state); // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:358
                }
            }
            Self::CondTrap{opcode, ref arg, code} =>  {
                ::core::hash::Hash::hash( &::core::mem::discriminant(self), state);
                ::core::hash::Hash::hash(&opcode, state);
                ::core::hash::Hash::hash(&code, state); // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:353
                ::core::hash::Hash::hash(&1, state); // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:355
                for &arg in core::slice::from_ref(arg) {
                    ::core::hash::Hash::hash(&arg, state); // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:358
                }
            }
            Self::DynamicStackLoad{opcode, dynamic_stack_slot} =>  {
                ::core::hash::Hash::hash( &::core::mem::discriminant(self), state);
                ::core::hash::Hash::hash(&opcode, state);
                ::core::hash::Hash::hash(&dynamic_stack_slot, state); // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:353
                ::core::hash::Hash::hash(&0, state); // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:355
            }
            Self::DynamicStackStore{opcode, ref arg, dynamic_stack_slot} =>  {
                ::core::hash::Hash::hash( &::core::mem::discriminant(self), state);
                ::core::hash::Hash::hash(&opcode, state);
                ::core::hash::Hash::hash(&dynamic_stack_slot, state); // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:353
                ::core::hash::Hash::hash(&1, state); // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:355
                for &arg in core::slice::from_ref(arg) {
                    ::core::hash::Hash::hash(&arg, state); // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:358
                }
            }
            Self::ExceptionHandlerAddress{opcode, block, imm} =>  {
                ::core::hash::Hash::hash( &::core::mem::discriminant(self), state);
                ::core::hash::Hash::hash(&opcode, state);
                ::core::hash::Hash::hash(&imm, state); // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:353
                ::core::hash::Hash::hash(&0, state); // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:355
                ::core::hash::Hash::hash(&block, state); // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:373
            }
            Self::FloatCompare{opcode, ref args, cond} =>  {
                ::core::hash::Hash::hash( &::core::mem::discriminant(self), state);
                ::core::hash::Hash::hash(&opcode, state);
                ::core::hash::Hash::hash(&cond, state); // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:353
                ::core::hash::Hash::hash(&args.len(), state); // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:355
                for &arg in args {
                    ::core::hash::Hash::hash(&arg, state); // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:358
                }
            }
            Self::FuncAddr{opcode, func_ref} =>  {
                ::core::hash::Hash::hash( &::core::mem::discriminant(self), state);
                ::core::hash::Hash::hash(&opcode, state);
                ::core::hash::Hash::hash(&func_ref, state); // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:353
                ::core::hash::Hash::hash(&0, state); // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:355
            }
            Self::IntAddTrap{opcode, ref args, code} =>  {
                ::core::hash::Hash::hash( &::core::mem::discriminant(self), state);
                ::core::hash::Hash::hash(&opcode, state);
                ::core::hash::Hash::hash(&code, state); // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:353
                ::core::hash::Hash::hash(&args.len(), state); // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:355
                for &arg in args {
                    ::core::hash::Hash::hash(&arg, state); // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:358
                }
            }
            Self::IntCompare{opcode, ref args, cond} =>  {
                ::core::hash::Hash::hash( &::core::mem::discriminant(self), state);
                ::core::hash::Hash::hash(&opcode, state);
                ::core::hash::Hash::hash(&cond, state); // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:353
                ::core::hash::Hash::hash(&args.len(), state); // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:355
                for &arg in args {
                    ::core::hash::Hash::hash(&arg, state); // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:358
                }
            }
            Self::IntCompareImm{opcode, ref arg, cond, imm} =>  {
                ::core::hash::Hash::hash( &::core::mem::discriminant(self), state);
                ::core::hash::Hash::hash(&opcode, state);
                ::core::hash::Hash::hash(&cond, state); // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:353
                ::core::hash::Hash::hash(&imm, state); // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:353
                ::core::hash::Hash::hash(&1, state); // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:355
                for &arg in core::slice::from_ref(arg) {
                    ::core::hash::Hash::hash(&arg, state); // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:358
                }
            }
            Self::Jump{opcode, ref destination} =>  {
                ::core::hash::Hash::hash( &::core::mem::discriminant(self), state);
                ::core::hash::Hash::hash(&opcode, state);
                ::core::hash::Hash::hash(&0, state); // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:355
                ::core::hash::Hash::hash(&1, state); // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:363
                for &block in core::slice::from_ref(destination) {
                    ::core::hash::Hash::hash(&block.block(pool), state); // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:365
                    for arg in block.args(pool) {
                        ::core::hash::Hash::hash(&arg, state); // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:367
                    }
                }
            }
            Self::Load{opcode, ref arg, flags, offset} =>  {
                ::core::hash::Hash::hash( &::core::mem::discriminant(self), state);
                ::core::hash::Hash::hash(&opcode, state);
                ::core::hash::Hash::hash(&flags, state); // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:353
                ::core::hash::Hash::hash(&offset, state); // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:353
                ::core::hash::Hash::hash(&1, state); // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:355
                for &arg in core::slice::from_ref(arg) {
                    ::core::hash::Hash::hash(&arg, state); // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:358
                }
            }
            Self::LoadNoOffset{opcode, ref arg, flags} =>  {
                ::core::hash::Hash::hash( &::core::mem::discriminant(self), state);
                ::core::hash::Hash::hash(&opcode, state);
                ::core::hash::Hash::hash(&flags, state); // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:353
                ::core::hash::Hash::hash(&1, state); // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:355
                for &arg in core::slice::from_ref(arg) {
                    ::core::hash::Hash::hash(&arg, state); // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:358
                }
            }
            Self::MultiAry{opcode, ref args} =>  {
                ::core::hash::Hash::hash( &::core::mem::discriminant(self), state);
                ::core::hash::Hash::hash(&opcode, state);
                ::core::hash::Hash::hash(&args.len(pool), state); // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:355
                for &arg in args.as_slice(pool) {
                    ::core::hash::Hash::hash(&arg, state); // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:358
                }
            }
            Self::NullAry{opcode} =>  {
                ::core::hash::Hash::hash( &::core::mem::discriminant(self), state);
                ::core::hash::Hash::hash(&opcode, state);
                ::core::hash::Hash::hash(&0, state); // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:355
            }
            Self::Shuffle{opcode, ref args, imm} =>  {
                ::core::hash::Hash::hash( &::core::mem::discriminant(self), state);
                ::core::hash::Hash::hash(&opcode, state);
                ::core::hash::Hash::hash(&imm, state); // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:353
                ::core::hash::Hash::hash(&args.len(), state); // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:355
                for &arg in args {
                    ::core::hash::Hash::hash(&arg, state); // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:358
                }
            }
            Self::StackLoad{opcode, stack_slot, offset} =>  {
                ::core::hash::Hash::hash( &::core::mem::discriminant(self), state);
                ::core::hash::Hash::hash(&opcode, state);
                ::core::hash::Hash::hash(&stack_slot, state); // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:353
                ::core::hash::Hash::hash(&offset, state); // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:353
                ::core::hash::Hash::hash(&0, state); // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:355
            }
            Self::StackStore{opcode, ref arg, stack_slot, offset} =>  {
                ::core::hash::Hash::hash( &::core::mem::discriminant(self), state);
                ::core::hash::Hash::hash(&opcode, state);
                ::core::hash::Hash::hash(&stack_slot, state); // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:353
                ::core::hash::Hash::hash(&offset, state); // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:353
                ::core::hash::Hash::hash(&1, state); // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:355
                for &arg in core::slice::from_ref(arg) {
                    ::core::hash::Hash::hash(&arg, state); // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:358
                }
            }
            Self::Store{opcode, ref args, flags, offset} =>  {
                ::core::hash::Hash::hash( &::core::mem::discriminant(self), state);
                ::core::hash::Hash::hash(&opcode, state);
                ::core::hash::Hash::hash(&flags, state); // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:353
                ::core::hash::Hash::hash(&offset, state); // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:353
                ::core::hash::Hash::hash(&args.len(), state); // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:355
                for &arg in args {
                    ::core::hash::Hash::hash(&arg, state); // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:358
                }
            }
            Self::StoreNoOffset{opcode, ref args, flags} =>  {
                ::core::hash::Hash::hash( &::core::mem::discriminant(self), state);
                ::core::hash::Hash::hash(&opcode, state);
                ::core::hash::Hash::hash(&flags, state); // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:353
                ::core::hash::Hash::hash(&args.len(), state); // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:355
                for &arg in args {
                    ::core::hash::Hash::hash(&arg, state); // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:358
                }
            }
            Self::Ternary{opcode, ref args} =>  {
                ::core::hash::Hash::hash( &::core::mem::discriminant(self), state);
                ::core::hash::Hash::hash(&opcode, state);
                ::core::hash::Hash::hash(&args.len(), state); // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:355
                for &arg in args {
                    ::core::hash::Hash::hash(&arg, state); // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:358
                }
            }
            Self::TernaryImm8{opcode, ref args, imm} =>  {
                ::core::hash::Hash::hash( &::core::mem::discriminant(self), state);
                ::core::hash::Hash::hash(&opcode, state);
                ::core::hash::Hash::hash(&imm, state); // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:353
                ::core::hash::Hash::hash(&args.len(), state); // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:355
                for &arg in args {
                    ::core::hash::Hash::hash(&arg, state); // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:358
                }
            }
            Self::Trap{opcode, code} =>  {
                ::core::hash::Hash::hash( &::core::mem::discriminant(self), state);
                ::core::hash::Hash::hash(&opcode, state);
                ::core::hash::Hash::hash(&code, state); // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:353
                ::core::hash::Hash::hash(&0, state); // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:355
            }
            Self::TryCall{opcode, ref args, func_ref, exception} =>  {
                ::core::hash::Hash::hash( &::core::mem::discriminant(self), state);
                ::core::hash::Hash::hash(&opcode, state);
                ::core::hash::Hash::hash(&func_ref, state); // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:353
                ::core::hash::Hash::hash(&exception, state); // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:353
                ::core::hash::Hash::hash(&args.len(pool), state); // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:355
                for &arg in args.as_slice(pool) {
                    ::core::hash::Hash::hash(&arg, state); // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:358
                }
            }
            Self::TryCallIndirect{opcode, ref args, exception} =>  {
                ::core::hash::Hash::hash( &::core::mem::discriminant(self), state);
                ::core::hash::Hash::hash(&opcode, state);
                ::core::hash::Hash::hash(&exception, state); // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:353
                ::core::hash::Hash::hash(&args.len(pool), state); // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:355
                for &arg in args.as_slice(pool) {
                    ::core::hash::Hash::hash(&arg, state); // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:358
                }
            }
            Self::Unary{opcode, ref arg} =>  {
                ::core::hash::Hash::hash( &::core::mem::discriminant(self), state);
                ::core::hash::Hash::hash(&opcode, state);
                ::core::hash::Hash::hash(&1, state); // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:355
                for &arg in core::slice::from_ref(arg) {
                    ::core::hash::Hash::hash(&arg, state); // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:358
                }
            }
            Self::UnaryConst{opcode, constant_handle} =>  {
                ::core::hash::Hash::hash( &::core::mem::discriminant(self), state);
                ::core::hash::Hash::hash(&opcode, state);
                ::core::hash::Hash::hash(&constant_handle, state); // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:353
                ::core::hash::Hash::hash(&0, state); // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:355
            }
            Self::UnaryGlobalValue{opcode, global_value} =>  {
                ::core::hash::Hash::hash( &::core::mem::discriminant(self), state);
                ::core::hash::Hash::hash(&opcode, state);
                ::core::hash::Hash::hash(&global_value, state); // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:353
                ::core::hash::Hash::hash(&0, state); // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:355
            }
            Self::UnaryIeee16{opcode, imm} =>  {
                ::core::hash::Hash::hash( &::core::mem::discriminant(self), state);
                ::core::hash::Hash::hash(&opcode, state);
                ::core::hash::Hash::hash(&imm, state); // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:353
                ::core::hash::Hash::hash(&0, state); // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:355
            }
            Self::UnaryIeee32{opcode, imm} =>  {
                ::core::hash::Hash::hash( &::core::mem::discriminant(self), state);
                ::core::hash::Hash::hash(&opcode, state);
                ::core::hash::Hash::hash(&imm, state); // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:353
                ::core::hash::Hash::hash(&0, state); // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:355
            }
            Self::UnaryIeee64{opcode, imm} =>  {
                ::core::hash::Hash::hash( &::core::mem::discriminant(self), state);
                ::core::hash::Hash::hash(&opcode, state);
                ::core::hash::Hash::hash(&imm, state); // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:353
                ::core::hash::Hash::hash(&0, state); // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:355
            }
            Self::UnaryImm{opcode, imm} =>  {
                ::core::hash::Hash::hash( &::core::mem::discriminant(self), state);
                ::core::hash::Hash::hash(&opcode, state);
                ::core::hash::Hash::hash(&imm, state); // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:353
                ::core::hash::Hash::hash(&0, state); // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:355
            }
        }
    }

    /// Deep-clone an `InstructionData`, including any referenced lists.
    ///
    /// This operation requires a reference to a `ValueListPool` to
    /// clone the `ValueLists`.
    pub fn deep_clone(&self, pool: &mut ir::ValueListPool) -> Self {
        match *self {
            Self::AtomicCas{opcode, args, flags} =>  {
                Self::AtomicCas {
                    opcode, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:427
                    args, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:434
                    flags, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:457
                }
            }
            Self::AtomicRmw{opcode, args, flags, op} =>  {
                Self::AtomicRmw {
                    opcode, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:427
                    args, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:434
                    flags, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:457
                    op, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:457
                }
            }
            Self::Binary{opcode, args} =>  {
                Self::Binary {
                    opcode, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:427
                    args, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:434
                }
            }
            Self::BinaryImm64{opcode, arg, imm} =>  {
                Self::BinaryImm64 {
                    opcode, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:427
                    arg, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:432
                    imm, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:457
                }
            }
            Self::BinaryImm8{opcode, arg, imm} =>  {
                Self::BinaryImm8 {
                    opcode, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:427
                    arg, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:432
                    imm, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:457
                }
            }
            Self::BranchTable{opcode, arg, table} =>  {
                Self::BranchTable {
                    opcode, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:427
                    arg, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:432
                    table, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:457
                }
            }
            Self::Brif{opcode, arg, blocks} =>  {
                Self::Brif {
                    opcode, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:427
                    arg, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:432
                    blocks: [blocks[0].deep_clone(pool), blocks[1].deep_clone(pool)], // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:443
                }
            }
            Self::Call{opcode, ref args, func_ref} =>  {
                Self::Call {
                    opcode, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:427
                    args: args.deep_clone(pool), // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:430
                    func_ref, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:457
                }
            }
            Self::CallIndirect{opcode, ref args, sig_ref} =>  {
                Self::CallIndirect {
                    opcode, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:427
                    args: args.deep_clone(pool), // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:430
                    sig_ref, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:457
                }
            }
            Self::CondTrap{opcode, arg, code} =>  {
                Self::CondTrap {
                    opcode, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:427
                    arg, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:432
                    code, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:457
                }
            }
            Self::DynamicStackLoad{opcode, dynamic_stack_slot} =>  {
                Self::DynamicStackLoad {
                    opcode, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:427
                    dynamic_stack_slot, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:457
                }
            }
            Self::DynamicStackStore{opcode, arg, dynamic_stack_slot} =>  {
                Self::DynamicStackStore {
                    opcode, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:427
                    arg, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:432
                    dynamic_stack_slot, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:457
                }
            }
            Self::ExceptionHandlerAddress{opcode, block, imm} =>  {
                Self::ExceptionHandlerAddress {
                    opcode, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:427
                    block, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:451
                    imm, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:457
                }
            }
            Self::FloatCompare{opcode, args, cond} =>  {
                Self::FloatCompare {
                    opcode, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:427
                    args, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:434
                    cond, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:457
                }
            }
            Self::FuncAddr{opcode, func_ref} =>  {
                Self::FuncAddr {
                    opcode, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:427
                    func_ref, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:457
                }
            }
            Self::IntAddTrap{opcode, args, code} =>  {
                Self::IntAddTrap {
                    opcode, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:427
                    args, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:434
                    code, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:457
                }
            }
            Self::IntCompare{opcode, args, cond} =>  {
                Self::IntCompare {
                    opcode, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:427
                    args, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:434
                    cond, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:457
                }
            }
            Self::IntCompareImm{opcode, arg, cond, imm} =>  {
                Self::IntCompareImm {
                    opcode, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:427
                    arg, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:432
                    cond, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:457
                    imm, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:457
                }
            }
            Self::Jump{opcode, destination} =>  {
                Self::Jump {
                    opcode, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:427
                    destination: destination.deep_clone(pool), // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:440
                }
            }
            Self::Load{opcode, arg, flags, offset} =>  {
                Self::Load {
                    opcode, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:427
                    arg, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:432
                    flags, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:457
                    offset, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:457
                }
            }
            Self::LoadNoOffset{opcode, arg, flags} =>  {
                Self::LoadNoOffset {
                    opcode, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:427
                    arg, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:432
                    flags, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:457
                }
            }
            Self::MultiAry{opcode, ref args} =>  {
                Self::MultiAry {
                    opcode, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:427
                    args: args.deep_clone(pool), // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:430
                }
            }
            Self::NullAry{opcode} =>  {
                Self::NullAry {
                    opcode, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:427
                }
            }
            Self::Shuffle{opcode, args, imm} =>  {
                Self::Shuffle {
                    opcode, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:427
                    args, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:434
                    imm, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:457
                }
            }
            Self::StackLoad{opcode, stack_slot, offset} =>  {
                Self::StackLoad {
                    opcode, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:427
                    stack_slot, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:457
                    offset, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:457
                }
            }
            Self::StackStore{opcode, arg, stack_slot, offset} =>  {
                Self::StackStore {
                    opcode, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:427
                    arg, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:432
                    stack_slot, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:457
                    offset, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:457
                }
            }
            Self::Store{opcode, args, flags, offset} =>  {
                Self::Store {
                    opcode, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:427
                    args, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:434
                    flags, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:457
                    offset, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:457
                }
            }
            Self::StoreNoOffset{opcode, args, flags} =>  {
                Self::StoreNoOffset {
                    opcode, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:427
                    args, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:434
                    flags, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:457
                }
            }
            Self::Ternary{opcode, args} =>  {
                Self::Ternary {
                    opcode, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:427
                    args, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:434
                }
            }
            Self::TernaryImm8{opcode, args, imm} =>  {
                Self::TernaryImm8 {
                    opcode, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:427
                    args, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:434
                    imm, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:457
                }
            }
            Self::Trap{opcode, code} =>  {
                Self::Trap {
                    opcode, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:427
                    code, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:457
                }
            }
            Self::TryCall{opcode, ref args, func_ref, exception} =>  {
                Self::TryCall {
                    opcode, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:427
                    args: args.deep_clone(pool), // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:430
                    func_ref, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:457
                    exception, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:457
                }
            }
            Self::TryCallIndirect{opcode, ref args, exception} =>  {
                Self::TryCallIndirect {
                    opcode, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:427
                    args: args.deep_clone(pool), // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:430
                    exception, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:457
                }
            }
            Self::Unary{opcode, arg} =>  {
                Self::Unary {
                    opcode, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:427
                    arg, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:432
                }
            }
            Self::UnaryConst{opcode, constant_handle} =>  {
                Self::UnaryConst {
                    opcode, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:427
                    constant_handle, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:457
                }
            }
            Self::UnaryGlobalValue{opcode, global_value} =>  {
                Self::UnaryGlobalValue {
                    opcode, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:427
                    global_value, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:457
                }
            }
            Self::UnaryIeee16{opcode, imm} =>  {
                Self::UnaryIeee16 {
                    opcode, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:427
                    imm, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:457
                }
            }
            Self::UnaryIeee32{opcode, imm} =>  {
                Self::UnaryIeee32 {
                    opcode, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:427
                    imm, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:457
                }
            }
            Self::UnaryIeee64{opcode, imm} =>  {
                Self::UnaryIeee64 {
                    opcode, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:427
                    imm, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:457
                }
            }
            Self::UnaryImm{opcode, imm} =>  {
                Self::UnaryImm {
                    opcode, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:427
                    imm, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:457
                }
            }
        }
    }
    /// Map some functions, described by the given `InstructionMapper`, over each of the
    /// entities within this instruction, producing a new `InstructionData`.
    pub fn map(&self, mut mapper: impl crate::ir::instructions::InstructionMapper) -> Self {
        match *self {
            Self::AtomicCas{opcode, args, flags} =>  {
                Self::AtomicCas {
                    opcode, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:507
                    args: [mapper.map_value(args[0]), mapper.map_value(args[1]), mapper.map_value(args[2])], // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:518
                    flags, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:562
                }
            }
            Self::AtomicRmw{opcode, args, flags, op} =>  {
                Self::AtomicRmw {
                    opcode, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:507
                    args: [mapper.map_value(args[0]), mapper.map_value(args[1])], // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:518
                    flags, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:562
                    op, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:562
                }
            }
            Self::Binary{opcode, args} =>  {
                Self::Binary {
                    opcode, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:507
                    args: [mapper.map_value(args[0]), mapper.map_value(args[1])], // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:518
                }
            }
            Self::BinaryImm64{opcode, arg, imm} =>  {
                Self::BinaryImm64 {
                    opcode, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:507
                    arg: mapper.map_value(arg), // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:512
                    imm, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:562
                }
            }
            Self::BinaryImm8{opcode, arg, imm} =>  {
                Self::BinaryImm8 {
                    opcode, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:507
                    arg: mapper.map_value(arg), // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:512
                    imm, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:562
                }
            }
            Self::BranchTable{opcode, arg, table} =>  {
                Self::BranchTable {
                    opcode, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:507
                    arg: mapper.map_value(arg), // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:512
                    table: mapper.map_jump_table(table), // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:555
                }
            }
            Self::Brif{opcode, arg, blocks} =>  {
                Self::Brif {
                    opcode, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:507
                    arg: mapper.map_value(arg), // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:512
                    blocks: [mapper.map_block_call(blocks[0]), mapper.map_block_call(blocks[1])], // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:527
                }
            }
            Self::Call{opcode, args, func_ref} =>  {
                Self::Call {
                    opcode, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:507
                    args: mapper.map_value_list(args), // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:510
                    func_ref: mapper.map_func_ref(func_ref), // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:555
                }
            }
            Self::CallIndirect{opcode, args, sig_ref} =>  {
                Self::CallIndirect {
                    opcode, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:507
                    args: mapper.map_value_list(args), // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:510
                    sig_ref: mapper.map_sig_ref(sig_ref), // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:555
                }
            }
            Self::CondTrap{opcode, arg, code} =>  {
                Self::CondTrap {
                    opcode, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:507
                    arg: mapper.map_value(arg), // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:512
                    code, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:562
                }
            }
            Self::DynamicStackLoad{opcode, dynamic_stack_slot} =>  {
                Self::DynamicStackLoad {
                    opcode, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:507
                    dynamic_stack_slot: mapper.map_dynamic_stack_slot(dynamic_stack_slot), // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:555
                }
            }
            Self::DynamicStackStore{opcode, arg, dynamic_stack_slot} =>  {
                Self::DynamicStackStore {
                    opcode, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:507
                    arg: mapper.map_value(arg), // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:512
                    dynamic_stack_slot: mapper.map_dynamic_stack_slot(dynamic_stack_slot), // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:555
                }
            }
            Self::ExceptionHandlerAddress{opcode, block, imm} =>  {
                Self::ExceptionHandlerAddress {
                    opcode, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:507
                    block: mapper.map_block(block), // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:535
                    imm, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:562
                }
            }
            Self::FloatCompare{opcode, args, cond} =>  {
                Self::FloatCompare {
                    opcode, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:507
                    args: [mapper.map_value(args[0]), mapper.map_value(args[1])], // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:518
                    cond, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:562
                }
            }
            Self::FuncAddr{opcode, func_ref} =>  {
                Self::FuncAddr {
                    opcode, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:507
                    func_ref: mapper.map_func_ref(func_ref), // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:555
                }
            }
            Self::IntAddTrap{opcode, args, code} =>  {
                Self::IntAddTrap {
                    opcode, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:507
                    args: [mapper.map_value(args[0]), mapper.map_value(args[1])], // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:518
                    code, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:562
                }
            }
            Self::IntCompare{opcode, args, cond} =>  {
                Self::IntCompare {
                    opcode, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:507
                    args: [mapper.map_value(args[0]), mapper.map_value(args[1])], // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:518
                    cond, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:562
                }
            }
            Self::IntCompareImm{opcode, arg, cond, imm} =>  {
                Self::IntCompareImm {
                    opcode, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:507
                    arg: mapper.map_value(arg), // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:512
                    cond, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:562
                    imm, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:562
                }
            }
            Self::Jump{opcode, destination} =>  {
                Self::Jump {
                    opcode, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:507
                    destination: mapper.map_block_call(destination), // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:524
                }
            }
            Self::Load{opcode, arg, flags, offset} =>  {
                Self::Load {
                    opcode, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:507
                    arg: mapper.map_value(arg), // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:512
                    flags, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:562
                    offset, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:562
                }
            }
            Self::LoadNoOffset{opcode, arg, flags} =>  {
                Self::LoadNoOffset {
                    opcode, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:507
                    arg: mapper.map_value(arg), // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:512
                    flags, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:562
                }
            }
            Self::MultiAry{opcode, args} =>  {
                Self::MultiAry {
                    opcode, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:507
                    args: mapper.map_value_list(args), // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:510
                }
            }
            Self::NullAry{opcode} =>  {
                Self::NullAry {
                    opcode, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:507
                }
            }
            Self::Shuffle{opcode, args, imm} =>  {
                Self::Shuffle {
                    opcode, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:507
                    args: [mapper.map_value(args[0]), mapper.map_value(args[1])], // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:518
                    imm: mapper.map_immediate(imm), // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:555
                }
            }
            Self::StackLoad{opcode, stack_slot, offset} =>  {
                Self::StackLoad {
                    opcode, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:507
                    stack_slot: mapper.map_stack_slot(stack_slot), // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:555
                    offset, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:562
                }
            }
            Self::StackStore{opcode, arg, stack_slot, offset} =>  {
                Self::StackStore {
                    opcode, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:507
                    arg: mapper.map_value(arg), // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:512
                    stack_slot: mapper.map_stack_slot(stack_slot), // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:555
                    offset, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:562
                }
            }
            Self::Store{opcode, args, flags, offset} =>  {
                Self::Store {
                    opcode, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:507
                    args: [mapper.map_value(args[0]), mapper.map_value(args[1])], // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:518
                    flags, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:562
                    offset, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:562
                }
            }
            Self::StoreNoOffset{opcode, args, flags} =>  {
                Self::StoreNoOffset {
                    opcode, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:507
                    args: [mapper.map_value(args[0]), mapper.map_value(args[1])], // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:518
                    flags, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:562
                }
            }
            Self::Ternary{opcode, args} =>  {
                Self::Ternary {
                    opcode, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:507
                    args: [mapper.map_value(args[0]), mapper.map_value(args[1]), mapper.map_value(args[2])], // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:518
                }
            }
            Self::TernaryImm8{opcode, args, imm} =>  {
                Self::TernaryImm8 {
                    opcode, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:507
                    args: [mapper.map_value(args[0]), mapper.map_value(args[1])], // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:518
                    imm, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:562
                }
            }
            Self::Trap{opcode, code} =>  {
                Self::Trap {
                    opcode, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:507
                    code, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:562
                }
            }
            Self::TryCall{opcode, args, func_ref, exception} =>  {
                Self::TryCall {
                    opcode, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:507
                    args: mapper.map_value_list(args), // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:510
                    func_ref: mapper.map_func_ref(func_ref), // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:555
                    exception: mapper.map_exception_table(exception), // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:555
                }
            }
            Self::TryCallIndirect{opcode, args, exception} =>  {
                Self::TryCallIndirect {
                    opcode, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:507
                    args: mapper.map_value_list(args), // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:510
                    exception: mapper.map_exception_table(exception), // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:555
                }
            }
            Self::Unary{opcode, arg} =>  {
                Self::Unary {
                    opcode, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:507
                    arg: mapper.map_value(arg), // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:512
                }
            }
            Self::UnaryConst{opcode, constant_handle} =>  {
                Self::UnaryConst {
                    opcode, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:507
                    constant_handle: mapper.map_constant(constant_handle), // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:555
                }
            }
            Self::UnaryGlobalValue{opcode, global_value} =>  {
                Self::UnaryGlobalValue {
                    opcode, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:507
                    global_value: mapper.map_global_value(global_value), // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:555
                }
            }
            Self::UnaryIeee16{opcode, imm} =>  {
                Self::UnaryIeee16 {
                    opcode, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:507
                    imm, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:562
                }
            }
            Self::UnaryIeee32{opcode, imm} =>  {
                Self::UnaryIeee32 {
                    opcode, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:507
                    imm, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:562
                }
            }
            Self::UnaryIeee64{opcode, imm} =>  {
                Self::UnaryIeee64 {
                    opcode, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:507
                    imm, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:562
                }
            }
            Self::UnaryImm{opcode, imm} =>  {
                Self::UnaryImm {
                    opcode, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:507
                    imm, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:562
                }
            }
        }
    }
}

/// An instruction opcode.
///
/// All instructions from all supported ISAs are present.
#[repr(u8)]
#[derive(Copy, Clone, PartialEq, Eq, Debug, Hash)]
#[cfg_attr(
            feature = "enable-serde",
            derive(serde_derive::Serialize, serde_derive::Deserialize)
        )]
pub enum Opcode {
    /// `jump block_call`. (Jump)
    Jump = 1, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:633
    /// `brif c, block_then, block_else`. (Brif)
    /// Type inferred from `c`.
    Brif, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:636
    /// `br_table x, JT`. (BranchTable)
    BrTable, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:636
    /// `debugtrap`. (NullAry)
    Debugtrap, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:636
    /// `trap code`. (Trap)
    Trap, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:636
    /// `trapz c, code`. (CondTrap)
    /// Type inferred from `c`.
    Trapz, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:636
    /// `trapnz c, code`. (CondTrap)
    /// Type inferred from `c`.
    Trapnz, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:636
    /// `return rvals`. (MultiAry)
    Return, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:636
    /// `rvals = call FN, args`. (Call)
    Call, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:636
    /// `rvals = call_indirect SIG, callee, args`. (CallIndirect)
    /// Type inferred from `callee`.
    CallIndirect, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:636
    /// `return_call FN, args`. (Call)
    ReturnCall, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:636
    /// `return_call_indirect SIG, callee, args`. (CallIndirect)
    /// Type inferred from `callee`.
    ReturnCallIndirect, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:636
    /// `addr = func_addr FN`. (FuncAddr)
    FuncAddr, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:636
    /// `try_call callee, args, ET`. (TryCall)
    TryCall, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:636
    /// `try_call_indirect callee, args, ET`. (TryCallIndirect)
    /// Type inferred from `callee`.
    TryCallIndirect, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:636
    /// `a = splat x`. (Unary)
    Splat, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:636
    /// `a = swizzle x, y`. (Binary)
    Swizzle, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:636
    /// `a = x86_pshufb x, y`. (Binary)
    X86Pshufb, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:636
    /// `a = insertlane x, y, Idx`. (TernaryImm8)
    /// Type inferred from `x`.
    Insertlane, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:636
    /// `a = extractlane x, Idx`. (BinaryImm8)
    /// Type inferred from `x`.
    Extractlane, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:636
    /// `a = smin x, y`. (Binary)
    /// Type inferred from `x`.
    Smin, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:636
    /// `a = umin x, y`. (Binary)
    /// Type inferred from `x`.
    Umin, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:636
    /// `a = smax x, y`. (Binary)
    /// Type inferred from `x`.
    Smax, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:636
    /// `a = umax x, y`. (Binary)
    /// Type inferred from `x`.
    Umax, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:636
    /// `a = avg_round x, y`. (Binary)
    /// Type inferred from `x`.
    AvgRound, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:636
    /// `a = uadd_sat x, y`. (Binary)
    /// Type inferred from `x`.
    UaddSat, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:636
    /// `a = sadd_sat x, y`. (Binary)
    /// Type inferred from `x`.
    SaddSat, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:636
    /// `a = usub_sat x, y`. (Binary)
    /// Type inferred from `x`.
    UsubSat, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:636
    /// `a = ssub_sat x, y`. (Binary)
    /// Type inferred from `x`.
    SsubSat, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:636
    /// `a = load MemFlags, p, Offset`. (Load)
    Load, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:636
    /// `store MemFlags, x, p, Offset`. (Store)
    /// Type inferred from `x`.
    Store, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:636
    /// `a = uload8 MemFlags, p, Offset`. (Load)
    Uload8, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:636
    /// `a = sload8 MemFlags, p, Offset`. (Load)
    Sload8, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:636
    /// `istore8 MemFlags, x, p, Offset`. (Store)
    /// Type inferred from `x`.
    Istore8, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:636
    /// `a = uload16 MemFlags, p, Offset`. (Load)
    Uload16, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:636
    /// `a = sload16 MemFlags, p, Offset`. (Load)
    Sload16, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:636
    /// `istore16 MemFlags, x, p, Offset`. (Store)
    /// Type inferred from `x`.
    Istore16, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:636
    /// `a = uload32 MemFlags, p, Offset`. (Load)
    /// Type inferred from `p`.
    Uload32, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:636
    /// `a = sload32 MemFlags, p, Offset`. (Load)
    /// Type inferred from `p`.
    Sload32, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:636
    /// `istore32 MemFlags, x, p, Offset`. (Store)
    /// Type inferred from `x`.
    Istore32, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:636
    /// `out_payload0 = stack_switch store_context_ptr, load_context_ptr, in_payload0`. (Ternary)
    /// Type inferred from `load_context_ptr`.
    StackSwitch, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:636
    /// `a = uload8x8 MemFlags, p, Offset`. (Load)
    /// Type inferred from `p`.
    Uload8x8, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:636
    /// `a = sload8x8 MemFlags, p, Offset`. (Load)
    /// Type inferred from `p`.
    Sload8x8, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:636
    /// `a = uload16x4 MemFlags, p, Offset`. (Load)
    /// Type inferred from `p`.
    Uload16x4, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:636
    /// `a = sload16x4 MemFlags, p, Offset`. (Load)
    /// Type inferred from `p`.
    Sload16x4, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:636
    /// `a = uload32x2 MemFlags, p, Offset`. (Load)
    /// Type inferred from `p`.
    Uload32x2, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:636
    /// `a = sload32x2 MemFlags, p, Offset`. (Load)
    /// Type inferred from `p`.
    Sload32x2, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:636
    /// `a = stack_load SS, Offset`. (StackLoad)
    StackLoad, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:636
    /// `stack_store x, SS, Offset`. (StackStore)
    /// Type inferred from `x`.
    StackStore, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:636
    /// `addr = stack_addr SS, Offset`. (StackLoad)
    StackAddr, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:636
    /// `a = dynamic_stack_load DSS`. (DynamicStackLoad)
    DynamicStackLoad, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:636
    /// `dynamic_stack_store x, DSS`. (DynamicStackStore)
    /// Type inferred from `x`.
    DynamicStackStore, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:636
    /// `addr = dynamic_stack_addr DSS`. (DynamicStackLoad)
    DynamicStackAddr, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:636
    /// `a = global_value GV`. (UnaryGlobalValue)
    GlobalValue, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:636
    /// `a = symbol_value GV`. (UnaryGlobalValue)
    SymbolValue, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:636
    /// `a = tls_value GV`. (UnaryGlobalValue)
    TlsValue, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:636
    /// `addr = get_pinned_reg`. (NullAry)
    GetPinnedReg, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:636
    /// `set_pinned_reg addr`. (Unary)
    /// Type inferred from `addr`.
    SetPinnedReg, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:636
    /// `addr = get_frame_pointer`. (NullAry)
    GetFramePointer, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:636
    /// `addr = get_stack_pointer`. (NullAry)
    GetStackPointer, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:636
    /// `addr = get_return_address`. (NullAry)
    GetReturnAddress, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:636
    /// `addr = get_exception_handler_address block, index`. (ExceptionHandlerAddress)
    GetExceptionHandlerAddress, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:636
    /// `a = iconst N`. (UnaryImm)
    Iconst, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:636
    /// `a = f16const N`. (UnaryIeee16)
    F16const, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:636
    /// `a = f32const N`. (UnaryIeee32)
    F32const, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:636
    /// `a = f64const N`. (UnaryIeee64)
    F64const, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:636
    /// `a = f128const N`. (UnaryConst)
    F128const, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:636
    /// `a = vconst N`. (UnaryConst)
    Vconst, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:636
    /// `a = shuffle a, b, mask`. (Shuffle)
    Shuffle, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:636
    /// `nop`. (NullAry)
    Nop, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:636
    /// `a = select c, x, y`. (Ternary)
    /// Type inferred from `x`.
    Select, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:636
    /// `a = select_spectre_guard c, x, y`. (Ternary)
    /// Type inferred from `x`.
    SelectSpectreGuard, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:636
    /// `a = bitselect c, x, y`. (Ternary)
    /// Type inferred from `x`.
    Bitselect, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:636
    /// `a = blendv c, x, y`. (Ternary)
    /// Type inferred from `x`.
    Blendv, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:636
    /// `s = vany_true a`. (Unary)
    /// Type inferred from `a`.
    VanyTrue, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:636
    /// `s = vall_true a`. (Unary)
    /// Type inferred from `a`.
    VallTrue, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:636
    /// `x = vhigh_bits a`. (Unary)
    VhighBits, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:636
    /// `a = icmp Cond, x, y`. (IntCompare)
    /// Type inferred from `x`.
    Icmp, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:636
    /// `a = icmp_imm Cond, x, Y`. (IntCompareImm)
    /// Type inferred from `x`.
    IcmpImm, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:636
    /// `a = iadd x, y`. (Binary)
    /// Type inferred from `x`.
    Iadd, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:636
    /// `a = isub x, y`. (Binary)
    /// Type inferred from `x`.
    Isub, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:636
    /// `a = ineg x`. (Unary)
    /// Type inferred from `x`.
    Ineg, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:636
    /// `a = iabs x`. (Unary)
    /// Type inferred from `x`.
    Iabs, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:636
    /// `a = imul x, y`. (Binary)
    /// Type inferred from `x`.
    Imul, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:636
    /// `a = umulhi x, y`. (Binary)
    /// Type inferred from `x`.
    Umulhi, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:636
    /// `a = smulhi x, y`. (Binary)
    /// Type inferred from `x`.
    Smulhi, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:636
    /// `a = sqmul_round_sat x, y`. (Binary)
    /// Type inferred from `x`.
    SqmulRoundSat, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:636
    /// `a = x86_pmulhrsw x, y`. (Binary)
    /// Type inferred from `x`.
    X86Pmulhrsw, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:636
    /// `a = udiv x, y`. (Binary)
    /// Type inferred from `x`.
    Udiv, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:636
    /// `a = sdiv x, y`. (Binary)
    /// Type inferred from `x`.
    Sdiv, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:636
    /// `a = urem x, y`. (Binary)
    /// Type inferred from `x`.
    Urem, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:636
    /// `a = srem x, y`. (Binary)
    /// Type inferred from `x`.
    Srem, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:636
    /// `a = iadd_imm x, Y`. (BinaryImm64)
    /// Type inferred from `x`.
    IaddImm, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:636
    /// `a = imul_imm x, Y`. (BinaryImm64)
    /// Type inferred from `x`.
    ImulImm, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:636
    /// `a = udiv_imm x, Y`. (BinaryImm64)
    /// Type inferred from `x`.
    UdivImm, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:636
    /// `a = sdiv_imm x, Y`. (BinaryImm64)
    /// Type inferred from `x`.
    SdivImm, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:636
    /// `a = urem_imm x, Y`. (BinaryImm64)
    /// Type inferred from `x`.
    UremImm, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:636
    /// `a = srem_imm x, Y`. (BinaryImm64)
    /// Type inferred from `x`.
    SremImm, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:636
    /// `a = irsub_imm x, Y`. (BinaryImm64)
    /// Type inferred from `x`.
    IrsubImm, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:636
    /// `a, c_out = sadd_overflow_cin x, y, c_in`. (Ternary)
    /// Type inferred from `y`.
    SaddOverflowCin, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:636
    /// `a, c_out = uadd_overflow_cin x, y, c_in`. (Ternary)
    /// Type inferred from `y`.
    UaddOverflowCin, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:636
    /// `a, of = uadd_overflow x, y`. (Binary)
    /// Type inferred from `x`.
    UaddOverflow, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:636
    /// `a, of = sadd_overflow x, y`. (Binary)
    /// Type inferred from `x`.
    SaddOverflow, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:636
    /// `a, of = usub_overflow x, y`. (Binary)
    /// Type inferred from `x`.
    UsubOverflow, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:636
    /// `a, of = ssub_overflow x, y`. (Binary)
    /// Type inferred from `x`.
    SsubOverflow, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:636
    /// `a, of = umul_overflow x, y`. (Binary)
    /// Type inferred from `x`.
    UmulOverflow, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:636
    /// `a, of = smul_overflow x, y`. (Binary)
    /// Type inferred from `x`.
    SmulOverflow, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:636
    /// `a = uadd_overflow_trap x, y, code`. (IntAddTrap)
    /// Type inferred from `x`.
    UaddOverflowTrap, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:636
    /// `a, b_out = ssub_overflow_bin x, y, b_in`. (Ternary)
    /// Type inferred from `y`.
    SsubOverflowBin, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:636
    /// `a, b_out = usub_overflow_bin x, y, b_in`. (Ternary)
    /// Type inferred from `y`.
    UsubOverflowBin, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:636
    /// `a = band x, y`. (Binary)
    /// Type inferred from `x`.
    Band, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:636
    /// `a = bor x, y`. (Binary)
    /// Type inferred from `x`.
    Bor, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:636
    /// `a = bxor x, y`. (Binary)
    /// Type inferred from `x`.
    Bxor, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:636
    /// `a = bnot x`. (Unary)
    /// Type inferred from `x`.
    Bnot, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:636
    /// `a = band_not x, y`. (Binary)
    /// Type inferred from `x`.
    BandNot, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:636
    /// `a = bor_not x, y`. (Binary)
    /// Type inferred from `x`.
    BorNot, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:636
    /// `a = bxor_not x, y`. (Binary)
    /// Type inferred from `x`.
    BxorNot, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:636
    /// `a = band_imm x, Y`. (BinaryImm64)
    /// Type inferred from `x`.
    BandImm, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:636
    /// `a = bor_imm x, Y`. (BinaryImm64)
    /// Type inferred from `x`.
    BorImm, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:636
    /// `a = bxor_imm x, Y`. (BinaryImm64)
    /// Type inferred from `x`.
    BxorImm, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:636
    /// `a = rotl x, y`. (Binary)
    /// Type inferred from `x`.
    Rotl, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:636
    /// `a = rotr x, y`. (Binary)
    /// Type inferred from `x`.
    Rotr, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:636
    /// `a = rotl_imm x, Y`. (BinaryImm64)
    /// Type inferred from `x`.
    RotlImm, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:636
    /// `a = rotr_imm x, Y`. (BinaryImm64)
    /// Type inferred from `x`.
    RotrImm, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:636
    /// `a = ishl x, y`. (Binary)
    /// Type inferred from `x`.
    Ishl, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:636
    /// `a = ushr x, y`. (Binary)
    /// Type inferred from `x`.
    Ushr, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:636
    /// `a = sshr x, y`. (Binary)
    /// Type inferred from `x`.
    Sshr, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:636
    /// `a = ishl_imm x, Y`. (BinaryImm64)
    /// Type inferred from `x`.
    IshlImm, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:636
    /// `a = ushr_imm x, Y`. (BinaryImm64)
    /// Type inferred from `x`.
    UshrImm, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:636
    /// `a = sshr_imm x, Y`. (BinaryImm64)
    /// Type inferred from `x`.
    SshrImm, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:636
    /// `a = bitrev x`. (Unary)
    /// Type inferred from `x`.
    Bitrev, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:636
    /// `a = clz x`. (Unary)
    /// Type inferred from `x`.
    Clz, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:636
    /// `a = cls x`. (Unary)
    /// Type inferred from `x`.
    Cls, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:636
    /// `a = ctz x`. (Unary)
    /// Type inferred from `x`.
    Ctz, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:636
    /// `a = bswap x`. (Unary)
    /// Type inferred from `x`.
    Bswap, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:636
    /// `a = popcnt x`. (Unary)
    /// Type inferred from `x`.
    Popcnt, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:636
    /// `a = fcmp Cond, x, y`. (FloatCompare)
    /// Type inferred from `x`.
    Fcmp, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:636
    /// `a = fadd x, y`. (Binary)
    /// Type inferred from `x`.
    Fadd, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:636
    /// `a = fsub x, y`. (Binary)
    /// Type inferred from `x`.
    Fsub, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:636
    /// `a = fmul x, y`. (Binary)
    /// Type inferred from `x`.
    Fmul, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:636
    /// `a = fdiv x, y`. (Binary)
    /// Type inferred from `x`.
    Fdiv, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:636
    /// `a = sqrt x`. (Unary)
    /// Type inferred from `x`.
    Sqrt, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:636
    /// `a = fma x, y, z`. (Ternary)
    /// Type inferred from `y`.
    Fma, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:636
    /// `a = fneg x`. (Unary)
    /// Type inferred from `x`.
    Fneg, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:636
    /// `a = fabs x`. (Unary)
    /// Type inferred from `x`.
    Fabs, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:636
    /// `a = fcopysign x, y`. (Binary)
    /// Type inferred from `x`.
    Fcopysign, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:636
    /// `a = fmin x, y`. (Binary)
    /// Type inferred from `x`.
    Fmin, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:636
    /// `a = fmax x, y`. (Binary)
    /// Type inferred from `x`.
    Fmax, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:636
    /// `a = ceil x`. (Unary)
    /// Type inferred from `x`.
    Ceil, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:636
    /// `a = floor x`. (Unary)
    /// Type inferred from `x`.
    Floor, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:636
    /// `a = trunc x`. (Unary)
    /// Type inferred from `x`.
    Trunc, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:636
    /// `a = nearest x`. (Unary)
    /// Type inferred from `x`.
    Nearest, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:636
    /// `a = bitcast MemFlags, x`. (LoadNoOffset)
    Bitcast, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:636
    /// `a = scalar_to_vector s`. (Unary)
    ScalarToVector, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:636
    /// `a = bmask x`. (Unary)
    Bmask, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:636
    /// `a = ireduce x`. (Unary)
    Ireduce, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:636
    /// `a = snarrow x, y`. (Binary)
    /// Type inferred from `x`.
    Snarrow, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:636
    /// `a = unarrow x, y`. (Binary)
    /// Type inferred from `x`.
    Unarrow, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:636
    /// `a = uunarrow x, y`. (Binary)
    /// Type inferred from `x`.
    Uunarrow, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:636
    /// `a = swiden_low x`. (Unary)
    /// Type inferred from `x`.
    SwidenLow, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:636
    /// `a = swiden_high x`. (Unary)
    /// Type inferred from `x`.
    SwidenHigh, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:636
    /// `a = uwiden_low x`. (Unary)
    /// Type inferred from `x`.
    UwidenLow, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:636
    /// `a = uwiden_high x`. (Unary)
    /// Type inferred from `x`.
    UwidenHigh, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:636
    /// `a = iadd_pairwise x, y`. (Binary)
    /// Type inferred from `x`.
    IaddPairwise, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:636
    /// `a = x86_pmaddubsw x, y`. (Binary)
    X86Pmaddubsw, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:636
    /// `a = uextend x`. (Unary)
    Uextend, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:636
    /// `a = sextend x`. (Unary)
    Sextend, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:636
    /// `a = fpromote x`. (Unary)
    Fpromote, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:636
    /// `a = fdemote x`. (Unary)
    Fdemote, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:636
    /// `a = fvdemote x`. (Unary)
    Fvdemote, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:636
    /// `x = fvpromote_low a`. (Unary)
    FvpromoteLow, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:636
    /// `a = fcvt_to_uint x`. (Unary)
    FcvtToUint, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:636
    /// `a = fcvt_to_sint x`. (Unary)
    FcvtToSint, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:636
    /// `a = fcvt_to_uint_sat x`. (Unary)
    FcvtToUintSat, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:636
    /// `a = fcvt_to_sint_sat x`. (Unary)
    FcvtToSintSat, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:636
    /// `a = x86_cvtt2dq x`. (Unary)
    X86Cvtt2dq, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:636
    /// `a = fcvt_from_uint x`. (Unary)
    FcvtFromUint, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:636
    /// `a = fcvt_from_sint x`. (Unary)
    FcvtFromSint, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:636
    /// `lo, hi = isplit x`. (Unary)
    /// Type inferred from `x`.
    Isplit, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:636
    /// `a = iconcat lo, hi`. (Binary)
    /// Type inferred from `lo`.
    Iconcat, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:636
    /// `a = atomic_rmw MemFlags, AtomicRmwOp, p, x`. (AtomicRmw)
    AtomicRmw, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:636
    /// `a = atomic_cas MemFlags, p, e, x`. (AtomicCas)
    /// Type inferred from `x`.
    AtomicCas, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:636
    /// `a = atomic_load MemFlags, p`. (LoadNoOffset)
    AtomicLoad, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:636
    /// `atomic_store MemFlags, x, p`. (StoreNoOffset)
    /// Type inferred from `x`.
    AtomicStore, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:636
    /// `fence`. (NullAry)
    Fence, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:636
    /// `a = extract_vector x, y`. (BinaryImm8)
    /// Type inferred from `x`.
    ExtractVector, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:636
    /// `sequence_point`. (NullAry)
    SequencePoint, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:636
}

impl Opcode {
    /// True for instructions that terminate the block
    pub fn is_terminator(self) -> bool {
        match self {
            Self::BrTable |
            Self::Brif |
            Self::Jump |
            Self::Return |
            Self::ReturnCall |
            Self::ReturnCallIndirect |
            Self::Trap |
            Self::TryCall |
            Self::TryCallIndirect => {
                true
            }
            _ => {
                false
            }
        }
    }

    /// True for all branch or jump instructions.
    pub fn is_branch(self) -> bool {
        match self {
            Self::BrTable |
            Self::Brif |
            Self::Jump |
            Self::TryCall |
            Self::TryCallIndirect => {
                true
            }
            _ => {
                false
            }
        }
    }

    /// Is this a call instruction?
    pub fn is_call(self) -> bool {
        match self {
            Self::Call |
            Self::CallIndirect |
            Self::ReturnCall |
            Self::ReturnCallIndirect |
            Self::StackSwitch |
            Self::TryCall |
            Self::TryCallIndirect => {
                true
            }
            _ => {
                false
            }
        }
    }

    /// Is this a return instruction?
    pub fn is_return(self) -> bool {
        match self {
            Self::Return |
            Self::ReturnCall |
            Self::ReturnCallIndirect => {
                true
            }
            _ => {
                false
            }
        }
    }

    /// Can this instruction read from memory?
    pub fn can_load(self) -> bool {
        match self {
            Self::AtomicCas |
            Self::AtomicLoad |
            Self::AtomicRmw |
            Self::Debugtrap |
            Self::DynamicStackLoad |
            Self::Load |
            Self::Sload16 |
            Self::Sload16x4 |
            Self::Sload32 |
            Self::Sload32x2 |
            Self::Sload8 |
            Self::Sload8x8 |
            Self::StackLoad |
            Self::StackSwitch |
            Self::Uload16 |
            Self::Uload16x4 |
            Self::Uload32 |
            Self::Uload32x2 |
            Self::Uload8 |
            Self::Uload8x8 => {
                true
            }
            _ => {
                false
            }
        }
    }

    /// Can this instruction write to memory?
    pub fn can_store(self) -> bool {
        match self {
            Self::AtomicCas |
            Self::AtomicRmw |
            Self::AtomicStore |
            Self::Debugtrap |
            Self::DynamicStackStore |
            Self::Istore16 |
            Self::Istore32 |
            Self::Istore8 |
            Self::StackStore |
            Self::StackSwitch |
            Self::Store => {
                true
            }
            _ => {
                false
            }
        }
    }

    /// Can this instruction cause a trap?
    pub fn can_trap(self) -> bool {
        match self {
            Self::FcvtToSint |
            Self::FcvtToUint |
            Self::Sdiv |
            Self::Srem |
            Self::Trap |
            Self::Trapnz |
            Self::Trapz |
            Self::UaddOverflowTrap |
            Self::Udiv |
            Self::Urem => {
                true
            }
            _ => {
                false
            }
        }
    }

    /// Does this instruction have other side effects besides can_* flags?
    pub fn other_side_effects(self) -> bool {
        match self {
            Self::AtomicCas |
            Self::AtomicLoad |
            Self::AtomicRmw |
            Self::AtomicStore |
            Self::Debugtrap |
            Self::Fence |
            Self::GetPinnedReg |
            Self::SequencePoint |
            Self::SetPinnedReg |
            Self::StackSwitch => {
                true
            }
            _ => {
                false
            }
        }
    }

    /// Despite having side effects, is this instruction okay to GVN?
    pub fn side_effects_idempotent(self) -> bool {
        match self {
            Self::FcvtToSint |
            Self::FcvtToUint |
            Self::Sdiv |
            Self::Srem |
            Self::Trapnz |
            Self::Trapz |
            Self::UaddOverflowTrap |
            Self::Udiv |
            Self::Urem => {
                true
            }
            _ => {
                false
            }
        }
    }

    /// All cranelift opcodes.
    pub fn all() -> &'static [Opcode] {
        return &[
            Opcode::Jump, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:713
            Opcode::Brif, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:713
            Opcode::BrTable, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:713
            Opcode::Debugtrap, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:713
            Opcode::Trap, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:713
            Opcode::Trapz, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:713
            Opcode::Trapnz, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:713
            Opcode::Return, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:713
            Opcode::Call, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:713
            Opcode::CallIndirect, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:713
            Opcode::ReturnCall, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:713
            Opcode::ReturnCallIndirect, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:713
            Opcode::FuncAddr, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:713
            Opcode::TryCall, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:713
            Opcode::TryCallIndirect, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:713
            Opcode::Splat, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:713
            Opcode::Swizzle, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:713
            Opcode::X86Pshufb, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:713
            Opcode::Insertlane, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:713
            Opcode::Extractlane, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:713
            Opcode::Smin, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:713
            Opcode::Umin, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:713
            Opcode::Smax, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:713
            Opcode::Umax, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:713
            Opcode::AvgRound, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:713
            Opcode::UaddSat, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:713
            Opcode::SaddSat, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:713
            Opcode::UsubSat, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:713
            Opcode::SsubSat, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:713
            Opcode::Load, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:713
            Opcode::Store, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:713
            Opcode::Uload8, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:713
            Opcode::Sload8, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:713
            Opcode::Istore8, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:713
            Opcode::Uload16, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:713
            Opcode::Sload16, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:713
            Opcode::Istore16, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:713
            Opcode::Uload32, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:713
            Opcode::Sload32, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:713
            Opcode::Istore32, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:713
            Opcode::StackSwitch, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:713
            Opcode::Uload8x8, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:713
            Opcode::Sload8x8, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:713
            Opcode::Uload16x4, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:713
            Opcode::Sload16x4, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:713
            Opcode::Uload32x2, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:713
            Opcode::Sload32x2, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:713
            Opcode::StackLoad, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:713
            Opcode::StackStore, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:713
            Opcode::StackAddr, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:713
            Opcode::DynamicStackLoad, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:713
            Opcode::DynamicStackStore, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:713
            Opcode::DynamicStackAddr, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:713
            Opcode::GlobalValue, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:713
            Opcode::SymbolValue, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:713
            Opcode::TlsValue, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:713
            Opcode::GetPinnedReg, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:713
            Opcode::SetPinnedReg, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:713
            Opcode::GetFramePointer, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:713
            Opcode::GetStackPointer, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:713
            Opcode::GetReturnAddress, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:713
            Opcode::GetExceptionHandlerAddress, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:713
            Opcode::Iconst, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:713
            Opcode::F16const, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:713
            Opcode::F32const, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:713
            Opcode::F64const, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:713
            Opcode::F128const, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:713
            Opcode::Vconst, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:713
            Opcode::Shuffle, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:713
            Opcode::Nop, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:713
            Opcode::Select, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:713
            Opcode::SelectSpectreGuard, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:713
            Opcode::Bitselect, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:713
            Opcode::Blendv, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:713
            Opcode::VanyTrue, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:713
            Opcode::VallTrue, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:713
            Opcode::VhighBits, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:713
            Opcode::Icmp, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:713
            Opcode::IcmpImm, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:713
            Opcode::Iadd, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:713
            Opcode::Isub, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:713
            Opcode::Ineg, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:713
            Opcode::Iabs, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:713
            Opcode::Imul, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:713
            Opcode::Umulhi, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:713
            Opcode::Smulhi, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:713
            Opcode::SqmulRoundSat, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:713
            Opcode::X86Pmulhrsw, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:713
            Opcode::Udiv, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:713
            Opcode::Sdiv, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:713
            Opcode::Urem, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:713
            Opcode::Srem, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:713
            Opcode::IaddImm, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:713
            Opcode::ImulImm, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:713
            Opcode::UdivImm, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:713
            Opcode::SdivImm, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:713
            Opcode::UremImm, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:713
            Opcode::SremImm, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:713
            Opcode::IrsubImm, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:713
            Opcode::SaddOverflowCin, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:713
            Opcode::UaddOverflowCin, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:713
            Opcode::UaddOverflow, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:713
            Opcode::SaddOverflow, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:713
            Opcode::UsubOverflow, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:713
            Opcode::SsubOverflow, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:713
            Opcode::UmulOverflow, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:713
            Opcode::SmulOverflow, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:713
            Opcode::UaddOverflowTrap, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:713
            Opcode::SsubOverflowBin, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:713
            Opcode::UsubOverflowBin, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:713
            Opcode::Band, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:713
            Opcode::Bor, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:713
            Opcode::Bxor, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:713
            Opcode::Bnot, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:713
            Opcode::BandNot, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:713
            Opcode::BorNot, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:713
            Opcode::BxorNot, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:713
            Opcode::BandImm, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:713
            Opcode::BorImm, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:713
            Opcode::BxorImm, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:713
            Opcode::Rotl, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:713
            Opcode::Rotr, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:713
            Opcode::RotlImm, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:713
            Opcode::RotrImm, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:713
            Opcode::Ishl, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:713
            Opcode::Ushr, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:713
            Opcode::Sshr, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:713
            Opcode::IshlImm, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:713
            Opcode::UshrImm, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:713
            Opcode::SshrImm, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:713
            Opcode::Bitrev, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:713
            Opcode::Clz, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:713
            Opcode::Cls, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:713
            Opcode::Ctz, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:713
            Opcode::Bswap, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:713
            Opcode::Popcnt, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:713
            Opcode::Fcmp, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:713
            Opcode::Fadd, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:713
            Opcode::Fsub, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:713
            Opcode::Fmul, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:713
            Opcode::Fdiv, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:713
            Opcode::Sqrt, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:713
            Opcode::Fma, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:713
            Opcode::Fneg, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:713
            Opcode::Fabs, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:713
            Opcode::Fcopysign, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:713
            Opcode::Fmin, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:713
            Opcode::Fmax, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:713
            Opcode::Ceil, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:713
            Opcode::Floor, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:713
            Opcode::Trunc, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:713
            Opcode::Nearest, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:713
            Opcode::Bitcast, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:713
            Opcode::ScalarToVector, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:713
            Opcode::Bmask, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:713
            Opcode::Ireduce, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:713
            Opcode::Snarrow, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:713
            Opcode::Unarrow, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:713
            Opcode::Uunarrow, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:713
            Opcode::SwidenLow, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:713
            Opcode::SwidenHigh, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:713
            Opcode::UwidenLow, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:713
            Opcode::UwidenHigh, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:713
            Opcode::IaddPairwise, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:713
            Opcode::X86Pmaddubsw, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:713
            Opcode::Uextend, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:713
            Opcode::Sextend, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:713
            Opcode::Fpromote, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:713
            Opcode::Fdemote, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:713
            Opcode::Fvdemote, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:713
            Opcode::FvpromoteLow, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:713
            Opcode::FcvtToUint, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:713
            Opcode::FcvtToSint, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:713
            Opcode::FcvtToUintSat, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:713
            Opcode::FcvtToSintSat, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:713
            Opcode::X86Cvtt2dq, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:713
            Opcode::FcvtFromUint, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:713
            Opcode::FcvtFromSint, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:713
            Opcode::Isplit, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:713
            Opcode::Iconcat, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:713
            Opcode::AtomicRmw, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:713
            Opcode::AtomicCas, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:713
            Opcode::AtomicLoad, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:713
            Opcode::AtomicStore, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:713
            Opcode::Fence, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:713
            Opcode::ExtractVector, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:713
            Opcode::SequencePoint, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:713
        ];
    }

}

const OPCODE_FORMAT: [InstructionFormat; 187] = [ // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:723
    InstructionFormat::Jump, // jump // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:730
    InstructionFormat::Brif, // brif // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:730
    InstructionFormat::BranchTable, // br_table // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:730
    InstructionFormat::NullAry, // debugtrap // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:730
    InstructionFormat::Trap, // trap // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:730
    InstructionFormat::CondTrap, // trapz // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:730
    InstructionFormat::CondTrap, // trapnz // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:730
    InstructionFormat::MultiAry, // return // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:730
    InstructionFormat::Call, // call // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:730
    InstructionFormat::CallIndirect, // call_indirect // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:730
    InstructionFormat::Call, // return_call // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:730
    InstructionFormat::CallIndirect, // return_call_indirect // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:730
    InstructionFormat::FuncAddr, // func_addr // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:730
    InstructionFormat::TryCall, // try_call // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:730
    InstructionFormat::TryCallIndirect, // try_call_indirect // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:730
    InstructionFormat::Unary, // splat // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:730
    InstructionFormat::Binary, // swizzle // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:730
    InstructionFormat::Binary, // x86_pshufb // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:730
    InstructionFormat::TernaryImm8, // insertlane // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:730
    InstructionFormat::BinaryImm8, // extractlane // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:730
    InstructionFormat::Binary, // smin // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:730
    InstructionFormat::Binary, // umin // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:730
    InstructionFormat::Binary, // smax // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:730
    InstructionFormat::Binary, // umax // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:730
    InstructionFormat::Binary, // avg_round // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:730
    InstructionFormat::Binary, // uadd_sat // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:730
    InstructionFormat::Binary, // sadd_sat // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:730
    InstructionFormat::Binary, // usub_sat // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:730
    InstructionFormat::Binary, // ssub_sat // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:730
    InstructionFormat::Load, // load // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:730
    InstructionFormat::Store, // store // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:730
    InstructionFormat::Load, // uload8 // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:730
    InstructionFormat::Load, // sload8 // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:730
    InstructionFormat::Store, // istore8 // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:730
    InstructionFormat::Load, // uload16 // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:730
    InstructionFormat::Load, // sload16 // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:730
    InstructionFormat::Store, // istore16 // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:730
    InstructionFormat::Load, // uload32 // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:730
    InstructionFormat::Load, // sload32 // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:730
    InstructionFormat::Store, // istore32 // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:730
    InstructionFormat::Ternary, // stack_switch // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:730
    InstructionFormat::Load, // uload8x8 // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:730
    InstructionFormat::Load, // sload8x8 // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:730
    InstructionFormat::Load, // uload16x4 // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:730
    InstructionFormat::Load, // sload16x4 // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:730
    InstructionFormat::Load, // uload32x2 // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:730
    InstructionFormat::Load, // sload32x2 // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:730
    InstructionFormat::StackLoad, // stack_load // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:730
    InstructionFormat::StackStore, // stack_store // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:730
    InstructionFormat::StackLoad, // stack_addr // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:730
    InstructionFormat::DynamicStackLoad, // dynamic_stack_load // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:730
    InstructionFormat::DynamicStackStore, // dynamic_stack_store // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:730
    InstructionFormat::DynamicStackLoad, // dynamic_stack_addr // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:730
    InstructionFormat::UnaryGlobalValue, // global_value // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:730
    InstructionFormat::UnaryGlobalValue, // symbol_value // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:730
    InstructionFormat::UnaryGlobalValue, // tls_value // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:730
    InstructionFormat::NullAry, // get_pinned_reg // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:730
    InstructionFormat::Unary, // set_pinned_reg // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:730
    InstructionFormat::NullAry, // get_frame_pointer // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:730
    InstructionFormat::NullAry, // get_stack_pointer // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:730
    InstructionFormat::NullAry, // get_return_address // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:730
    InstructionFormat::ExceptionHandlerAddress, // get_exception_handler_address // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:730
    InstructionFormat::UnaryImm, // iconst // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:730
    InstructionFormat::UnaryIeee16, // f16const // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:730
    InstructionFormat::UnaryIeee32, // f32const // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:730
    InstructionFormat::UnaryIeee64, // f64const // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:730
    InstructionFormat::UnaryConst, // f128const // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:730
    InstructionFormat::UnaryConst, // vconst // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:730
    InstructionFormat::Shuffle, // shuffle // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:730
    InstructionFormat::NullAry, // nop // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:730
    InstructionFormat::Ternary, // select // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:730
    InstructionFormat::Ternary, // select_spectre_guard // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:730
    InstructionFormat::Ternary, // bitselect // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:730
    InstructionFormat::Ternary, // blendv // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:730
    InstructionFormat::Unary, // vany_true // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:730
    InstructionFormat::Unary, // vall_true // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:730
    InstructionFormat::Unary, // vhigh_bits // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:730
    InstructionFormat::IntCompare, // icmp // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:730
    InstructionFormat::IntCompareImm, // icmp_imm // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:730
    InstructionFormat::Binary, // iadd // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:730
    InstructionFormat::Binary, // isub // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:730
    InstructionFormat::Unary, // ineg // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:730
    InstructionFormat::Unary, // iabs // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:730
    InstructionFormat::Binary, // imul // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:730
    InstructionFormat::Binary, // umulhi // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:730
    InstructionFormat::Binary, // smulhi // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:730
    InstructionFormat::Binary, // sqmul_round_sat // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:730
    InstructionFormat::Binary, // x86_pmulhrsw // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:730
    InstructionFormat::Binary, // udiv // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:730
    InstructionFormat::Binary, // sdiv // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:730
    InstructionFormat::Binary, // urem // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:730
    InstructionFormat::Binary, // srem // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:730
    InstructionFormat::BinaryImm64, // iadd_imm // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:730
    InstructionFormat::BinaryImm64, // imul_imm // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:730
    InstructionFormat::BinaryImm64, // udiv_imm // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:730
    InstructionFormat::BinaryImm64, // sdiv_imm // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:730
    InstructionFormat::BinaryImm64, // urem_imm // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:730
    InstructionFormat::BinaryImm64, // srem_imm // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:730
    InstructionFormat::BinaryImm64, // irsub_imm // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:730
    InstructionFormat::Ternary, // sadd_overflow_cin // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:730
    InstructionFormat::Ternary, // uadd_overflow_cin // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:730
    InstructionFormat::Binary, // uadd_overflow // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:730
    InstructionFormat::Binary, // sadd_overflow // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:730
    InstructionFormat::Binary, // usub_overflow // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:730
    InstructionFormat::Binary, // ssub_overflow // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:730
    InstructionFormat::Binary, // umul_overflow // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:730
    InstructionFormat::Binary, // smul_overflow // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:730
    InstructionFormat::IntAddTrap, // uadd_overflow_trap // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:730
    InstructionFormat::Ternary, // ssub_overflow_bin // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:730
    InstructionFormat::Ternary, // usub_overflow_bin // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:730
    InstructionFormat::Binary, // band // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:730
    InstructionFormat::Binary, // bor // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:730
    InstructionFormat::Binary, // bxor // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:730
    InstructionFormat::Unary, // bnot // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:730
    InstructionFormat::Binary, // band_not // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:730
    InstructionFormat::Binary, // bor_not // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:730
    InstructionFormat::Binary, // bxor_not // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:730
    InstructionFormat::BinaryImm64, // band_imm // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:730
    InstructionFormat::BinaryImm64, // bor_imm // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:730
    InstructionFormat::BinaryImm64, // bxor_imm // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:730
    InstructionFormat::Binary, // rotl // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:730
    InstructionFormat::Binary, // rotr // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:730
    InstructionFormat::BinaryImm64, // rotl_imm // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:730
    InstructionFormat::BinaryImm64, // rotr_imm // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:730
    InstructionFormat::Binary, // ishl // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:730
    InstructionFormat::Binary, // ushr // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:730
    InstructionFormat::Binary, // sshr // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:730
    InstructionFormat::BinaryImm64, // ishl_imm // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:730
    InstructionFormat::BinaryImm64, // ushr_imm // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:730
    InstructionFormat::BinaryImm64, // sshr_imm // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:730
    InstructionFormat::Unary, // bitrev // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:730
    InstructionFormat::Unary, // clz // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:730
    InstructionFormat::Unary, // cls // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:730
    InstructionFormat::Unary, // ctz // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:730
    InstructionFormat::Unary, // bswap // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:730
    InstructionFormat::Unary, // popcnt // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:730
    InstructionFormat::FloatCompare, // fcmp // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:730
    InstructionFormat::Binary, // fadd // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:730
    InstructionFormat::Binary, // fsub // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:730
    InstructionFormat::Binary, // fmul // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:730
    InstructionFormat::Binary, // fdiv // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:730
    InstructionFormat::Unary, // sqrt // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:730
    InstructionFormat::Ternary, // fma // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:730
    InstructionFormat::Unary, // fneg // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:730
    InstructionFormat::Unary, // fabs // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:730
    InstructionFormat::Binary, // fcopysign // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:730
    InstructionFormat::Binary, // fmin // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:730
    InstructionFormat::Binary, // fmax // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:730
    InstructionFormat::Unary, // ceil // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:730
    InstructionFormat::Unary, // floor // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:730
    InstructionFormat::Unary, // trunc // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:730
    InstructionFormat::Unary, // nearest // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:730
    InstructionFormat::LoadNoOffset, // bitcast // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:730
    InstructionFormat::Unary, // scalar_to_vector // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:730
    InstructionFormat::Unary, // bmask // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:730
    InstructionFormat::Unary, // ireduce // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:730
    InstructionFormat::Binary, // snarrow // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:730
    InstructionFormat::Binary, // unarrow // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:730
    InstructionFormat::Binary, // uunarrow // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:730
    InstructionFormat::Unary, // swiden_low // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:730
    InstructionFormat::Unary, // swiden_high // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:730
    InstructionFormat::Unary, // uwiden_low // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:730
    InstructionFormat::Unary, // uwiden_high // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:730
    InstructionFormat::Binary, // iadd_pairwise // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:730
    InstructionFormat::Binary, // x86_pmaddubsw // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:730
    InstructionFormat::Unary, // uextend // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:730
    InstructionFormat::Unary, // sextend // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:730
    InstructionFormat::Unary, // fpromote // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:730
    InstructionFormat::Unary, // fdemote // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:730
    InstructionFormat::Unary, // fvdemote // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:730
    InstructionFormat::Unary, // fvpromote_low // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:730
    InstructionFormat::Unary, // fcvt_to_uint // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:730
    InstructionFormat::Unary, // fcvt_to_sint // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:730
    InstructionFormat::Unary, // fcvt_to_uint_sat // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:730
    InstructionFormat::Unary, // fcvt_to_sint_sat // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:730
    InstructionFormat::Unary, // x86_cvtt2dq // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:730
    InstructionFormat::Unary, // fcvt_from_uint // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:730
    InstructionFormat::Unary, // fcvt_from_sint // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:730
    InstructionFormat::Unary, // isplit // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:730
    InstructionFormat::Binary, // iconcat // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:730
    InstructionFormat::AtomicRmw, // atomic_rmw // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:730
    InstructionFormat::AtomicCas, // atomic_cas // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:730
    InstructionFormat::LoadNoOffset, // atomic_load // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:730
    InstructionFormat::StoreNoOffset, // atomic_store // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:730
    InstructionFormat::NullAry, // fence // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:730
    InstructionFormat::BinaryImm8, // extract_vector // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:730
    InstructionFormat::NullAry, // sequence_point // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:730
]; // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:738

fn opcode_name(opc: Opcode) -> &'static str {
    match opc {
        Opcode::AtomicCas => {
            "atomic_cas"
        }
        Opcode::AtomicLoad => {
            "atomic_load"
        }
        Opcode::AtomicRmw => {
            "atomic_rmw"
        }
        Opcode::AtomicStore => {
            "atomic_store"
        }
        Opcode::AvgRound => {
            "avg_round"
        }
        Opcode::Band => {
            "band"
        }
        Opcode::BandImm => {
            "band_imm"
        }
        Opcode::BandNot => {
            "band_not"
        }
        Opcode::Bitcast => {
            "bitcast"
        }
        Opcode::Bitrev => {
            "bitrev"
        }
        Opcode::Bitselect => {
            "bitselect"
        }
        Opcode::Blendv => {
            "blendv"
        }
        Opcode::Bmask => {
            "bmask"
        }
        Opcode::Bnot => {
            "bnot"
        }
        Opcode::Bor => {
            "bor"
        }
        Opcode::BorImm => {
            "bor_imm"
        }
        Opcode::BorNot => {
            "bor_not"
        }
        Opcode::BrTable => {
            "br_table"
        }
        Opcode::Brif => {
            "brif"
        }
        Opcode::Bswap => {
            "bswap"
        }
        Opcode::Bxor => {
            "bxor"
        }
        Opcode::BxorImm => {
            "bxor_imm"
        }
        Opcode::BxorNot => {
            "bxor_not"
        }
        Opcode::Call => {
            "call"
        }
        Opcode::CallIndirect => {
            "call_indirect"
        }
        Opcode::Ceil => {
            "ceil"
        }
        Opcode::Cls => {
            "cls"
        }
        Opcode::Clz => {
            "clz"
        }
        Opcode::Ctz => {
            "ctz"
        }
        Opcode::Debugtrap => {
            "debugtrap"
        }
        Opcode::DynamicStackAddr => {
            "dynamic_stack_addr"
        }
        Opcode::DynamicStackLoad => {
            "dynamic_stack_load"
        }
        Opcode::DynamicStackStore => {
            "dynamic_stack_store"
        }
        Opcode::ExtractVector => {
            "extract_vector"
        }
        Opcode::Extractlane => {
            "extractlane"
        }
        Opcode::F128const => {
            "f128const"
        }
        Opcode::F16const => {
            "f16const"
        }
        Opcode::F32const => {
            "f32const"
        }
        Opcode::F64const => {
            "f64const"
        }
        Opcode::Fabs => {
            "fabs"
        }
        Opcode::Fadd => {
            "fadd"
        }
        Opcode::Fcmp => {
            "fcmp"
        }
        Opcode::Fcopysign => {
            "fcopysign"
        }
        Opcode::FcvtFromSint => {
            "fcvt_from_sint"
        }
        Opcode::FcvtFromUint => {
            "fcvt_from_uint"
        }
        Opcode::FcvtToSint => {
            "fcvt_to_sint"
        }
        Opcode::FcvtToSintSat => {
            "fcvt_to_sint_sat"
        }
        Opcode::FcvtToUint => {
            "fcvt_to_uint"
        }
        Opcode::FcvtToUintSat => {
            "fcvt_to_uint_sat"
        }
        Opcode::Fdemote => {
            "fdemote"
        }
        Opcode::Fdiv => {
            "fdiv"
        }
        Opcode::Fence => {
            "fence"
        }
        Opcode::Floor => {
            "floor"
        }
        Opcode::Fma => {
            "fma"
        }
        Opcode::Fmax => {
            "fmax"
        }
        Opcode::Fmin => {
            "fmin"
        }
        Opcode::Fmul => {
            "fmul"
        }
        Opcode::Fneg => {
            "fneg"
        }
        Opcode::Fpromote => {
            "fpromote"
        }
        Opcode::Fsub => {
            "fsub"
        }
        Opcode::FuncAddr => {
            "func_addr"
        }
        Opcode::Fvdemote => {
            "fvdemote"
        }
        Opcode::FvpromoteLow => {
            "fvpromote_low"
        }
        Opcode::GetExceptionHandlerAddress => {
            "get_exception_handler_address"
        }
        Opcode::GetFramePointer => {
            "get_frame_pointer"
        }
        Opcode::GetPinnedReg => {
            "get_pinned_reg"
        }
        Opcode::GetReturnAddress => {
            "get_return_address"
        }
        Opcode::GetStackPointer => {
            "get_stack_pointer"
        }
        Opcode::GlobalValue => {
            "global_value"
        }
        Opcode::Iabs => {
            "iabs"
        }
        Opcode::Iadd => {
            "iadd"
        }
        Opcode::IaddImm => {
            "iadd_imm"
        }
        Opcode::IaddPairwise => {
            "iadd_pairwise"
        }
        Opcode::Icmp => {
            "icmp"
        }
        Opcode::IcmpImm => {
            "icmp_imm"
        }
        Opcode::Iconcat => {
            "iconcat"
        }
        Opcode::Iconst => {
            "iconst"
        }
        Opcode::Imul => {
            "imul"
        }
        Opcode::ImulImm => {
            "imul_imm"
        }
        Opcode::Ineg => {
            "ineg"
        }
        Opcode::Insertlane => {
            "insertlane"
        }
        Opcode::Ireduce => {
            "ireduce"
        }
        Opcode::IrsubImm => {
            "irsub_imm"
        }
        Opcode::Ishl => {
            "ishl"
        }
        Opcode::IshlImm => {
            "ishl_imm"
        }
        Opcode::Isplit => {
            "isplit"
        }
        Opcode::Istore16 => {
            "istore16"
        }
        Opcode::Istore32 => {
            "istore32"
        }
        Opcode::Istore8 => {
            "istore8"
        }
        Opcode::Isub => {
            "isub"
        }
        Opcode::Jump => {
            "jump"
        }
        Opcode::Load => {
            "load"
        }
        Opcode::Nearest => {
            "nearest"
        }
        Opcode::Nop => {
            "nop"
        }
        Opcode::Popcnt => {
            "popcnt"
        }
        Opcode::Return => {
            "return"
        }
        Opcode::ReturnCall => {
            "return_call"
        }
        Opcode::ReturnCallIndirect => {
            "return_call_indirect"
        }
        Opcode::Rotl => {
            "rotl"
        }
        Opcode::RotlImm => {
            "rotl_imm"
        }
        Opcode::Rotr => {
            "rotr"
        }
        Opcode::RotrImm => {
            "rotr_imm"
        }
        Opcode::SaddOverflow => {
            "sadd_overflow"
        }
        Opcode::SaddOverflowCin => {
            "sadd_overflow_cin"
        }
        Opcode::SaddSat => {
            "sadd_sat"
        }
        Opcode::ScalarToVector => {
            "scalar_to_vector"
        }
        Opcode::Sdiv => {
            "sdiv"
        }
        Opcode::SdivImm => {
            "sdiv_imm"
        }
        Opcode::Select => {
            "select"
        }
        Opcode::SelectSpectreGuard => {
            "select_spectre_guard"
        }
        Opcode::SequencePoint => {
            "sequence_point"
        }
        Opcode::SetPinnedReg => {
            "set_pinned_reg"
        }
        Opcode::Sextend => {
            "sextend"
        }
        Opcode::Shuffle => {
            "shuffle"
        }
        Opcode::Sload16 => {
            "sload16"
        }
        Opcode::Sload16x4 => {
            "sload16x4"
        }
        Opcode::Sload32 => {
            "sload32"
        }
        Opcode::Sload32x2 => {
            "sload32x2"
        }
        Opcode::Sload8 => {
            "sload8"
        }
        Opcode::Sload8x8 => {
            "sload8x8"
        }
        Opcode::Smax => {
            "smax"
        }
        Opcode::Smin => {
            "smin"
        }
        Opcode::SmulOverflow => {
            "smul_overflow"
        }
        Opcode::Smulhi => {
            "smulhi"
        }
        Opcode::Snarrow => {
            "snarrow"
        }
        Opcode::Splat => {
            "splat"
        }
        Opcode::SqmulRoundSat => {
            "sqmul_round_sat"
        }
        Opcode::Sqrt => {
            "sqrt"
        }
        Opcode::Srem => {
            "srem"
        }
        Opcode::SremImm => {
            "srem_imm"
        }
        Opcode::Sshr => {
            "sshr"
        }
        Opcode::SshrImm => {
            "sshr_imm"
        }
        Opcode::SsubOverflow => {
            "ssub_overflow"
        }
        Opcode::SsubOverflowBin => {
            "ssub_overflow_bin"
        }
        Opcode::SsubSat => {
            "ssub_sat"
        }
        Opcode::StackAddr => {
            "stack_addr"
        }
        Opcode::StackLoad => {
            "stack_load"
        }
        Opcode::StackStore => {
            "stack_store"
        }
        Opcode::StackSwitch => {
            "stack_switch"
        }
        Opcode::Store => {
            "store"
        }
        Opcode::SwidenHigh => {
            "swiden_high"
        }
        Opcode::SwidenLow => {
            "swiden_low"
        }
        Opcode::Swizzle => {
            "swizzle"
        }
        Opcode::SymbolValue => {
            "symbol_value"
        }
        Opcode::TlsValue => {
            "tls_value"
        }
        Opcode::Trap => {
            "trap"
        }
        Opcode::Trapnz => {
            "trapnz"
        }
        Opcode::Trapz => {
            "trapz"
        }
        Opcode::Trunc => {
            "trunc"
        }
        Opcode::TryCall => {
            "try_call"
        }
        Opcode::TryCallIndirect => {
            "try_call_indirect"
        }
        Opcode::UaddOverflow => {
            "uadd_overflow"
        }
        Opcode::UaddOverflowCin => {
            "uadd_overflow_cin"
        }
        Opcode::UaddOverflowTrap => {
            "uadd_overflow_trap"
        }
        Opcode::UaddSat => {
            "uadd_sat"
        }
        Opcode::Udiv => {
            "udiv"
        }
        Opcode::UdivImm => {
            "udiv_imm"
        }
        Opcode::Uextend => {
            "uextend"
        }
        Opcode::Uload16 => {
            "uload16"
        }
        Opcode::Uload16x4 => {
            "uload16x4"
        }
        Opcode::Uload32 => {
            "uload32"
        }
        Opcode::Uload32x2 => {
            "uload32x2"
        }
        Opcode::Uload8 => {
            "uload8"
        }
        Opcode::Uload8x8 => {
            "uload8x8"
        }
        Opcode::Umax => {
            "umax"
        }
        Opcode::Umin => {
            "umin"
        }
        Opcode::UmulOverflow => {
            "umul_overflow"
        }
        Opcode::Umulhi => {
            "umulhi"
        }
        Opcode::Unarrow => {
            "unarrow"
        }
        Opcode::Urem => {
            "urem"
        }
        Opcode::UremImm => {
            "urem_imm"
        }
        Opcode::Ushr => {
            "ushr"
        }
        Opcode::UshrImm => {
            "ushr_imm"
        }
        Opcode::UsubOverflow => {
            "usub_overflow"
        }
        Opcode::UsubOverflowBin => {
            "usub_overflow_bin"
        }
        Opcode::UsubSat => {
            "usub_sat"
        }
        Opcode::Uunarrow => {
            "uunarrow"
        }
        Opcode::UwidenHigh => {
            "uwiden_high"
        }
        Opcode::UwidenLow => {
            "uwiden_low"
        }
        Opcode::VallTrue => {
            "vall_true"
        }
        Opcode::VanyTrue => {
            "vany_true"
        }
        Opcode::Vconst => {
            "vconst"
        }
        Opcode::VhighBits => {
            "vhigh_bits"
        }
        Opcode::X86Cvtt2dq => {
            "x86_cvtt2dq"
        }
        Opcode::X86Pmaddubsw => {
            "x86_pmaddubsw"
        }
        Opcode::X86Pmulhrsw => {
            "x86_pmulhrsw"
        }
        Opcode::X86Pshufb => {
            "x86_pshufb"
        }
    }
}

const OPCODE_HASH_TABLE: [Option<Opcode>; 256] = [ // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:759
    Some(Opcode::Imul), // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:767
    Some(Opcode::TlsValue), // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:767
    None, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:768
    Some(Opcode::Brif), // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:767
    Some(Opcode::Nearest), // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:767
    Some(Opcode::FcvtToSintSat), // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:767
    Some(Opcode::Fsub), // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:767
    Some(Opcode::Trunc), // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:767
    Some(Opcode::Urem), // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:767
    Some(Opcode::Iconst), // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:767
    Some(Opcode::ReturnCall), // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:767
    Some(Opcode::Umin), // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:767
    None, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:768
    Some(Opcode::Store), // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:767
    Some(Opcode::GetFramePointer), // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:767
    Some(Opcode::UshrImm), // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:767
    Some(Opcode::Isub), // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:767
    Some(Opcode::FcvtFromSint), // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:767
    Some(Opcode::Trap), // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:767
    Some(Opcode::Sdiv), // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:767
    Some(Opcode::Srem), // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:767
    Some(Opcode::SshrImm), // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:767
    Some(Opcode::Uunarrow), // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:767
    Some(Opcode::UaddOverflowCin), // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:767
    Some(Opcode::Bxor), // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:767
    None, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:768
    Some(Opcode::X86Pmaddubsw), // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:767
    Some(Opcode::Umax), // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:767
    Some(Opcode::SremImm), // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:767
    Some(Opcode::Insertlane), // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:767
    Some(Opcode::BxorNot), // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:767
    Some(Opcode::Swizzle), // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:767
    Some(Opcode::Load), // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:767
    Some(Opcode::Fadd), // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:767
    Some(Opcode::Jump), // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:767
    Some(Opcode::BxorImm), // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:767
    Some(Opcode::Shuffle), // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:767
    Some(Opcode::Fneg), // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:767
    Some(Opcode::Umulhi), // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:767
    Some(Opcode::Ushr), // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:767
    None, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:768
    Some(Opcode::UaddOverflowTrap), // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:767
    Some(Opcode::FcvtFromUint), // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:767
    Some(Opcode::VallTrue), // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:767
    Some(Opcode::Band), // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:767
    None, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:768
    Some(Opcode::SsubOverflow), // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:767
    Some(Opcode::Uload16x4), // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:767
    Some(Opcode::Ishl), // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:767
    Some(Opcode::Fmax), // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:767
    Some(Opcode::Vconst), // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:767
    Some(Opcode::Call), // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:767
    Some(Opcode::ExtractVector), // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:767
    Some(Opcode::Sqrt), // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:767
    None, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:768
    None, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:768
    Some(Opcode::Ceil), // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:767
    Some(Opcode::Ineg), // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:767
    Some(Opcode::FuncAddr), // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:767
    Some(Opcode::SaddSat), // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:767
    Some(Opcode::Popcnt), // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:767
    None, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:768
    Some(Opcode::Fabs), // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:767
    Some(Opcode::Fmin), // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:767
    Some(Opcode::SsubOverflowBin), // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:767
    Some(Opcode::GlobalValue), // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:767
    Some(Opcode::Bnot), // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:767
    Some(Opcode::Sextend), // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:767
    Some(Opcode::Isplit), // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:767
    Some(Opcode::FcvtToUint), // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:767
    None, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:768
    None, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:768
    Some(Opcode::RotlImm), // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:767
    Some(Opcode::Fcmp), // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:767
    Some(Opcode::SwidenHigh), // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:767
    Some(Opcode::Fmul), // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:767
    Some(Opcode::FcvtToSint), // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:767
    None, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:768
    Some(Opcode::UsubOverflow), // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:767
    Some(Opcode::Uload8x8), // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:767
    None, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:768
    None, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:768
    Some(Opcode::Fdiv), // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:767
    None, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:768
    None, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:768
    None, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:768
    Some(Opcode::UremImm), // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:767
    Some(Opcode::AtomicLoad), // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:767
    None, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:768
    Some(Opcode::Trapnz), // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:767
    Some(Opcode::Uload16), // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:767
    Some(Opcode::IaddImm), // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:767
    Some(Opcode::Uload32), // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:767
    Some(Opcode::Bitrev), // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:767
    None, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:768
    None, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:768
    Some(Opcode::Smulhi), // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:767
    Some(Opcode::TryCall), // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:767
    None, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:768
    None, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:768
    Some(Opcode::Blendv), // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:767
    None, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:768
    None, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:768
    Some(Opcode::BorNot), // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:767
    None, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:768
    None, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:768
    Some(Opcode::Sload8x8), // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:767
    None, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:768
    None, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:768
    None, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:768
    None, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:768
    Some(Opcode::SetPinnedReg), // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:767
    None, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:768
    None, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:768
    None, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:768
    None, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:768
    None, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:768
    Some(Opcode::ImulImm), // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:767
    Some(Opcode::Ireduce), // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:767
    None, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:768
    Some(Opcode::RotrImm), // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:767
    Some(Opcode::DynamicStackStore), // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:767
    Some(Opcode::StackStore), // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:767
    Some(Opcode::UwidenLow), // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:767
    Some(Opcode::Select), // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:767
    Some(Opcode::BorImm), // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:767
    Some(Opcode::Istore32), // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:767
    Some(Opcode::FvpromoteLow), // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:767
    Some(Opcode::Istore16), // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:767
    None, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:768
    Some(Opcode::Fdemote), // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:767
    None, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:768
    None, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:768
    Some(Opcode::IcmpImm), // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:767
    Some(Opcode::Fvdemote), // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:767
    None, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:768
    Some(Opcode::Sload16), // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:767
    Some(Opcode::Fcopysign), // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:767
    None, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:768
    Some(Opcode::SdivImm), // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:767
    Some(Opcode::Unarrow), // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:767
    Some(Opcode::AvgRound), // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:767
    Some(Opcode::Sload32), // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:767
    None, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:768
    Some(Opcode::X86Pshufb), // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:767
    Some(Opcode::Extractlane), // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:767
    Some(Opcode::StackAddr), // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:767
    Some(Opcode::SaddOverflowCin), // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:767
    Some(Opcode::UaddOverflow), // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:767
    Some(Opcode::BandImm), // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:767
    Some(Opcode::Return), // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:767
    None, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:768
    Some(Opcode::Uload32x2), // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:767
    None, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:768
    None, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:768
    Some(Opcode::VanyTrue), // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:767
    None, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:768
    Some(Opcode::UsubSat), // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:767
    Some(Opcode::GetExceptionHandlerAddress), // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:767
    None, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:768
    None, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:768
    None, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:768
    Some(Opcode::DynamicStackLoad), // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:767
    Some(Opcode::Iconcat), // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:767
    Some(Opcode::SmulOverflow), // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:767
    None, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:768
    Some(Opcode::Fence), // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:767
    None, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:768
    None, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:768
    Some(Opcode::Fma), // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:767
    Some(Opcode::Bitselect), // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:767
    Some(Opcode::Istore8), // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:767
    Some(Opcode::BrTable), // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:767
    Some(Opcode::F64const), // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:767
    Some(Opcode::StackSwitch), // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:767
    Some(Opcode::StackLoad), // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:767
    Some(Opcode::IrsubImm), // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:767
    Some(Opcode::Nop), // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:767
    Some(Opcode::SqmulRoundSat), // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:767
    Some(Opcode::X86Pmulhrsw), // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:767
    Some(Opcode::Debugtrap), // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:767
    Some(Opcode::Sload16x4), // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:767
    Some(Opcode::UmulOverflow), // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:767
    Some(Opcode::IshlImm), // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:767
    Some(Opcode::SaddOverflow), // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:767
    Some(Opcode::Ctz), // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:767
    Some(Opcode::Bor), // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:767
    Some(Opcode::TryCallIndirect), // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:767
    Some(Opcode::BandNot), // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:767
    Some(Opcode::Clz), // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:767
    Some(Opcode::UwidenHigh), // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:767
    Some(Opcode::Uextend), // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:767
    Some(Opcode::Floor), // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:767
    Some(Opcode::UaddSat), // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:767
    Some(Opcode::Sload32x2), // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:767
    Some(Opcode::SequencePoint), // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:767
    None, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:768
    Some(Opcode::SelectSpectreGuard), // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:767
    Some(Opcode::Cls), // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:767
    Some(Opcode::Fpromote), // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:767
    Some(Opcode::Bitcast), // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:767
    None, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:768
    Some(Opcode::SymbolValue), // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:767
    Some(Opcode::DynamicStackAddr), // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:767
    Some(Opcode::Bmask), // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:767
    Some(Opcode::GetPinnedReg), // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:767
    Some(Opcode::SsubSat), // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:767
    Some(Opcode::AtomicRmw), // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:767
    None, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:768
    Some(Opcode::ScalarToVector), // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:767
    None, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:768
    None, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:768
    Some(Opcode::Uload8), // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:767
    Some(Opcode::FcvtToUintSat), // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:767
    None, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:768
    Some(Opcode::Smin), // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:767
    Some(Opcode::Trapz), // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:767
    Some(Opcode::Iabs), // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:767
    Some(Opcode::F16const), // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:767
    Some(Opcode::Udiv), // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:767
    Some(Opcode::AtomicCas), // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:767
    Some(Opcode::GetReturnAddress), // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:767
    Some(Opcode::UsubOverflowBin), // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:767
    Some(Opcode::SwidenLow), // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:767
    None, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:768
    Some(Opcode::ReturnCallIndirect), // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:767
    Some(Opcode::Rotl), // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:767
    Some(Opcode::IaddPairwise), // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:767
    None, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:768
    Some(Opcode::Smax), // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:767
    Some(Opcode::F128const), // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:767
    None, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:768
    Some(Opcode::F32const), // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:767
    Some(Opcode::UdivImm), // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:767
    None, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:768
    Some(Opcode::Splat), // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:767
    Some(Opcode::Rotr), // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:767
    Some(Opcode::Snarrow), // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:767
    Some(Opcode::CallIndirect), // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:767
    Some(Opcode::Sload8), // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:767
    Some(Opcode::X86Cvtt2dq), // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:767
    Some(Opcode::VhighBits), // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:767
    None, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:768
    None, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:768
    Some(Opcode::Iadd), // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:767
    Some(Opcode::Icmp), // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:767
    None, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:768
    None, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:768
    None, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:768
    None, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:768
    Some(Opcode::GetStackPointer), // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:767
    Some(Opcode::Bswap), // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:767
    None, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:768
    Some(Opcode::Sshr), // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:767
    Some(Opcode::AtomicStore), // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:767
    None, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:768
]; // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:772


// Table of opcode constraints.
const OPCODE_CONSTRAINTS: [OpcodeConstraints; 187] = [ // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:893
    // Jump: fixed_results=0, use_typevar_operand=false, requires_typevar_operand=false, fixed_values=0
    // Constraints=[]
    OpcodeConstraints {
        flags: 0x00, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:964
        typeset_offset: 255, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:965
        constraint_offset: 0, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:966
    }
    ,
    // Brif: fixed_results=0, use_typevar_operand=true, requires_typevar_operand=true, fixed_values=1
    // Constraints=['Same']
    // Polymorphic over TypeSet(lanes={1}, ints={8, 16, 32, 64, 128})
    OpcodeConstraints {
        flags: 0x38, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:964
        typeset_offset: 0, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:965
        constraint_offset: 0, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:966
    }
    ,
    // BrTable: fixed_results=0, use_typevar_operand=false, requires_typevar_operand=false, fixed_values=1
    // Constraints=['Concrete(ir::types::I32)']
    OpcodeConstraints {
        flags: 0x20, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:964
        typeset_offset: 255, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:965
        constraint_offset: 3, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:966
    }
    ,
    // Debugtrap: fixed_results=0, use_typevar_operand=false, requires_typevar_operand=false, fixed_values=0
    // Constraints=[]
    OpcodeConstraints {
        flags: 0x00, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:964
        typeset_offset: 255, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:965
        constraint_offset: 0, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:966
    }
    ,
    // Trap: fixed_results=0, use_typevar_operand=false, requires_typevar_operand=false, fixed_values=0
    // Constraints=[]
    OpcodeConstraints {
        flags: 0x00, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:964
        typeset_offset: 255, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:965
        constraint_offset: 0, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:966
    }
    ,
    // Trapz: fixed_results=0, use_typevar_operand=true, requires_typevar_operand=true, fixed_values=1
    // Constraints=['Same']
    // Polymorphic over TypeSet(lanes={1}, ints={8, 16, 32, 64, 128})
    OpcodeConstraints {
        flags: 0x38, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:964
        typeset_offset: 0, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:965
        constraint_offset: 0, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:966
    }
    ,
    // Trapnz: fixed_results=0, use_typevar_operand=true, requires_typevar_operand=true, fixed_values=1
    // Constraints=['Same']
    // Polymorphic over TypeSet(lanes={1}, ints={8, 16, 32, 64, 128})
    OpcodeConstraints {
        flags: 0x38, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:964
        typeset_offset: 0, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:965
        constraint_offset: 0, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:966
    }
    ,
    // Return: fixed_results=0, use_typevar_operand=false, requires_typevar_operand=false, fixed_values=0
    // Constraints=[]
    OpcodeConstraints {
        flags: 0x00, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:964
        typeset_offset: 255, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:965
        constraint_offset: 0, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:966
    }
    ,
    // Call: fixed_results=0, use_typevar_operand=false, requires_typevar_operand=false, fixed_values=0
    // Constraints=[]
    OpcodeConstraints {
        flags: 0x00, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:964
        typeset_offset: 255, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:965
        constraint_offset: 0, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:966
    }
    ,
    // CallIndirect: fixed_results=0, use_typevar_operand=true, requires_typevar_operand=true, fixed_values=1
    // Constraints=['Same']
    // Polymorphic over TypeSet(lanes={1}, ints={32, 64})
    OpcodeConstraints {
        flags: 0x38, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:964
        typeset_offset: 1, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:965
        constraint_offset: 0, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:966
    }
    ,
    // ReturnCall: fixed_results=0, use_typevar_operand=false, requires_typevar_operand=false, fixed_values=0
    // Constraints=[]
    OpcodeConstraints {
        flags: 0x00, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:964
        typeset_offset: 255, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:965
        constraint_offset: 0, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:966
    }
    ,
    // ReturnCallIndirect: fixed_results=0, use_typevar_operand=true, requires_typevar_operand=true, fixed_values=1
    // Constraints=['Same']
    // Polymorphic over TypeSet(lanes={1}, ints={32, 64})
    OpcodeConstraints {
        flags: 0x38, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:964
        typeset_offset: 1, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:965
        constraint_offset: 0, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:966
    }
    ,
    // FuncAddr: fixed_results=1, use_typevar_operand=false, requires_typevar_operand=false, fixed_values=0
    // Constraints=['Same']
    // Polymorphic over TypeSet(lanes={1}, ints={32, 64})
    OpcodeConstraints {
        flags: 0x01, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:964
        typeset_offset: 1, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:965
        constraint_offset: 0, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:966
    }
    ,
    // TryCall: fixed_results=0, use_typevar_operand=false, requires_typevar_operand=false, fixed_values=0
    // Constraints=[]
    OpcodeConstraints {
        flags: 0x00, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:964
        typeset_offset: 255, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:965
        constraint_offset: 0, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:966
    }
    ,
    // TryCallIndirect: fixed_results=0, use_typevar_operand=true, requires_typevar_operand=true, fixed_values=1
    // Constraints=['Same']
    // Polymorphic over TypeSet(lanes={1}, ints={32, 64})
    OpcodeConstraints {
        flags: 0x38, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:964
        typeset_offset: 1, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:965
        constraint_offset: 0, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:966
    }
    ,
    // Splat: fixed_results=1, use_typevar_operand=false, requires_typevar_operand=false, fixed_values=1
    // Constraints=['Same', 'LaneOf']
    // Polymorphic over TypeSet(lanes={2, 4, 8, 16, 32, 64, 128, 256}, ints={8, 16, 32, 64, 128}, floats={16, 32, 64, 128})
    OpcodeConstraints {
        flags: 0x21, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:964
        typeset_offset: 2, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:965
        constraint_offset: 4, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:966
    }
    ,
    // Swizzle: fixed_results=1, use_typevar_operand=false, requires_typevar_operand=false, fixed_values=2
    // Constraints=['Concrete(ir::types::I8X16)', 'Concrete(ir::types::I8X16)', 'Concrete(ir::types::I8X16)']
    OpcodeConstraints {
        flags: 0x41, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:964
        typeset_offset: 255, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:965
        constraint_offset: 6, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:966
    }
    ,
    // X86Pshufb: fixed_results=1, use_typevar_operand=false, requires_typevar_operand=false, fixed_values=2
    // Constraints=['Concrete(ir::types::I8X16)', 'Concrete(ir::types::I8X16)', 'Concrete(ir::types::I8X16)']
    OpcodeConstraints {
        flags: 0x41, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:964
        typeset_offset: 255, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:965
        constraint_offset: 6, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:966
    }
    ,
    // Insertlane: fixed_results=1, use_typevar_operand=true, requires_typevar_operand=false, fixed_values=2
    // Constraints=['Same', 'Same', 'LaneOf']
    // Polymorphic over TypeSet(lanes={2, 4, 8, 16, 32, 64, 128, 256}, ints={8, 16, 32, 64, 128}, floats={16, 32, 64, 128})
    OpcodeConstraints {
        flags: 0x49, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:964
        typeset_offset: 2, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:965
        constraint_offset: 9, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:966
    }
    ,
    // Extractlane: fixed_results=1, use_typevar_operand=true, requires_typevar_operand=true, fixed_values=1
    // Constraints=['LaneOf', 'Same']
    // Polymorphic over TypeSet(lanes={2, 4, 8, 16, 32, 64, 128, 256}, ints={8, 16, 32, 64, 128}, floats={16, 32, 64, 128})
    OpcodeConstraints {
        flags: 0x39, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:964
        typeset_offset: 2, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:965
        constraint_offset: 11, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:966
    }
    ,
    // Smin: fixed_results=1, use_typevar_operand=true, requires_typevar_operand=false, fixed_values=2
    // Constraints=['Same', 'Same', 'Same']
    // Polymorphic over TypeSet(lanes={1, 2, 4, 8, 16, 32, 64, 128, 256}, ints={8, 16, 32, 64, 128})
    OpcodeConstraints {
        flags: 0x49, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:964
        typeset_offset: 3, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:965
        constraint_offset: 0, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:966
    }
    ,
    // Umin: fixed_results=1, use_typevar_operand=true, requires_typevar_operand=false, fixed_values=2
    // Constraints=['Same', 'Same', 'Same']
    // Polymorphic over TypeSet(lanes={1, 2, 4, 8, 16, 32, 64, 128, 256}, ints={8, 16, 32, 64, 128})
    OpcodeConstraints {
        flags: 0x49, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:964
        typeset_offset: 3, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:965
        constraint_offset: 0, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:966
    }
    ,
    // Smax: fixed_results=1, use_typevar_operand=true, requires_typevar_operand=false, fixed_values=2
    // Constraints=['Same', 'Same', 'Same']
    // Polymorphic over TypeSet(lanes={1, 2, 4, 8, 16, 32, 64, 128, 256}, ints={8, 16, 32, 64, 128})
    OpcodeConstraints {
        flags: 0x49, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:964
        typeset_offset: 3, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:965
        constraint_offset: 0, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:966
    }
    ,
    // Umax: fixed_results=1, use_typevar_operand=true, requires_typevar_operand=false, fixed_values=2
    // Constraints=['Same', 'Same', 'Same']
    // Polymorphic over TypeSet(lanes={1, 2, 4, 8, 16, 32, 64, 128, 256}, ints={8, 16, 32, 64, 128})
    OpcodeConstraints {
        flags: 0x49, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:964
        typeset_offset: 3, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:965
        constraint_offset: 0, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:966
    }
    ,
    // AvgRound: fixed_results=1, use_typevar_operand=true, requires_typevar_operand=false, fixed_values=2
    // Constraints=['Same', 'Same', 'Same']
    // Polymorphic over TypeSet(lanes={2, 4, 8, 16, 32, 64, 128, 256}, ints={8, 16, 32, 64, 128})
    OpcodeConstraints {
        flags: 0x49, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:964
        typeset_offset: 4, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:965
        constraint_offset: 0, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:966
    }
    ,
    // UaddSat: fixed_results=1, use_typevar_operand=true, requires_typevar_operand=false, fixed_values=2
    // Constraints=['Same', 'Same', 'Same']
    // Polymorphic over TypeSet(lanes={2, 4, 8, 16, 32, 64, 128, 256}, ints={8, 16, 32, 64, 128})
    OpcodeConstraints {
        flags: 0x49, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:964
        typeset_offset: 4, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:965
        constraint_offset: 0, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:966
    }
    ,
    // SaddSat: fixed_results=1, use_typevar_operand=true, requires_typevar_operand=false, fixed_values=2
    // Constraints=['Same', 'Same', 'Same']
    // Polymorphic over TypeSet(lanes={2, 4, 8, 16, 32, 64, 128, 256}, ints={8, 16, 32, 64, 128})
    OpcodeConstraints {
        flags: 0x49, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:964
        typeset_offset: 4, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:965
        constraint_offset: 0, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:966
    }
    ,
    // UsubSat: fixed_results=1, use_typevar_operand=true, requires_typevar_operand=false, fixed_values=2
    // Constraints=['Same', 'Same', 'Same']
    // Polymorphic over TypeSet(lanes={2, 4, 8, 16, 32, 64, 128, 256}, ints={8, 16, 32, 64, 128})
    OpcodeConstraints {
        flags: 0x49, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:964
        typeset_offset: 4, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:965
        constraint_offset: 0, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:966
    }
    ,
    // SsubSat: fixed_results=1, use_typevar_operand=true, requires_typevar_operand=false, fixed_values=2
    // Constraints=['Same', 'Same', 'Same']
    // Polymorphic over TypeSet(lanes={2, 4, 8, 16, 32, 64, 128, 256}, ints={8, 16, 32, 64, 128})
    OpcodeConstraints {
        flags: 0x49, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:964
        typeset_offset: 4, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:965
        constraint_offset: 0, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:966
    }
    ,
    // Load: fixed_results=1, use_typevar_operand=false, requires_typevar_operand=false, fixed_values=1
    // Constraints=['Same', 'Free(1)']
    // Polymorphic over TypeSet(lanes={1, 2, 4, 8, 16, 32, 64, 128, 256}, ints={8, 16, 32, 64, 128}, floats={16, 32, 64, 128})
    OpcodeConstraints {
        flags: 0x21, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:964
        typeset_offset: 5, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:965
        constraint_offset: 12, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:966
    }
    ,
    // Store: fixed_results=0, use_typevar_operand=true, requires_typevar_operand=true, fixed_values=2
    // Constraints=['Same', 'Free(1)']
    // Polymorphic over TypeSet(lanes={1, 2, 4, 8, 16, 32, 64, 128, 256}, ints={8, 16, 32, 64, 128}, floats={16, 32, 64, 128})
    OpcodeConstraints {
        flags: 0x58, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:964
        typeset_offset: 5, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:965
        constraint_offset: 12, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:966
    }
    ,
    // Uload8: fixed_results=1, use_typevar_operand=false, requires_typevar_operand=false, fixed_values=1
    // Constraints=['Same', 'Free(1)']
    // Polymorphic over TypeSet(lanes={1}, ints={16, 32, 64})
    OpcodeConstraints {
        flags: 0x21, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:964
        typeset_offset: 6, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:965
        constraint_offset: 12, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:966
    }
    ,
    // Sload8: fixed_results=1, use_typevar_operand=false, requires_typevar_operand=false, fixed_values=1
    // Constraints=['Same', 'Free(1)']
    // Polymorphic over TypeSet(lanes={1}, ints={16, 32, 64})
    OpcodeConstraints {
        flags: 0x21, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:964
        typeset_offset: 6, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:965
        constraint_offset: 12, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:966
    }
    ,
    // Istore8: fixed_results=0, use_typevar_operand=true, requires_typevar_operand=true, fixed_values=2
    // Constraints=['Same', 'Free(1)']
    // Polymorphic over TypeSet(lanes={1}, ints={16, 32, 64})
    OpcodeConstraints {
        flags: 0x58, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:964
        typeset_offset: 6, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:965
        constraint_offset: 12, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:966
    }
    ,
    // Uload16: fixed_results=1, use_typevar_operand=false, requires_typevar_operand=false, fixed_values=1
    // Constraints=['Same', 'Free(1)']
    // Polymorphic over TypeSet(lanes={1}, ints={32, 64})
    OpcodeConstraints {
        flags: 0x21, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:964
        typeset_offset: 1, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:965
        constraint_offset: 12, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:966
    }
    ,
    // Sload16: fixed_results=1, use_typevar_operand=false, requires_typevar_operand=false, fixed_values=1
    // Constraints=['Same', 'Free(1)']
    // Polymorphic over TypeSet(lanes={1}, ints={32, 64})
    OpcodeConstraints {
        flags: 0x21, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:964
        typeset_offset: 1, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:965
        constraint_offset: 12, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:966
    }
    ,
    // Istore16: fixed_results=0, use_typevar_operand=true, requires_typevar_operand=true, fixed_values=2
    // Constraints=['Same', 'Free(1)']
    // Polymorphic over TypeSet(lanes={1}, ints={32, 64})
    OpcodeConstraints {
        flags: 0x58, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:964
        typeset_offset: 1, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:965
        constraint_offset: 12, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:966
    }
    ,
    // Uload32: fixed_results=1, use_typevar_operand=true, requires_typevar_operand=true, fixed_values=1
    // Constraints=['Concrete(ir::types::I64)', 'Same']
    // Polymorphic over TypeSet(lanes={1}, ints={32, 64})
    OpcodeConstraints {
        flags: 0x39, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:964
        typeset_offset: 1, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:965
        constraint_offset: 14, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:966
    }
    ,
    // Sload32: fixed_results=1, use_typevar_operand=true, requires_typevar_operand=true, fixed_values=1
    // Constraints=['Concrete(ir::types::I64)', 'Same']
    // Polymorphic over TypeSet(lanes={1}, ints={32, 64})
    OpcodeConstraints {
        flags: 0x39, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:964
        typeset_offset: 1, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:965
        constraint_offset: 14, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:966
    }
    ,
    // Istore32: fixed_results=0, use_typevar_operand=true, requires_typevar_operand=true, fixed_values=2
    // Constraints=['Concrete(ir::types::I64)', 'Free(1)']
    // Polymorphic over TypeSet(lanes={1}, ints={64})
    OpcodeConstraints {
        flags: 0x58, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:964
        typeset_offset: 7, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:965
        constraint_offset: 16, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:966
    }
    ,
    // StackSwitch: fixed_results=1, use_typevar_operand=true, requires_typevar_operand=false, fixed_values=3
    // Constraints=['Same', 'Same', 'Same', 'Same']
    // Polymorphic over TypeSet(lanes={1}, ints={32, 64})
    OpcodeConstraints {
        flags: 0x69, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:964
        typeset_offset: 1, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:965
        constraint_offset: 18, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:966
    }
    ,
    // Uload8x8: fixed_results=1, use_typevar_operand=true, requires_typevar_operand=true, fixed_values=1
    // Constraints=['Concrete(ir::types::I16X8)', 'Same']
    // Polymorphic over TypeSet(lanes={1}, ints={32, 64})
    OpcodeConstraints {
        flags: 0x39, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:964
        typeset_offset: 1, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:965
        constraint_offset: 22, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:966
    }
    ,
    // Sload8x8: fixed_results=1, use_typevar_operand=true, requires_typevar_operand=true, fixed_values=1
    // Constraints=['Concrete(ir::types::I16X8)', 'Same']
    // Polymorphic over TypeSet(lanes={1}, ints={32, 64})
    OpcodeConstraints {
        flags: 0x39, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:964
        typeset_offset: 1, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:965
        constraint_offset: 22, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:966
    }
    ,
    // Uload16x4: fixed_results=1, use_typevar_operand=true, requires_typevar_operand=true, fixed_values=1
    // Constraints=['Concrete(ir::types::I32X4)', 'Same']
    // Polymorphic over TypeSet(lanes={1}, ints={32, 64})
    OpcodeConstraints {
        flags: 0x39, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:964
        typeset_offset: 1, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:965
        constraint_offset: 24, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:966
    }
    ,
    // Sload16x4: fixed_results=1, use_typevar_operand=true, requires_typevar_operand=true, fixed_values=1
    // Constraints=['Concrete(ir::types::I32X4)', 'Same']
    // Polymorphic over TypeSet(lanes={1}, ints={32, 64})
    OpcodeConstraints {
        flags: 0x39, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:964
        typeset_offset: 1, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:965
        constraint_offset: 24, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:966
    }
    ,
    // Uload32x2: fixed_results=1, use_typevar_operand=true, requires_typevar_operand=true, fixed_values=1
    // Constraints=['Concrete(ir::types::I64X2)', 'Same']
    // Polymorphic over TypeSet(lanes={1}, ints={32, 64})
    OpcodeConstraints {
        flags: 0x39, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:964
        typeset_offset: 1, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:965
        constraint_offset: 26, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:966
    }
    ,
    // Sload32x2: fixed_results=1, use_typevar_operand=true, requires_typevar_operand=true, fixed_values=1
    // Constraints=['Concrete(ir::types::I64X2)', 'Same']
    // Polymorphic over TypeSet(lanes={1}, ints={32, 64})
    OpcodeConstraints {
        flags: 0x39, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:964
        typeset_offset: 1, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:965
        constraint_offset: 26, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:966
    }
    ,
    // StackLoad: fixed_results=1, use_typevar_operand=false, requires_typevar_operand=false, fixed_values=0
    // Constraints=['Same']
    // Polymorphic over TypeSet(lanes={1, 2, 4, 8, 16, 32, 64, 128, 256}, ints={8, 16, 32, 64, 128}, floats={16, 32, 64, 128})
    OpcodeConstraints {
        flags: 0x01, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:964
        typeset_offset: 5, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:965
        constraint_offset: 0, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:966
    }
    ,
    // StackStore: fixed_results=0, use_typevar_operand=true, requires_typevar_operand=true, fixed_values=1
    // Constraints=['Same']
    // Polymorphic over TypeSet(lanes={1, 2, 4, 8, 16, 32, 64, 128, 256}, ints={8, 16, 32, 64, 128}, floats={16, 32, 64, 128})
    OpcodeConstraints {
        flags: 0x38, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:964
        typeset_offset: 5, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:965
        constraint_offset: 0, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:966
    }
    ,
    // StackAddr: fixed_results=1, use_typevar_operand=false, requires_typevar_operand=false, fixed_values=0
    // Constraints=['Same']
    // Polymorphic over TypeSet(lanes={1}, ints={32, 64})
    OpcodeConstraints {
        flags: 0x01, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:964
        typeset_offset: 1, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:965
        constraint_offset: 0, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:966
    }
    ,
    // DynamicStackLoad: fixed_results=1, use_typevar_operand=false, requires_typevar_operand=false, fixed_values=0
    // Constraints=['Same']
    // Polymorphic over TypeSet(lanes={1, 2, 4, 8, 16, 32, 64, 128, 256}, ints={8, 16, 32, 64, 128}, floats={16, 32, 64, 128})
    OpcodeConstraints {
        flags: 0x01, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:964
        typeset_offset: 5, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:965
        constraint_offset: 0, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:966
    }
    ,
    // DynamicStackStore: fixed_results=0, use_typevar_operand=true, requires_typevar_operand=true, fixed_values=1
    // Constraints=['Same']
    // Polymorphic over TypeSet(lanes={1, 2, 4, 8, 16, 32, 64, 128, 256}, ints={8, 16, 32, 64, 128}, floats={16, 32, 64, 128})
    OpcodeConstraints {
        flags: 0x38, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:964
        typeset_offset: 5, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:965
        constraint_offset: 0, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:966
    }
    ,
    // DynamicStackAddr: fixed_results=1, use_typevar_operand=false, requires_typevar_operand=false, fixed_values=0
    // Constraints=['Same']
    // Polymorphic over TypeSet(lanes={1}, ints={32, 64})
    OpcodeConstraints {
        flags: 0x01, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:964
        typeset_offset: 1, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:965
        constraint_offset: 0, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:966
    }
    ,
    // GlobalValue: fixed_results=1, use_typevar_operand=false, requires_typevar_operand=false, fixed_values=0
    // Constraints=['Same']
    // Polymorphic over TypeSet(lanes={1, 2, 4, 8, 16, 32, 64, 128, 256}, ints={8, 16, 32, 64, 128}, floats={16, 32, 64, 128})
    OpcodeConstraints {
        flags: 0x01, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:964
        typeset_offset: 5, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:965
        constraint_offset: 0, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:966
    }
    ,
    // SymbolValue: fixed_results=1, use_typevar_operand=false, requires_typevar_operand=false, fixed_values=0
    // Constraints=['Same']
    // Polymorphic over TypeSet(lanes={1, 2, 4, 8, 16, 32, 64, 128, 256}, ints={8, 16, 32, 64, 128}, floats={16, 32, 64, 128})
    OpcodeConstraints {
        flags: 0x01, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:964
        typeset_offset: 5, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:965
        constraint_offset: 0, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:966
    }
    ,
    // TlsValue: fixed_results=1, use_typevar_operand=false, requires_typevar_operand=false, fixed_values=0
    // Constraints=['Same']
    // Polymorphic over TypeSet(lanes={1, 2, 4, 8, 16, 32, 64, 128, 256}, ints={8, 16, 32, 64, 128}, floats={16, 32, 64, 128})
    OpcodeConstraints {
        flags: 0x01, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:964
        typeset_offset: 5, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:965
        constraint_offset: 0, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:966
    }
    ,
    // GetPinnedReg: fixed_results=1, use_typevar_operand=false, requires_typevar_operand=false, fixed_values=0
    // Constraints=['Same']
    // Polymorphic over TypeSet(lanes={1}, ints={32, 64})
    OpcodeConstraints {
        flags: 0x01, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:964
        typeset_offset: 1, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:965
        constraint_offset: 0, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:966
    }
    ,
    // SetPinnedReg: fixed_results=0, use_typevar_operand=true, requires_typevar_operand=true, fixed_values=1
    // Constraints=['Same']
    // Polymorphic over TypeSet(lanes={1}, ints={32, 64})
    OpcodeConstraints {
        flags: 0x38, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:964
        typeset_offset: 1, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:965
        constraint_offset: 0, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:966
    }
    ,
    // GetFramePointer: fixed_results=1, use_typevar_operand=false, requires_typevar_operand=false, fixed_values=0
    // Constraints=['Same']
    // Polymorphic over TypeSet(lanes={1}, ints={32, 64})
    OpcodeConstraints {
        flags: 0x01, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:964
        typeset_offset: 1, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:965
        constraint_offset: 0, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:966
    }
    ,
    // GetStackPointer: fixed_results=1, use_typevar_operand=false, requires_typevar_operand=false, fixed_values=0
    // Constraints=['Same']
    // Polymorphic over TypeSet(lanes={1}, ints={32, 64})
    OpcodeConstraints {
        flags: 0x01, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:964
        typeset_offset: 1, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:965
        constraint_offset: 0, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:966
    }
    ,
    // GetReturnAddress: fixed_results=1, use_typevar_operand=false, requires_typevar_operand=false, fixed_values=0
    // Constraints=['Same']
    // Polymorphic over TypeSet(lanes={1}, ints={32, 64})
    OpcodeConstraints {
        flags: 0x01, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:964
        typeset_offset: 1, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:965
        constraint_offset: 0, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:966
    }
    ,
    // GetExceptionHandlerAddress: fixed_results=1, use_typevar_operand=false, requires_typevar_operand=false, fixed_values=0
    // Constraints=['Same']
    // Polymorphic over TypeSet(lanes={1}, ints={32, 64})
    OpcodeConstraints {
        flags: 0x01, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:964
        typeset_offset: 1, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:965
        constraint_offset: 0, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:966
    }
    ,
    // Iconst: fixed_results=1, use_typevar_operand=false, requires_typevar_operand=false, fixed_values=0
    // Constraints=['Same']
    // Polymorphic over TypeSet(lanes={1}, ints={8, 16, 32, 64})
    OpcodeConstraints {
        flags: 0x01, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:964
        typeset_offset: 8, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:965
        constraint_offset: 0, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:966
    }
    ,
    // F16const: fixed_results=1, use_typevar_operand=false, requires_typevar_operand=false, fixed_values=0
    // Constraints=['Concrete(ir::types::F16)']
    OpcodeConstraints {
        flags: 0x01, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:964
        typeset_offset: 255, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:965
        constraint_offset: 28, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:966
    }
    ,
    // F32const: fixed_results=1, use_typevar_operand=false, requires_typevar_operand=false, fixed_values=0
    // Constraints=['Concrete(ir::types::F32)']
    OpcodeConstraints {
        flags: 0x01, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:964
        typeset_offset: 255, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:965
        constraint_offset: 29, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:966
    }
    ,
    // F64const: fixed_results=1, use_typevar_operand=false, requires_typevar_operand=false, fixed_values=0
    // Constraints=['Concrete(ir::types::F64)']
    OpcodeConstraints {
        flags: 0x01, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:964
        typeset_offset: 255, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:965
        constraint_offset: 30, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:966
    }
    ,
    // F128const: fixed_results=1, use_typevar_operand=false, requires_typevar_operand=false, fixed_values=0
    // Constraints=['Concrete(ir::types::F128)']
    OpcodeConstraints {
        flags: 0x01, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:964
        typeset_offset: 255, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:965
        constraint_offset: 31, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:966
    }
    ,
    // Vconst: fixed_results=1, use_typevar_operand=false, requires_typevar_operand=false, fixed_values=0
    // Constraints=['Same']
    // Polymorphic over TypeSet(lanes={2, 4, 8, 16, 32, 64, 128, 256}, ints={8, 16, 32, 64, 128}, floats={16, 32, 64, 128})
    OpcodeConstraints {
        flags: 0x01, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:964
        typeset_offset: 9, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:965
        constraint_offset: 0, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:966
    }
    ,
    // Shuffle: fixed_results=1, use_typevar_operand=false, requires_typevar_operand=false, fixed_values=2
    // Constraints=['Concrete(ir::types::I8X16)', 'Concrete(ir::types::I8X16)', 'Concrete(ir::types::I8X16)']
    OpcodeConstraints {
        flags: 0x41, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:964
        typeset_offset: 255, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:965
        constraint_offset: 6, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:966
    }
    ,
    // Nop: fixed_results=0, use_typevar_operand=false, requires_typevar_operand=false, fixed_values=0
    // Constraints=[]
    OpcodeConstraints {
        flags: 0x00, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:964
        typeset_offset: 255, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:965
        constraint_offset: 0, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:966
    }
    ,
    // Select: fixed_results=1, use_typevar_operand=true, requires_typevar_operand=false, fixed_values=3
    // Constraints=['Same', 'Free(0)', 'Same', 'Same']
    // Polymorphic over TypeSet(lanes={1, 2, 4, 8, 16, 32, 64, 128, 256}, ints={8, 16, 32, 64, 128}, floats={16, 32, 64, 128})
    OpcodeConstraints {
        flags: 0x69, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:964
        typeset_offset: 10, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:965
        constraint_offset: 32, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:966
    }
    ,
    // SelectSpectreGuard: fixed_results=1, use_typevar_operand=true, requires_typevar_operand=false, fixed_values=3
    // Constraints=['Same', 'Free(0)', 'Same', 'Same']
    // Polymorphic over TypeSet(lanes={1, 2, 4, 8, 16, 32, 64, 128, 256}, ints={8, 16, 32, 64, 128}, floats={16, 32, 64, 128})
    OpcodeConstraints {
        flags: 0x69, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:964
        typeset_offset: 10, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:965
        constraint_offset: 32, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:966
    }
    ,
    // Bitselect: fixed_results=1, use_typevar_operand=true, requires_typevar_operand=false, fixed_values=3
    // Constraints=['Same', 'Same', 'Same', 'Same']
    // Polymorphic over TypeSet(lanes={1, 2, 4, 8, 16, 32, 64, 128, 256}, ints={8, 16, 32, 64, 128}, floats={16, 32, 64, 128})
    OpcodeConstraints {
        flags: 0x69, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:964
        typeset_offset: 10, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:965
        constraint_offset: 18, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:966
    }
    ,
    // Blendv: fixed_results=1, use_typevar_operand=true, requires_typevar_operand=false, fixed_values=3
    // Constraints=['Same', 'Same', 'Same', 'Same']
    // Polymorphic over TypeSet(lanes={1, 2, 4, 8, 16, 32, 64, 128, 256}, ints={8, 16, 32, 64, 128}, floats={16, 32, 64, 128})
    OpcodeConstraints {
        flags: 0x69, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:964
        typeset_offset: 10, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:965
        constraint_offset: 18, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:966
    }
    ,
    // VanyTrue: fixed_results=1, use_typevar_operand=true, requires_typevar_operand=true, fixed_values=1
    // Constraints=['Concrete(ir::types::I8)', 'Same']
    // Polymorphic over TypeSet(lanes={2, 4, 8, 16, 32, 64, 128, 256}, ints={8, 16, 32, 64, 128}, floats={16, 32, 64, 128})
    OpcodeConstraints {
        flags: 0x39, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:964
        typeset_offset: 9, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:965
        constraint_offset: 36, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:966
    }
    ,
    // VallTrue: fixed_results=1, use_typevar_operand=true, requires_typevar_operand=true, fixed_values=1
    // Constraints=['Concrete(ir::types::I8)', 'Same']
    // Polymorphic over TypeSet(lanes={2, 4, 8, 16, 32, 64, 128, 256}, ints={8, 16, 32, 64, 128}, floats={16, 32, 64, 128})
    OpcodeConstraints {
        flags: 0x39, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:964
        typeset_offset: 9, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:965
        constraint_offset: 36, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:966
    }
    ,
    // VhighBits: fixed_results=1, use_typevar_operand=false, requires_typevar_operand=false, fixed_values=1
    // Constraints=['Same', 'Free(9)']
    // Polymorphic over TypeSet(lanes={1}, ints={8, 16, 32, 64})
    OpcodeConstraints {
        flags: 0x21, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:964
        typeset_offset: 8, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:965
        constraint_offset: 37, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:966
    }
    ,
    // Icmp: fixed_results=1, use_typevar_operand=true, requires_typevar_operand=true, fixed_values=2
    // Constraints=['AsTruthy', 'Same', 'Same']
    // Polymorphic over TypeSet(lanes={1, 2, 4, 8, 16, 32, 64, 128, 256}, ints={8, 16, 32, 64, 128})
    OpcodeConstraints {
        flags: 0x59, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:964
        typeset_offset: 11, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:965
        constraint_offset: 39, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:966
    }
    ,
    // IcmpImm: fixed_results=1, use_typevar_operand=true, requires_typevar_operand=true, fixed_values=1
    // Constraints=['Concrete(ir::types::I8)', 'Same']
    // Polymorphic over TypeSet(lanes={1}, ints={8, 16, 32, 64, 128})
    OpcodeConstraints {
        flags: 0x39, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:964
        typeset_offset: 0, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:965
        constraint_offset: 36, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:966
    }
    ,
    // Iadd: fixed_results=1, use_typevar_operand=true, requires_typevar_operand=false, fixed_values=2
    // Constraints=['Same', 'Same', 'Same']
    // Polymorphic over TypeSet(lanes={1, 2, 4, 8, 16, 32, 64, 128, 256}, ints={8, 16, 32, 64, 128})
    OpcodeConstraints {
        flags: 0x49, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:964
        typeset_offset: 11, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:965
        constraint_offset: 0, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:966
    }
    ,
    // Isub: fixed_results=1, use_typevar_operand=true, requires_typevar_operand=false, fixed_values=2
    // Constraints=['Same', 'Same', 'Same']
    // Polymorphic over TypeSet(lanes={1, 2, 4, 8, 16, 32, 64, 128, 256}, ints={8, 16, 32, 64, 128})
    OpcodeConstraints {
        flags: 0x49, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:964
        typeset_offset: 11, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:965
        constraint_offset: 0, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:966
    }
    ,
    // Ineg: fixed_results=1, use_typevar_operand=true, requires_typevar_operand=false, fixed_values=1
    // Constraints=['Same', 'Same']
    // Polymorphic over TypeSet(lanes={1, 2, 4, 8, 16, 32, 64, 128, 256}, ints={8, 16, 32, 64, 128})
    OpcodeConstraints {
        flags: 0x29, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:964
        typeset_offset: 11, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:965
        constraint_offset: 0, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:966
    }
    ,
    // Iabs: fixed_results=1, use_typevar_operand=true, requires_typevar_operand=false, fixed_values=1
    // Constraints=['Same', 'Same']
    // Polymorphic over TypeSet(lanes={1, 2, 4, 8, 16, 32, 64, 128, 256}, ints={8, 16, 32, 64, 128})
    OpcodeConstraints {
        flags: 0x29, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:964
        typeset_offset: 11, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:965
        constraint_offset: 0, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:966
    }
    ,
    // Imul: fixed_results=1, use_typevar_operand=true, requires_typevar_operand=false, fixed_values=2
    // Constraints=['Same', 'Same', 'Same']
    // Polymorphic over TypeSet(lanes={1, 2, 4, 8, 16, 32, 64, 128, 256}, ints={8, 16, 32, 64, 128})
    OpcodeConstraints {
        flags: 0x49, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:964
        typeset_offset: 11, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:965
        constraint_offset: 0, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:966
    }
    ,
    // Umulhi: fixed_results=1, use_typevar_operand=true, requires_typevar_operand=false, fixed_values=2
    // Constraints=['Same', 'Same', 'Same']
    // Polymorphic over TypeSet(lanes={1, 2, 4, 8, 16, 32, 64, 128, 256}, ints={8, 16, 32, 64, 128})
    OpcodeConstraints {
        flags: 0x49, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:964
        typeset_offset: 11, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:965
        constraint_offset: 0, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:966
    }
    ,
    // Smulhi: fixed_results=1, use_typevar_operand=true, requires_typevar_operand=false, fixed_values=2
    // Constraints=['Same', 'Same', 'Same']
    // Polymorphic over TypeSet(lanes={1, 2, 4, 8, 16, 32, 64, 128, 256}, ints={8, 16, 32, 64, 128})
    OpcodeConstraints {
        flags: 0x49, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:964
        typeset_offset: 11, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:965
        constraint_offset: 0, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:966
    }
    ,
    // SqmulRoundSat: fixed_results=1, use_typevar_operand=true, requires_typevar_operand=false, fixed_values=2
    // Constraints=['Same', 'Same', 'Same']
    // Polymorphic over TypeSet(lanes={4, 8}, ints={16, 32})
    OpcodeConstraints {
        flags: 0x49, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:964
        typeset_offset: 12, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:965
        constraint_offset: 0, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:966
    }
    ,
    // X86Pmulhrsw: fixed_results=1, use_typevar_operand=true, requires_typevar_operand=false, fixed_values=2
    // Constraints=['Same', 'Same', 'Same']
    // Polymorphic over TypeSet(lanes={4, 8}, ints={16, 32})
    OpcodeConstraints {
        flags: 0x49, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:964
        typeset_offset: 12, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:965
        constraint_offset: 0, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:966
    }
    ,
    // Udiv: fixed_results=1, use_typevar_operand=true, requires_typevar_operand=false, fixed_values=2
    // Constraints=['Same', 'Same', 'Same']
    // Polymorphic over TypeSet(lanes={1}, ints={8, 16, 32, 64, 128})
    OpcodeConstraints {
        flags: 0x49, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:964
        typeset_offset: 0, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:965
        constraint_offset: 0, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:966
    }
    ,
    // Sdiv: fixed_results=1, use_typevar_operand=true, requires_typevar_operand=false, fixed_values=2
    // Constraints=['Same', 'Same', 'Same']
    // Polymorphic over TypeSet(lanes={1}, ints={8, 16, 32, 64, 128})
    OpcodeConstraints {
        flags: 0x49, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:964
        typeset_offset: 0, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:965
        constraint_offset: 0, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:966
    }
    ,
    // Urem: fixed_results=1, use_typevar_operand=true, requires_typevar_operand=false, fixed_values=2
    // Constraints=['Same', 'Same', 'Same']
    // Polymorphic over TypeSet(lanes={1}, ints={8, 16, 32, 64, 128})
    OpcodeConstraints {
        flags: 0x49, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:964
        typeset_offset: 0, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:965
        constraint_offset: 0, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:966
    }
    ,
    // Srem: fixed_results=1, use_typevar_operand=true, requires_typevar_operand=false, fixed_values=2
    // Constraints=['Same', 'Same', 'Same']
    // Polymorphic over TypeSet(lanes={1}, ints={8, 16, 32, 64, 128})
    OpcodeConstraints {
        flags: 0x49, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:964
        typeset_offset: 0, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:965
        constraint_offset: 0, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:966
    }
    ,
    // IaddImm: fixed_results=1, use_typevar_operand=true, requires_typevar_operand=false, fixed_values=1
    // Constraints=['Same', 'Same']
    // Polymorphic over TypeSet(lanes={1}, ints={8, 16, 32, 64, 128})
    OpcodeConstraints {
        flags: 0x29, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:964
        typeset_offset: 0, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:965
        constraint_offset: 0, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:966
    }
    ,
    // ImulImm: fixed_results=1, use_typevar_operand=true, requires_typevar_operand=false, fixed_values=1
    // Constraints=['Same', 'Same']
    // Polymorphic over TypeSet(lanes={1}, ints={8, 16, 32, 64, 128})
    OpcodeConstraints {
        flags: 0x29, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:964
        typeset_offset: 0, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:965
        constraint_offset: 0, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:966
    }
    ,
    // UdivImm: fixed_results=1, use_typevar_operand=true, requires_typevar_operand=false, fixed_values=1
    // Constraints=['Same', 'Same']
    // Polymorphic over TypeSet(lanes={1}, ints={8, 16, 32, 64, 128})
    OpcodeConstraints {
        flags: 0x29, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:964
        typeset_offset: 0, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:965
        constraint_offset: 0, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:966
    }
    ,
    // SdivImm: fixed_results=1, use_typevar_operand=true, requires_typevar_operand=false, fixed_values=1
    // Constraints=['Same', 'Same']
    // Polymorphic over TypeSet(lanes={1}, ints={8, 16, 32, 64, 128})
    OpcodeConstraints {
        flags: 0x29, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:964
        typeset_offset: 0, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:965
        constraint_offset: 0, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:966
    }
    ,
    // UremImm: fixed_results=1, use_typevar_operand=true, requires_typevar_operand=false, fixed_values=1
    // Constraints=['Same', 'Same']
    // Polymorphic over TypeSet(lanes={1}, ints={8, 16, 32, 64, 128})
    OpcodeConstraints {
        flags: 0x29, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:964
        typeset_offset: 0, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:965
        constraint_offset: 0, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:966
    }
    ,
    // SremImm: fixed_results=1, use_typevar_operand=true, requires_typevar_operand=false, fixed_values=1
    // Constraints=['Same', 'Same']
    // Polymorphic over TypeSet(lanes={1}, ints={8, 16, 32, 64, 128})
    OpcodeConstraints {
        flags: 0x29, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:964
        typeset_offset: 0, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:965
        constraint_offset: 0, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:966
    }
    ,
    // IrsubImm: fixed_results=1, use_typevar_operand=true, requires_typevar_operand=false, fixed_values=1
    // Constraints=['Same', 'Same']
    // Polymorphic over TypeSet(lanes={1}, ints={8, 16, 32, 64, 128})
    OpcodeConstraints {
        flags: 0x29, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:964
        typeset_offset: 0, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:965
        constraint_offset: 0, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:966
    }
    ,
    // SaddOverflowCin: fixed_results=2, use_typevar_operand=true, requires_typevar_operand=false, fixed_values=3
    // Constraints=['Same', 'Concrete(ir::types::I8)', 'Same', 'Same', 'Concrete(ir::types::I8)']
    // Polymorphic over TypeSet(lanes={1}, ints={8, 16, 32, 64, 128})
    OpcodeConstraints {
        flags: 0x6a, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:964
        typeset_offset: 0, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:965
        constraint_offset: 41, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:966
    }
    ,
    // UaddOverflowCin: fixed_results=2, use_typevar_operand=true, requires_typevar_operand=false, fixed_values=3
    // Constraints=['Same', 'Concrete(ir::types::I8)', 'Same', 'Same', 'Concrete(ir::types::I8)']
    // Polymorphic over TypeSet(lanes={1}, ints={8, 16, 32, 64, 128})
    OpcodeConstraints {
        flags: 0x6a, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:964
        typeset_offset: 0, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:965
        constraint_offset: 41, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:966
    }
    ,
    // UaddOverflow: fixed_results=2, use_typevar_operand=true, requires_typevar_operand=false, fixed_values=2
    // Constraints=['Same', 'Concrete(ir::types::I8)', 'Same', 'Same']
    // Polymorphic over TypeSet(lanes={1}, ints={8, 16, 32, 64, 128})
    OpcodeConstraints {
        flags: 0x4a, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:964
        typeset_offset: 0, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:965
        constraint_offset: 41, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:966
    }
    ,
    // SaddOverflow: fixed_results=2, use_typevar_operand=true, requires_typevar_operand=false, fixed_values=2
    // Constraints=['Same', 'Concrete(ir::types::I8)', 'Same', 'Same']
    // Polymorphic over TypeSet(lanes={1}, ints={8, 16, 32, 64, 128})
    OpcodeConstraints {
        flags: 0x4a, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:964
        typeset_offset: 0, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:965
        constraint_offset: 41, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:966
    }
    ,
    // UsubOverflow: fixed_results=2, use_typevar_operand=true, requires_typevar_operand=false, fixed_values=2
    // Constraints=['Same', 'Concrete(ir::types::I8)', 'Same', 'Same']
    // Polymorphic over TypeSet(lanes={1}, ints={8, 16, 32, 64, 128})
    OpcodeConstraints {
        flags: 0x4a, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:964
        typeset_offset: 0, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:965
        constraint_offset: 41, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:966
    }
    ,
    // SsubOverflow: fixed_results=2, use_typevar_operand=true, requires_typevar_operand=false, fixed_values=2
    // Constraints=['Same', 'Concrete(ir::types::I8)', 'Same', 'Same']
    // Polymorphic over TypeSet(lanes={1}, ints={8, 16, 32, 64, 128})
    OpcodeConstraints {
        flags: 0x4a, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:964
        typeset_offset: 0, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:965
        constraint_offset: 41, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:966
    }
    ,
    // UmulOverflow: fixed_results=2, use_typevar_operand=true, requires_typevar_operand=false, fixed_values=2
    // Constraints=['Same', 'Concrete(ir::types::I8)', 'Same', 'Same']
    // Polymorphic over TypeSet(lanes={1}, ints={8, 16, 32, 64})
    OpcodeConstraints {
        flags: 0x4a, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:964
        typeset_offset: 8, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:965
        constraint_offset: 41, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:966
    }
    ,
    // SmulOverflow: fixed_results=2, use_typevar_operand=true, requires_typevar_operand=false, fixed_values=2
    // Constraints=['Same', 'Concrete(ir::types::I8)', 'Same', 'Same']
    // Polymorphic over TypeSet(lanes={1}, ints={8, 16, 32, 64})
    OpcodeConstraints {
        flags: 0x4a, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:964
        typeset_offset: 8, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:965
        constraint_offset: 41, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:966
    }
    ,
    // UaddOverflowTrap: fixed_results=1, use_typevar_operand=true, requires_typevar_operand=false, fixed_values=2
    // Constraints=['Same', 'Same', 'Same']
    // Polymorphic over TypeSet(lanes={1}, ints={32, 64})
    OpcodeConstraints {
        flags: 0x49, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:964
        typeset_offset: 1, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:965
        constraint_offset: 0, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:966
    }
    ,
    // SsubOverflowBin: fixed_results=2, use_typevar_operand=true, requires_typevar_operand=false, fixed_values=3
    // Constraints=['Same', 'Concrete(ir::types::I8)', 'Same', 'Same', 'Concrete(ir::types::I8)']
    // Polymorphic over TypeSet(lanes={1}, ints={8, 16, 32, 64, 128})
    OpcodeConstraints {
        flags: 0x6a, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:964
        typeset_offset: 0, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:965
        constraint_offset: 41, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:966
    }
    ,
    // UsubOverflowBin: fixed_results=2, use_typevar_operand=true, requires_typevar_operand=false, fixed_values=3
    // Constraints=['Same', 'Concrete(ir::types::I8)', 'Same', 'Same', 'Concrete(ir::types::I8)']
    // Polymorphic over TypeSet(lanes={1}, ints={8, 16, 32, 64, 128})
    OpcodeConstraints {
        flags: 0x6a, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:964
        typeset_offset: 0, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:965
        constraint_offset: 41, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:966
    }
    ,
    // Band: fixed_results=1, use_typevar_operand=true, requires_typevar_operand=false, fixed_values=2
    // Constraints=['Same', 'Same', 'Same']
    // Polymorphic over TypeSet(lanes={1, 2, 4, 8, 16, 32, 64, 128, 256}, ints={8, 16, 32, 64, 128}, floats={16, 32, 64, 128})
    OpcodeConstraints {
        flags: 0x49, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:964
        typeset_offset: 10, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:965
        constraint_offset: 0, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:966
    }
    ,
    // Bor: fixed_results=1, use_typevar_operand=true, requires_typevar_operand=false, fixed_values=2
    // Constraints=['Same', 'Same', 'Same']
    // Polymorphic over TypeSet(lanes={1, 2, 4, 8, 16, 32, 64, 128, 256}, ints={8, 16, 32, 64, 128}, floats={16, 32, 64, 128})
    OpcodeConstraints {
        flags: 0x49, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:964
        typeset_offset: 10, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:965
        constraint_offset: 0, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:966
    }
    ,
    // Bxor: fixed_results=1, use_typevar_operand=true, requires_typevar_operand=false, fixed_values=2
    // Constraints=['Same', 'Same', 'Same']
    // Polymorphic over TypeSet(lanes={1, 2, 4, 8, 16, 32, 64, 128, 256}, ints={8, 16, 32, 64, 128}, floats={16, 32, 64, 128})
    OpcodeConstraints {
        flags: 0x49, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:964
        typeset_offset: 10, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:965
        constraint_offset: 0, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:966
    }
    ,
    // Bnot: fixed_results=1, use_typevar_operand=true, requires_typevar_operand=false, fixed_values=1
    // Constraints=['Same', 'Same']
    // Polymorphic over TypeSet(lanes={1, 2, 4, 8, 16, 32, 64, 128, 256}, ints={8, 16, 32, 64, 128}, floats={16, 32, 64, 128})
    OpcodeConstraints {
        flags: 0x29, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:964
        typeset_offset: 10, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:965
        constraint_offset: 0, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:966
    }
    ,
    // BandNot: fixed_results=1, use_typevar_operand=true, requires_typevar_operand=false, fixed_values=2
    // Constraints=['Same', 'Same', 'Same']
    // Polymorphic over TypeSet(lanes={1, 2, 4, 8, 16, 32, 64, 128, 256}, ints={8, 16, 32, 64, 128}, floats={16, 32, 64, 128})
    OpcodeConstraints {
        flags: 0x49, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:964
        typeset_offset: 10, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:965
        constraint_offset: 0, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:966
    }
    ,
    // BorNot: fixed_results=1, use_typevar_operand=true, requires_typevar_operand=false, fixed_values=2
    // Constraints=['Same', 'Same', 'Same']
    // Polymorphic over TypeSet(lanes={1, 2, 4, 8, 16, 32, 64, 128, 256}, ints={8, 16, 32, 64, 128}, floats={16, 32, 64, 128})
    OpcodeConstraints {
        flags: 0x49, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:964
        typeset_offset: 10, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:965
        constraint_offset: 0, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:966
    }
    ,
    // BxorNot: fixed_results=1, use_typevar_operand=true, requires_typevar_operand=false, fixed_values=2
    // Constraints=['Same', 'Same', 'Same']
    // Polymorphic over TypeSet(lanes={1, 2, 4, 8, 16, 32, 64, 128, 256}, ints={8, 16, 32, 64, 128}, floats={16, 32, 64, 128})
    OpcodeConstraints {
        flags: 0x49, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:964
        typeset_offset: 10, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:965
        constraint_offset: 0, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:966
    }
    ,
    // BandImm: fixed_results=1, use_typevar_operand=true, requires_typevar_operand=false, fixed_values=1
    // Constraints=['Same', 'Same']
    // Polymorphic over TypeSet(lanes={1}, ints={8, 16, 32, 64, 128})
    OpcodeConstraints {
        flags: 0x29, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:964
        typeset_offset: 0, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:965
        constraint_offset: 0, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:966
    }
    ,
    // BorImm: fixed_results=1, use_typevar_operand=true, requires_typevar_operand=false, fixed_values=1
    // Constraints=['Same', 'Same']
    // Polymorphic over TypeSet(lanes={1}, ints={8, 16, 32, 64, 128})
    OpcodeConstraints {
        flags: 0x29, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:964
        typeset_offset: 0, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:965
        constraint_offset: 0, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:966
    }
    ,
    // BxorImm: fixed_results=1, use_typevar_operand=true, requires_typevar_operand=false, fixed_values=1
    // Constraints=['Same', 'Same']
    // Polymorphic over TypeSet(lanes={1}, ints={8, 16, 32, 64, 128})
    OpcodeConstraints {
        flags: 0x29, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:964
        typeset_offset: 0, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:965
        constraint_offset: 0, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:966
    }
    ,
    // Rotl: fixed_results=1, use_typevar_operand=true, requires_typevar_operand=false, fixed_values=2
    // Constraints=['Same', 'Same', 'Free(0)']
    // Polymorphic over TypeSet(lanes={1, 2, 4, 8, 16, 32, 64, 128, 256}, ints={8, 16, 32, 64, 128})
    OpcodeConstraints {
        flags: 0x49, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:964
        typeset_offset: 11, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:965
        constraint_offset: 46, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:966
    }
    ,
    // Rotr: fixed_results=1, use_typevar_operand=true, requires_typevar_operand=false, fixed_values=2
    // Constraints=['Same', 'Same', 'Free(0)']
    // Polymorphic over TypeSet(lanes={1, 2, 4, 8, 16, 32, 64, 128, 256}, ints={8, 16, 32, 64, 128})
    OpcodeConstraints {
        flags: 0x49, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:964
        typeset_offset: 11, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:965
        constraint_offset: 46, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:966
    }
    ,
    // RotlImm: fixed_results=1, use_typevar_operand=true, requires_typevar_operand=false, fixed_values=1
    // Constraints=['Same', 'Same']
    // Polymorphic over TypeSet(lanes={1, 2, 4, 8, 16, 32, 64, 128, 256}, ints={8, 16, 32, 64, 128})
    OpcodeConstraints {
        flags: 0x29, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:964
        typeset_offset: 11, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:965
        constraint_offset: 0, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:966
    }
    ,
    // RotrImm: fixed_results=1, use_typevar_operand=true, requires_typevar_operand=false, fixed_values=1
    // Constraints=['Same', 'Same']
    // Polymorphic over TypeSet(lanes={1, 2, 4, 8, 16, 32, 64, 128, 256}, ints={8, 16, 32, 64, 128})
    OpcodeConstraints {
        flags: 0x29, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:964
        typeset_offset: 11, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:965
        constraint_offset: 0, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:966
    }
    ,
    // Ishl: fixed_results=1, use_typevar_operand=true, requires_typevar_operand=false, fixed_values=2
    // Constraints=['Same', 'Same', 'Free(0)']
    // Polymorphic over TypeSet(lanes={1, 2, 4, 8, 16, 32, 64, 128, 256}, ints={8, 16, 32, 64, 128})
    OpcodeConstraints {
        flags: 0x49, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:964
        typeset_offset: 11, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:965
        constraint_offset: 46, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:966
    }
    ,
    // Ushr: fixed_results=1, use_typevar_operand=true, requires_typevar_operand=false, fixed_values=2
    // Constraints=['Same', 'Same', 'Free(0)']
    // Polymorphic over TypeSet(lanes={1, 2, 4, 8, 16, 32, 64, 128, 256}, ints={8, 16, 32, 64, 128})
    OpcodeConstraints {
        flags: 0x49, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:964
        typeset_offset: 11, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:965
        constraint_offset: 46, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:966
    }
    ,
    // Sshr: fixed_results=1, use_typevar_operand=true, requires_typevar_operand=false, fixed_values=2
    // Constraints=['Same', 'Same', 'Free(0)']
    // Polymorphic over TypeSet(lanes={1, 2, 4, 8, 16, 32, 64, 128, 256}, ints={8, 16, 32, 64, 128})
    OpcodeConstraints {
        flags: 0x49, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:964
        typeset_offset: 11, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:965
        constraint_offset: 46, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:966
    }
    ,
    // IshlImm: fixed_results=1, use_typevar_operand=true, requires_typevar_operand=false, fixed_values=1
    // Constraints=['Same', 'Same']
    // Polymorphic over TypeSet(lanes={1, 2, 4, 8, 16, 32, 64, 128, 256}, ints={8, 16, 32, 64, 128})
    OpcodeConstraints {
        flags: 0x29, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:964
        typeset_offset: 11, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:965
        constraint_offset: 0, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:966
    }
    ,
    // UshrImm: fixed_results=1, use_typevar_operand=true, requires_typevar_operand=false, fixed_values=1
    // Constraints=['Same', 'Same']
    // Polymorphic over TypeSet(lanes={1, 2, 4, 8, 16, 32, 64, 128, 256}, ints={8, 16, 32, 64, 128})
    OpcodeConstraints {
        flags: 0x29, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:964
        typeset_offset: 11, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:965
        constraint_offset: 0, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:966
    }
    ,
    // SshrImm: fixed_results=1, use_typevar_operand=true, requires_typevar_operand=false, fixed_values=1
    // Constraints=['Same', 'Same']
    // Polymorphic over TypeSet(lanes={1, 2, 4, 8, 16, 32, 64, 128, 256}, ints={8, 16, 32, 64, 128})
    OpcodeConstraints {
        flags: 0x29, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:964
        typeset_offset: 11, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:965
        constraint_offset: 0, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:966
    }
    ,
    // Bitrev: fixed_results=1, use_typevar_operand=true, requires_typevar_operand=false, fixed_values=1
    // Constraints=['Same', 'Same']
    // Polymorphic over TypeSet(lanes={1}, ints={8, 16, 32, 64, 128})
    OpcodeConstraints {
        flags: 0x29, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:964
        typeset_offset: 0, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:965
        constraint_offset: 0, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:966
    }
    ,
    // Clz: fixed_results=1, use_typevar_operand=true, requires_typevar_operand=false, fixed_values=1
    // Constraints=['Same', 'Same']
    // Polymorphic over TypeSet(lanes={1}, ints={8, 16, 32, 64, 128})
    OpcodeConstraints {
        flags: 0x29, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:964
        typeset_offset: 0, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:965
        constraint_offset: 0, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:966
    }
    ,
    // Cls: fixed_results=1, use_typevar_operand=true, requires_typevar_operand=false, fixed_values=1
    // Constraints=['Same', 'Same']
    // Polymorphic over TypeSet(lanes={1}, ints={8, 16, 32, 64, 128})
    OpcodeConstraints {
        flags: 0x29, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:964
        typeset_offset: 0, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:965
        constraint_offset: 0, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:966
    }
    ,
    // Ctz: fixed_results=1, use_typevar_operand=true, requires_typevar_operand=false, fixed_values=1
    // Constraints=['Same', 'Same']
    // Polymorphic over TypeSet(lanes={1}, ints={8, 16, 32, 64, 128})
    OpcodeConstraints {
        flags: 0x29, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:964
        typeset_offset: 0, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:965
        constraint_offset: 0, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:966
    }
    ,
    // Bswap: fixed_results=1, use_typevar_operand=true, requires_typevar_operand=false, fixed_values=1
    // Constraints=['Same', 'Same']
    // Polymorphic over TypeSet(lanes={1}, ints={16, 32, 64, 128})
    OpcodeConstraints {
        flags: 0x29, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:964
        typeset_offset: 13, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:965
        constraint_offset: 0, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:966
    }
    ,
    // Popcnt: fixed_results=1, use_typevar_operand=true, requires_typevar_operand=false, fixed_values=1
    // Constraints=['Same', 'Same']
    // Polymorphic over TypeSet(lanes={1, 2, 4, 8, 16, 32, 64, 128, 256}, ints={8, 16, 32, 64, 128})
    OpcodeConstraints {
        flags: 0x29, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:964
        typeset_offset: 11, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:965
        constraint_offset: 0, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:966
    }
    ,
    // Fcmp: fixed_results=1, use_typevar_operand=true, requires_typevar_operand=true, fixed_values=2
    // Constraints=['AsTruthy', 'Same', 'Same']
    // Polymorphic over TypeSet(lanes={1, 2, 4, 8, 16, 32, 64, 128, 256}, floats={16, 32, 64, 128})
    OpcodeConstraints {
        flags: 0x59, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:964
        typeset_offset: 14, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:965
        constraint_offset: 39, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:966
    }
    ,
    // Fadd: fixed_results=1, use_typevar_operand=true, requires_typevar_operand=false, fixed_values=2
    // Constraints=['Same', 'Same', 'Same']
    // Polymorphic over TypeSet(lanes={1, 2, 4, 8, 16, 32, 64, 128, 256}, floats={16, 32, 64, 128})
    OpcodeConstraints {
        flags: 0x49, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:964
        typeset_offset: 14, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:965
        constraint_offset: 0, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:966
    }
    ,
    // Fsub: fixed_results=1, use_typevar_operand=true, requires_typevar_operand=false, fixed_values=2
    // Constraints=['Same', 'Same', 'Same']
    // Polymorphic over TypeSet(lanes={1, 2, 4, 8, 16, 32, 64, 128, 256}, floats={16, 32, 64, 128})
    OpcodeConstraints {
        flags: 0x49, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:964
        typeset_offset: 14, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:965
        constraint_offset: 0, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:966
    }
    ,
    // Fmul: fixed_results=1, use_typevar_operand=true, requires_typevar_operand=false, fixed_values=2
    // Constraints=['Same', 'Same', 'Same']
    // Polymorphic over TypeSet(lanes={1, 2, 4, 8, 16, 32, 64, 128, 256}, floats={16, 32, 64, 128})
    OpcodeConstraints {
        flags: 0x49, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:964
        typeset_offset: 14, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:965
        constraint_offset: 0, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:966
    }
    ,
    // Fdiv: fixed_results=1, use_typevar_operand=true, requires_typevar_operand=false, fixed_values=2
    // Constraints=['Same', 'Same', 'Same']
    // Polymorphic over TypeSet(lanes={1, 2, 4, 8, 16, 32, 64, 128, 256}, floats={16, 32, 64, 128})
    OpcodeConstraints {
        flags: 0x49, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:964
        typeset_offset: 14, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:965
        constraint_offset: 0, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:966
    }
    ,
    // Sqrt: fixed_results=1, use_typevar_operand=true, requires_typevar_operand=false, fixed_values=1
    // Constraints=['Same', 'Same']
    // Polymorphic over TypeSet(lanes={1, 2, 4, 8, 16, 32, 64, 128, 256}, floats={16, 32, 64, 128})
    OpcodeConstraints {
        flags: 0x29, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:964
        typeset_offset: 14, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:965
        constraint_offset: 0, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:966
    }
    ,
    // Fma: fixed_results=1, use_typevar_operand=true, requires_typevar_operand=false, fixed_values=3
    // Constraints=['Same', 'Same', 'Same', 'Same']
    // Polymorphic over TypeSet(lanes={1, 2, 4, 8, 16, 32, 64, 128, 256}, floats={16, 32, 64, 128})
    OpcodeConstraints {
        flags: 0x69, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:964
        typeset_offset: 14, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:965
        constraint_offset: 18, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:966
    }
    ,
    // Fneg: fixed_results=1, use_typevar_operand=true, requires_typevar_operand=false, fixed_values=1
    // Constraints=['Same', 'Same']
    // Polymorphic over TypeSet(lanes={1, 2, 4, 8, 16, 32, 64, 128, 256}, floats={16, 32, 64, 128})
    OpcodeConstraints {
        flags: 0x29, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:964
        typeset_offset: 14, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:965
        constraint_offset: 0, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:966
    }
    ,
    // Fabs: fixed_results=1, use_typevar_operand=true, requires_typevar_operand=false, fixed_values=1
    // Constraints=['Same', 'Same']
    // Polymorphic over TypeSet(lanes={1, 2, 4, 8, 16, 32, 64, 128, 256}, floats={16, 32, 64, 128})
    OpcodeConstraints {
        flags: 0x29, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:964
        typeset_offset: 14, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:965
        constraint_offset: 0, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:966
    }
    ,
    // Fcopysign: fixed_results=1, use_typevar_operand=true, requires_typevar_operand=false, fixed_values=2
    // Constraints=['Same', 'Same', 'Same']
    // Polymorphic over TypeSet(lanes={1, 2, 4, 8, 16, 32, 64, 128, 256}, floats={16, 32, 64, 128})
    OpcodeConstraints {
        flags: 0x49, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:964
        typeset_offset: 14, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:965
        constraint_offset: 0, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:966
    }
    ,
    // Fmin: fixed_results=1, use_typevar_operand=true, requires_typevar_operand=false, fixed_values=2
    // Constraints=['Same', 'Same', 'Same']
    // Polymorphic over TypeSet(lanes={1, 2, 4, 8, 16, 32, 64, 128, 256}, floats={16, 32, 64, 128})
    OpcodeConstraints {
        flags: 0x49, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:964
        typeset_offset: 14, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:965
        constraint_offset: 0, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:966
    }
    ,
    // Fmax: fixed_results=1, use_typevar_operand=true, requires_typevar_operand=false, fixed_values=2
    // Constraints=['Same', 'Same', 'Same']
    // Polymorphic over TypeSet(lanes={1, 2, 4, 8, 16, 32, 64, 128, 256}, floats={16, 32, 64, 128})
    OpcodeConstraints {
        flags: 0x49, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:964
        typeset_offset: 14, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:965
        constraint_offset: 0, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:966
    }
    ,
    // Ceil: fixed_results=1, use_typevar_operand=true, requires_typevar_operand=false, fixed_values=1
    // Constraints=['Same', 'Same']
    // Polymorphic over TypeSet(lanes={1, 2, 4, 8, 16, 32, 64, 128, 256}, floats={16, 32, 64, 128})
    OpcodeConstraints {
        flags: 0x29, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:964
        typeset_offset: 14, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:965
        constraint_offset: 0, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:966
    }
    ,
    // Floor: fixed_results=1, use_typevar_operand=true, requires_typevar_operand=false, fixed_values=1
    // Constraints=['Same', 'Same']
    // Polymorphic over TypeSet(lanes={1, 2, 4, 8, 16, 32, 64, 128, 256}, floats={16, 32, 64, 128})
    OpcodeConstraints {
        flags: 0x29, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:964
        typeset_offset: 14, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:965
        constraint_offset: 0, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:966
    }
    ,
    // Trunc: fixed_results=1, use_typevar_operand=true, requires_typevar_operand=false, fixed_values=1
    // Constraints=['Same', 'Same']
    // Polymorphic over TypeSet(lanes={1, 2, 4, 8, 16, 32, 64, 128, 256}, floats={16, 32, 64, 128})
    OpcodeConstraints {
        flags: 0x29, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:964
        typeset_offset: 14, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:965
        constraint_offset: 0, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:966
    }
    ,
    // Nearest: fixed_results=1, use_typevar_operand=true, requires_typevar_operand=false, fixed_values=1
    // Constraints=['Same', 'Same']
    // Polymorphic over TypeSet(lanes={1, 2, 4, 8, 16, 32, 64, 128, 256}, floats={16, 32, 64, 128})
    OpcodeConstraints {
        flags: 0x29, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:964
        typeset_offset: 14, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:965
        constraint_offset: 0, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:966
    }
    ,
    // Bitcast: fixed_results=1, use_typevar_operand=false, requires_typevar_operand=false, fixed_values=1
    // Constraints=['Same', 'Free(5)']
    // Polymorphic over TypeSet(lanes={1, 2, 4, 8, 16, 32, 64, 128, 256}, ints={8, 16, 32, 64, 128}, floats={16, 32, 64, 128})
    OpcodeConstraints {
        flags: 0x21, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:964
        typeset_offset: 5, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:965
        constraint_offset: 49, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:966
    }
    ,
    // ScalarToVector: fixed_results=1, use_typevar_operand=false, requires_typevar_operand=false, fixed_values=1
    // Constraints=['Same', 'LaneOf']
    // Polymorphic over TypeSet(lanes={2, 4, 8, 16, 32, 64, 128, 256}, ints={8, 16, 32, 64, 128}, floats={16, 32, 64, 128})
    OpcodeConstraints {
        flags: 0x21, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:964
        typeset_offset: 9, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:965
        constraint_offset: 4, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:966
    }
    ,
    // Bmask: fixed_results=1, use_typevar_operand=false, requires_typevar_operand=false, fixed_values=1
    // Constraints=['Same', 'Free(0)']
    // Polymorphic over TypeSet(lanes={1}, ints={8, 16, 32, 64, 128})
    OpcodeConstraints {
        flags: 0x21, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:964
        typeset_offset: 0, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:965
        constraint_offset: 32, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:966
    }
    ,
    // Ireduce: fixed_results=1, use_typevar_operand=false, requires_typevar_operand=false, fixed_values=1
    // Constraints=['Same', 'Wider']
    // Polymorphic over TypeSet(lanes={1}, ints={8, 16, 32, 64, 128})
    OpcodeConstraints {
        flags: 0x21, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:964
        typeset_offset: 0, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:965
        constraint_offset: 51, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:966
    }
    ,
    // Snarrow: fixed_results=1, use_typevar_operand=true, requires_typevar_operand=true, fixed_values=2
    // Constraints=['SplitLanes', 'Same', 'Same']
    // Polymorphic over TypeSet(lanes={2, 4, 8}, ints={16, 32, 64})
    OpcodeConstraints {
        flags: 0x59, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:964
        typeset_offset: 15, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:965
        constraint_offset: 53, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:966
    }
    ,
    // Unarrow: fixed_results=1, use_typevar_operand=true, requires_typevar_operand=true, fixed_values=2
    // Constraints=['SplitLanes', 'Same', 'Same']
    // Polymorphic over TypeSet(lanes={2, 4, 8}, ints={16, 32, 64})
    OpcodeConstraints {
        flags: 0x59, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:964
        typeset_offset: 15, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:965
        constraint_offset: 53, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:966
    }
    ,
    // Uunarrow: fixed_results=1, use_typevar_operand=true, requires_typevar_operand=true, fixed_values=2
    // Constraints=['SplitLanes', 'Same', 'Same']
    // Polymorphic over TypeSet(lanes={2, 4, 8}, ints={16, 32, 64})
    OpcodeConstraints {
        flags: 0x59, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:964
        typeset_offset: 15, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:965
        constraint_offset: 53, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:966
    }
    ,
    // SwidenLow: fixed_results=1, use_typevar_operand=true, requires_typevar_operand=true, fixed_values=1
    // Constraints=['MergeLanes', 'Same']
    // Polymorphic over TypeSet(lanes={2, 4, 8, 16}, ints={8, 16, 32})
    OpcodeConstraints {
        flags: 0x39, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:964
        typeset_offset: 16, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:965
        constraint_offset: 56, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:966
    }
    ,
    // SwidenHigh: fixed_results=1, use_typevar_operand=true, requires_typevar_operand=true, fixed_values=1
    // Constraints=['MergeLanes', 'Same']
    // Polymorphic over TypeSet(lanes={2, 4, 8, 16}, ints={8, 16, 32})
    OpcodeConstraints {
        flags: 0x39, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:964
        typeset_offset: 16, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:965
        constraint_offset: 56, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:966
    }
    ,
    // UwidenLow: fixed_results=1, use_typevar_operand=true, requires_typevar_operand=true, fixed_values=1
    // Constraints=['MergeLanes', 'Same']
    // Polymorphic over TypeSet(lanes={2, 4, 8, 16}, ints={8, 16, 32})
    OpcodeConstraints {
        flags: 0x39, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:964
        typeset_offset: 16, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:965
        constraint_offset: 56, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:966
    }
    ,
    // UwidenHigh: fixed_results=1, use_typevar_operand=true, requires_typevar_operand=true, fixed_values=1
    // Constraints=['MergeLanes', 'Same']
    // Polymorphic over TypeSet(lanes={2, 4, 8, 16}, ints={8, 16, 32})
    OpcodeConstraints {
        flags: 0x39, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:964
        typeset_offset: 16, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:965
        constraint_offset: 56, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:966
    }
    ,
    // IaddPairwise: fixed_results=1, use_typevar_operand=true, requires_typevar_operand=false, fixed_values=2
    // Constraints=['Same', 'Same', 'Same']
    // Polymorphic over TypeSet(lanes={2, 4, 8, 16}, ints={8, 16, 32})
    OpcodeConstraints {
        flags: 0x49, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:964
        typeset_offset: 16, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:965
        constraint_offset: 0, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:966
    }
    ,
    // X86Pmaddubsw: fixed_results=1, use_typevar_operand=false, requires_typevar_operand=false, fixed_values=2
    // Constraints=['Concrete(ir::types::I16X8)', 'Concrete(ir::types::I8X16)', 'Concrete(ir::types::I8X16)']
    OpcodeConstraints {
        flags: 0x41, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:964
        typeset_offset: 255, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:965
        constraint_offset: 58, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:966
    }
    ,
    // Uextend: fixed_results=1, use_typevar_operand=false, requires_typevar_operand=false, fixed_values=1
    // Constraints=['Same', 'Narrower']
    // Polymorphic over TypeSet(lanes={1}, ints={8, 16, 32, 64, 128})
    OpcodeConstraints {
        flags: 0x21, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:964
        typeset_offset: 0, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:965
        constraint_offset: 61, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:966
    }
    ,
    // Sextend: fixed_results=1, use_typevar_operand=false, requires_typevar_operand=false, fixed_values=1
    // Constraints=['Same', 'Narrower']
    // Polymorphic over TypeSet(lanes={1}, ints={8, 16, 32, 64, 128})
    OpcodeConstraints {
        flags: 0x21, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:964
        typeset_offset: 0, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:965
        constraint_offset: 61, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:966
    }
    ,
    // Fpromote: fixed_results=1, use_typevar_operand=false, requires_typevar_operand=false, fixed_values=1
    // Constraints=['Same', 'Narrower']
    // Polymorphic over TypeSet(lanes={1}, floats={16, 32, 64, 128})
    OpcodeConstraints {
        flags: 0x21, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:964
        typeset_offset: 17, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:965
        constraint_offset: 61, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:966
    }
    ,
    // Fdemote: fixed_results=1, use_typevar_operand=false, requires_typevar_operand=false, fixed_values=1
    // Constraints=['Same', 'Wider']
    // Polymorphic over TypeSet(lanes={1}, floats={16, 32, 64, 128})
    OpcodeConstraints {
        flags: 0x21, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:964
        typeset_offset: 17, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:965
        constraint_offset: 51, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:966
    }
    ,
    // Fvdemote: fixed_results=1, use_typevar_operand=false, requires_typevar_operand=false, fixed_values=1
    // Constraints=['Concrete(ir::types::F32X4)', 'Concrete(ir::types::F64X2)']
    OpcodeConstraints {
        flags: 0x21, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:964
        typeset_offset: 255, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:965
        constraint_offset: 63, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:966
    }
    ,
    // FvpromoteLow: fixed_results=1, use_typevar_operand=false, requires_typevar_operand=false, fixed_values=1
    // Constraints=['Concrete(ir::types::F64X2)', 'Concrete(ir::types::F32X4)']
    OpcodeConstraints {
        flags: 0x21, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:964
        typeset_offset: 255, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:965
        constraint_offset: 64, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:966
    }
    ,
    // FcvtToUint: fixed_results=1, use_typevar_operand=false, requires_typevar_operand=false, fixed_values=1
    // Constraints=['Same', 'Free(17)']
    // Polymorphic over TypeSet(lanes={1}, ints={8, 16, 32, 64, 128})
    OpcodeConstraints {
        flags: 0x21, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:964
        typeset_offset: 0, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:965
        constraint_offset: 66, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:966
    }
    ,
    // FcvtToSint: fixed_results=1, use_typevar_operand=false, requires_typevar_operand=false, fixed_values=1
    // Constraints=['Same', 'Free(17)']
    // Polymorphic over TypeSet(lanes={1}, ints={8, 16, 32, 64, 128})
    OpcodeConstraints {
        flags: 0x21, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:964
        typeset_offset: 0, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:965
        constraint_offset: 66, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:966
    }
    ,
    // FcvtToUintSat: fixed_results=1, use_typevar_operand=false, requires_typevar_operand=false, fixed_values=1
    // Constraints=['Same', 'Free(14)']
    // Polymorphic over TypeSet(lanes={1, 2, 4, 8, 16, 32, 64, 128, 256}, ints={8, 16, 32, 64, 128})
    OpcodeConstraints {
        flags: 0x21, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:964
        typeset_offset: 3, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:965
        constraint_offset: 68, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:966
    }
    ,
    // FcvtToSintSat: fixed_results=1, use_typevar_operand=false, requires_typevar_operand=false, fixed_values=1
    // Constraints=['Same', 'Free(14)']
    // Polymorphic over TypeSet(lanes={1, 2, 4, 8, 16, 32, 64, 128, 256}, ints={8, 16, 32, 64, 128})
    OpcodeConstraints {
        flags: 0x21, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:964
        typeset_offset: 3, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:965
        constraint_offset: 68, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:966
    }
    ,
    // X86Cvtt2dq: fixed_results=1, use_typevar_operand=false, requires_typevar_operand=false, fixed_values=1
    // Constraints=['Same', 'Free(14)']
    // Polymorphic over TypeSet(lanes={1, 2, 4, 8, 16, 32, 64, 128, 256}, ints={8, 16, 32, 64, 128})
    OpcodeConstraints {
        flags: 0x21, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:964
        typeset_offset: 3, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:965
        constraint_offset: 68, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:966
    }
    ,
    // FcvtFromUint: fixed_results=1, use_typevar_operand=false, requires_typevar_operand=false, fixed_values=1
    // Constraints=['Same', 'Free(3)']
    // Polymorphic over TypeSet(lanes={1, 2, 4, 8, 16, 32, 64, 128, 256}, floats={16, 32, 64, 128})
    OpcodeConstraints {
        flags: 0x21, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:964
        typeset_offset: 18, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:965
        constraint_offset: 70, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:966
    }
    ,
    // FcvtFromSint: fixed_results=1, use_typevar_operand=false, requires_typevar_operand=false, fixed_values=1
    // Constraints=['Same', 'Free(3)']
    // Polymorphic over TypeSet(lanes={1, 2, 4, 8, 16, 32, 64, 128, 256}, floats={16, 32, 64, 128})
    OpcodeConstraints {
        flags: 0x21, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:964
        typeset_offset: 18, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:965
        constraint_offset: 70, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:966
    }
    ,
    // Isplit: fixed_results=2, use_typevar_operand=true, requires_typevar_operand=true, fixed_values=1
    // Constraints=['HalfWidth', 'HalfWidth', 'Same']
    // Polymorphic over TypeSet(lanes={1}, ints={16, 32, 64, 128})
    OpcodeConstraints {
        flags: 0x3a, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:964
        typeset_offset: 13, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:965
        constraint_offset: 72, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:966
    }
    ,
    // Iconcat: fixed_results=1, use_typevar_operand=true, requires_typevar_operand=true, fixed_values=2
    // Constraints=['DoubleWidth', 'Same', 'Same']
    // Polymorphic over TypeSet(lanes={1}, ints={8, 16, 32, 64})
    OpcodeConstraints {
        flags: 0x59, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:964
        typeset_offset: 8, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:965
        constraint_offset: 75, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:966
    }
    ,
    // AtomicRmw: fixed_results=1, use_typevar_operand=false, requires_typevar_operand=false, fixed_values=2
    // Constraints=['Same', 'Free(1)', 'Same']
    // Polymorphic over TypeSet(lanes={1}, ints={8, 16, 32, 64, 128})
    OpcodeConstraints {
        flags: 0x41, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:964
        typeset_offset: 0, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:965
        constraint_offset: 77, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:966
    }
    ,
    // AtomicCas: fixed_results=1, use_typevar_operand=true, requires_typevar_operand=false, fixed_values=3
    // Constraints=['Same', 'Free(1)', 'Same', 'Same']
    // Polymorphic over TypeSet(lanes={1}, ints={8, 16, 32, 64, 128})
    OpcodeConstraints {
        flags: 0x69, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:964
        typeset_offset: 0, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:965
        constraint_offset: 77, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:966
    }
    ,
    // AtomicLoad: fixed_results=1, use_typevar_operand=false, requires_typevar_operand=false, fixed_values=1
    // Constraints=['Same', 'Free(1)']
    // Polymorphic over TypeSet(lanes={1}, ints={8, 16, 32, 64, 128})
    OpcodeConstraints {
        flags: 0x21, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:964
        typeset_offset: 0, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:965
        constraint_offset: 12, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:966
    }
    ,
    // AtomicStore: fixed_results=0, use_typevar_operand=true, requires_typevar_operand=true, fixed_values=2
    // Constraints=['Same', 'Free(1)']
    // Polymorphic over TypeSet(lanes={1}, ints={8, 16, 32, 64, 128})
    OpcodeConstraints {
        flags: 0x58, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:964
        typeset_offset: 0, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:965
        constraint_offset: 12, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:966
    }
    ,
    // Fence: fixed_results=0, use_typevar_operand=false, requires_typevar_operand=false, fixed_values=0
    // Constraints=[]
    OpcodeConstraints {
        flags: 0x00, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:964
        typeset_offset: 255, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:965
        constraint_offset: 0, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:966
    }
    ,
    // ExtractVector: fixed_results=1, use_typevar_operand=true, requires_typevar_operand=true, fixed_values=1
    // Constraints=['DynamicToVector', 'Same']
    // Polymorphic over TypeSet(lanes={1}, ints={8, 16, 32, 64, 128}, floats={16, 32, 64, 128})
    OpcodeConstraints {
        flags: 0x39, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:964
        typeset_offset: 19, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:965
        constraint_offset: 81, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:966
    }
    ,
    // SequencePoint: fixed_results=0, use_typevar_operand=false, requires_typevar_operand=false, fixed_values=0
    // Constraints=[]
    OpcodeConstraints {
        flags: 0x00, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:964
        typeset_offset: 255, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:965
        constraint_offset: 0, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:966
    }
    ,
]; // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:971

// Table of value type sets.
const TYPE_SETS: [ir::instructions::ValueTypeSet; 20] = [ // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:854
    ir::instructions::ValueTypeSet {
        // TypeSet(lanes={1}, ints={8, 16, 32, 64, 128})
        lanes: ScalarBitSet::<u16>(1), // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:822
        dynamic_lanes: ScalarBitSet::<u16>(0), // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:822
        ints: ScalarBitSet::<u8>(248), // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:822
        floats: ScalarBitSet::<u8>(0), // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:822
    }
    ,
    ir::instructions::ValueTypeSet {
        // TypeSet(lanes={1}, ints={32, 64})
        lanes: ScalarBitSet::<u16>(1), // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:822
        dynamic_lanes: ScalarBitSet::<u16>(0), // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:822
        ints: ScalarBitSet::<u8>(96), // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:822
        floats: ScalarBitSet::<u8>(0), // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:822
    }
    ,
    ir::instructions::ValueTypeSet {
        // TypeSet(lanes={2, 4, 8, 16, 32, 64, 128, 256}, ints={8, 16, 32, 64, 128}, floats={16, 32, 64, 128})
        lanes: ScalarBitSet::<u16>(510), // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:822
        dynamic_lanes: ScalarBitSet::<u16>(510), // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:822
        ints: ScalarBitSet::<u8>(248), // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:822
        floats: ScalarBitSet::<u8>(240), // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:822
    }
    ,
    ir::instructions::ValueTypeSet {
        // TypeSet(lanes={1, 2, 4, 8, 16, 32, 64, 128, 256}, ints={8, 16, 32, 64, 128})
        lanes: ScalarBitSet::<u16>(511), // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:822
        dynamic_lanes: ScalarBitSet::<u16>(0), // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:822
        ints: ScalarBitSet::<u8>(248), // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:822
        floats: ScalarBitSet::<u8>(0), // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:822
    }
    ,
    ir::instructions::ValueTypeSet {
        // TypeSet(lanes={2, 4, 8, 16, 32, 64, 128, 256}, ints={8, 16, 32, 64, 128})
        lanes: ScalarBitSet::<u16>(510), // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:822
        dynamic_lanes: ScalarBitSet::<u16>(0), // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:822
        ints: ScalarBitSet::<u8>(248), // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:822
        floats: ScalarBitSet::<u8>(0), // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:822
    }
    ,
    ir::instructions::ValueTypeSet {
        // TypeSet(lanes={1, 2, 4, 8, 16, 32, 64, 128, 256}, ints={8, 16, 32, 64, 128}, floats={16, 32, 64, 128})
        lanes: ScalarBitSet::<u16>(511), // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:822
        dynamic_lanes: ScalarBitSet::<u16>(510), // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:822
        ints: ScalarBitSet::<u8>(248), // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:822
        floats: ScalarBitSet::<u8>(240), // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:822
    }
    ,
    ir::instructions::ValueTypeSet {
        // TypeSet(lanes={1}, ints={16, 32, 64})
        lanes: ScalarBitSet::<u16>(1), // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:822
        dynamic_lanes: ScalarBitSet::<u16>(0), // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:822
        ints: ScalarBitSet::<u8>(112), // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:822
        floats: ScalarBitSet::<u8>(0), // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:822
    }
    ,
    ir::instructions::ValueTypeSet {
        // TypeSet(lanes={1}, ints={64})
        lanes: ScalarBitSet::<u16>(1), // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:822
        dynamic_lanes: ScalarBitSet::<u16>(0), // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:822
        ints: ScalarBitSet::<u8>(64), // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:822
        floats: ScalarBitSet::<u8>(0), // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:822
    }
    ,
    ir::instructions::ValueTypeSet {
        // TypeSet(lanes={1}, ints={8, 16, 32, 64})
        lanes: ScalarBitSet::<u16>(1), // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:822
        dynamic_lanes: ScalarBitSet::<u16>(0), // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:822
        ints: ScalarBitSet::<u8>(120), // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:822
        floats: ScalarBitSet::<u8>(0), // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:822
    }
    ,
    ir::instructions::ValueTypeSet {
        // TypeSet(lanes={2, 4, 8, 16, 32, 64, 128, 256}, ints={8, 16, 32, 64, 128}, floats={16, 32, 64, 128})
        lanes: ScalarBitSet::<u16>(510), // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:822
        dynamic_lanes: ScalarBitSet::<u16>(0), // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:822
        ints: ScalarBitSet::<u8>(248), // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:822
        floats: ScalarBitSet::<u8>(240), // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:822
    }
    ,
    ir::instructions::ValueTypeSet {
        // TypeSet(lanes={1, 2, 4, 8, 16, 32, 64, 128, 256}, ints={8, 16, 32, 64, 128}, floats={16, 32, 64, 128})
        lanes: ScalarBitSet::<u16>(511), // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:822
        dynamic_lanes: ScalarBitSet::<u16>(0), // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:822
        ints: ScalarBitSet::<u8>(248), // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:822
        floats: ScalarBitSet::<u8>(240), // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:822
    }
    ,
    ir::instructions::ValueTypeSet {
        // TypeSet(lanes={1, 2, 4, 8, 16, 32, 64, 128, 256}, ints={8, 16, 32, 64, 128})
        lanes: ScalarBitSet::<u16>(511), // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:822
        dynamic_lanes: ScalarBitSet::<u16>(510), // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:822
        ints: ScalarBitSet::<u8>(248), // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:822
        floats: ScalarBitSet::<u8>(0), // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:822
    }
    ,
    ir::instructions::ValueTypeSet {
        // TypeSet(lanes={4, 8}, ints={16, 32})
        lanes: ScalarBitSet::<u16>(12), // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:822
        dynamic_lanes: ScalarBitSet::<u16>(0), // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:822
        ints: ScalarBitSet::<u8>(48), // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:822
        floats: ScalarBitSet::<u8>(0), // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:822
    }
    ,
    ir::instructions::ValueTypeSet {
        // TypeSet(lanes={1}, ints={16, 32, 64, 128})
        lanes: ScalarBitSet::<u16>(1), // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:822
        dynamic_lanes: ScalarBitSet::<u16>(0), // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:822
        ints: ScalarBitSet::<u8>(240), // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:822
        floats: ScalarBitSet::<u8>(0), // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:822
    }
    ,
    ir::instructions::ValueTypeSet {
        // TypeSet(lanes={1, 2, 4, 8, 16, 32, 64, 128, 256}, floats={16, 32, 64, 128})
        lanes: ScalarBitSet::<u16>(511), // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:822
        dynamic_lanes: ScalarBitSet::<u16>(510), // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:822
        ints: ScalarBitSet::<u8>(0), // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:822
        floats: ScalarBitSet::<u8>(240), // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:822
    }
    ,
    ir::instructions::ValueTypeSet {
        // TypeSet(lanes={2, 4, 8}, ints={16, 32, 64})
        lanes: ScalarBitSet::<u16>(14), // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:822
        dynamic_lanes: ScalarBitSet::<u16>(14), // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:822
        ints: ScalarBitSet::<u8>(112), // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:822
        floats: ScalarBitSet::<u8>(0), // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:822
    }
    ,
    ir::instructions::ValueTypeSet {
        // TypeSet(lanes={2, 4, 8, 16}, ints={8, 16, 32})
        lanes: ScalarBitSet::<u16>(30), // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:822
        dynamic_lanes: ScalarBitSet::<u16>(30), // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:822
        ints: ScalarBitSet::<u8>(56), // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:822
        floats: ScalarBitSet::<u8>(0), // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:822
    }
    ,
    ir::instructions::ValueTypeSet {
        // TypeSet(lanes={1}, floats={16, 32, 64, 128})
        lanes: ScalarBitSet::<u16>(1), // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:822
        dynamic_lanes: ScalarBitSet::<u16>(0), // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:822
        ints: ScalarBitSet::<u8>(0), // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:822
        floats: ScalarBitSet::<u8>(240), // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:822
    }
    ,
    ir::instructions::ValueTypeSet {
        // TypeSet(lanes={1, 2, 4, 8, 16, 32, 64, 128, 256}, floats={16, 32, 64, 128})
        lanes: ScalarBitSet::<u16>(511), // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:822
        dynamic_lanes: ScalarBitSet::<u16>(0), // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:822
        ints: ScalarBitSet::<u8>(0), // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:822
        floats: ScalarBitSet::<u8>(240), // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:822
    }
    ,
    ir::instructions::ValueTypeSet {
        // TypeSet(lanes={1}, ints={8, 16, 32, 64, 128}, floats={16, 32, 64, 128})
        lanes: ScalarBitSet::<u16>(1), // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:822
        dynamic_lanes: ScalarBitSet::<u16>(510), // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:822
        ints: ScalarBitSet::<u8>(248), // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:822
        floats: ScalarBitSet::<u8>(240), // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:822
    }
    ,
]; // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:871

// Table of operand constraint sequences.
const OPERAND_CONSTRAINTS: [OperandConstraint; 83] = [ // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:978
    OperandConstraint::Same, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:985
    OperandConstraint::Same, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:985
    OperandConstraint::Same, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:985
    OperandConstraint::Concrete(ir::types::I32), // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:985
    OperandConstraint::Same, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:985
    OperandConstraint::LaneOf, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:985
    OperandConstraint::Concrete(ir::types::I8X16), // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:985
    OperandConstraint::Concrete(ir::types::I8X16), // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:985
    OperandConstraint::Concrete(ir::types::I8X16), // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:985
    OperandConstraint::Same, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:985
    OperandConstraint::Same, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:985
    OperandConstraint::LaneOf, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:985
    OperandConstraint::Same, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:985
    OperandConstraint::Free(1), // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:985
    OperandConstraint::Concrete(ir::types::I64), // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:985
    OperandConstraint::Same, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:985
    OperandConstraint::Concrete(ir::types::I64), // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:985
    OperandConstraint::Free(1), // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:985
    OperandConstraint::Same, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:985
    OperandConstraint::Same, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:985
    OperandConstraint::Same, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:985
    OperandConstraint::Same, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:985
    OperandConstraint::Concrete(ir::types::I16X8), // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:985
    OperandConstraint::Same, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:985
    OperandConstraint::Concrete(ir::types::I32X4), // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:985
    OperandConstraint::Same, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:985
    OperandConstraint::Concrete(ir::types::I64X2), // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:985
    OperandConstraint::Same, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:985
    OperandConstraint::Concrete(ir::types::F16), // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:985
    OperandConstraint::Concrete(ir::types::F32), // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:985
    OperandConstraint::Concrete(ir::types::F64), // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:985
    OperandConstraint::Concrete(ir::types::F128), // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:985
    OperandConstraint::Same, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:985
    OperandConstraint::Free(0), // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:985
    OperandConstraint::Same, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:985
    OperandConstraint::Same, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:985
    OperandConstraint::Concrete(ir::types::I8), // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:985
    OperandConstraint::Same, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:985
    OperandConstraint::Free(9), // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:985
    OperandConstraint::AsTruthy, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:985
    OperandConstraint::Same, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:985
    OperandConstraint::Same, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:985
    OperandConstraint::Concrete(ir::types::I8), // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:985
    OperandConstraint::Same, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:985
    OperandConstraint::Same, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:985
    OperandConstraint::Concrete(ir::types::I8), // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:985
    OperandConstraint::Same, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:985
    OperandConstraint::Same, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:985
    OperandConstraint::Free(0), // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:985
    OperandConstraint::Same, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:985
    OperandConstraint::Free(5), // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:985
    OperandConstraint::Same, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:985
    OperandConstraint::Wider, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:985
    OperandConstraint::SplitLanes, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:985
    OperandConstraint::Same, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:985
    OperandConstraint::Same, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:985
    OperandConstraint::MergeLanes, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:985
    OperandConstraint::Same, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:985
    OperandConstraint::Concrete(ir::types::I16X8), // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:985
    OperandConstraint::Concrete(ir::types::I8X16), // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:985
    OperandConstraint::Concrete(ir::types::I8X16), // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:985
    OperandConstraint::Same, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:985
    OperandConstraint::Narrower, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:985
    OperandConstraint::Concrete(ir::types::F32X4), // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:985
    OperandConstraint::Concrete(ir::types::F64X2), // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:985
    OperandConstraint::Concrete(ir::types::F32X4), // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:985
    OperandConstraint::Same, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:985
    OperandConstraint::Free(17), // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:985
    OperandConstraint::Same, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:985
    OperandConstraint::Free(14), // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:985
    OperandConstraint::Same, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:985
    OperandConstraint::Free(3), // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:985
    OperandConstraint::HalfWidth, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:985
    OperandConstraint::HalfWidth, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:985
    OperandConstraint::Same, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:985
    OperandConstraint::DoubleWidth, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:985
    OperandConstraint::Same, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:985
    OperandConstraint::Same, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:985
    OperandConstraint::Free(1), // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:985
    OperandConstraint::Same, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:985
    OperandConstraint::Same, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:985
    OperandConstraint::DynamicToVector, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:985
    OperandConstraint::Same, // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:985
]; // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_inst.rs:988
