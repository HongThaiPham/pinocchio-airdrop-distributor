#![no_std]
#![allow(unexpected_cfgs)]
use pinocchio::{no_allocator, nostd_panic_handler, program_entrypoint};
pub mod errors;
pub mod instructions;
pub mod processor;
pub mod states;
pub mod utils;

use processor::process_instruction;

pinocchio_pubkey::declare_id!("FoNu94ZtecyvwuJ1BvKXkmpbGFaiZ5TCcpe9yXdcQbr2");

program_entrypoint!(process_instruction);
no_allocator!();
nostd_panic_handler!();
