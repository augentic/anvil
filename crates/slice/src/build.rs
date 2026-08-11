//! Target build envelope kernel: request assembly, the D2
//! [`canonical`]izer and report [`gate`], the D4 repair [`brief`],
//! the D6 [`attempt`] store, and the D5 artifact [`stage`] (RFC-90).

pub(crate) mod assemble;
pub mod attempt;
pub mod brief;
pub mod canonical;
pub mod deferred;
pub mod gate;
pub mod stage;
