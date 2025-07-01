mod api;
mod cache;
mod internal;
mod memory;
mod message;
mod response;
mod runtime_data;
mod storage;

use memory::*;

use api::to_api_function;
use wasmer::{Exports, Function, FunctionEnv, Module, Store};

use crate::executor::Env;

/// Errors that can be encountered during execution
#[derive(Debug)]
pub enum FunctionErrors {
    ApiNotConfigured = -1,
    ReturnBufferTooSmall = -2,
    ErrorCouldNotSerialize = -3,
    InternalApiError = -4,
    ParametersNotUtf8 = -5,
    InvalidPointer = -6,
    CacheDisabled = -7,
    CouldNotGetAdequateMemory = -8,
    FailedToWriteGuestMemory = -9,
    StorageLimitReached = -10,
    TestMode = -11,
    OperationNotAllowed = -12,
    SharedDbError = -13,
}

#[derive(Debug)]
pub enum LinkError {
    NoSuchFunction(String),
}

impl std::fmt::Display for LinkError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LinkError::NoSuchFunction(name) => write!(f, "No such function: {}", name),
        }
    }
}

pub fn fake_stdio_exit() {
    warn!("placeholder")
}
pub fn fake_proc_exit(placeholder: i32) {
    warn!("placeholder")
}
pub fn fake_main_main(placeholder: i32) {
    warn!("placeholder")
}
pub fn fake_arc4random(placeholder: i32) {
    warn!("placeholder")
}
pub fn fake_fd_write(placeholder: i32) {
    warn!("placeholder")
}
pub fn fake_tinygo_getCurrentStackPointer(placeholder: i32) {
    warn!("placeholder")
}
pub fn fake_runtime_ticks(placeholder: i32) {
    warn!("placeholder")
}
pub fn fake_syscall_js_valueGet(placeholder: i32) {
    warn!("placeholder")
}
pub fn fake_syscall_js_valuePrepareString(placeholder: i32) {
    warn!("placeholder")
}
pub fn fake_syscall_js_valueLoadString(placeholder: i32) {
    warn!("placeholder")
}
pub fn fake_syscall_js_finalizeRef(placeholder: i32) {
    warn!("placeholder")
}

pub fn fake_wbindgen_describe(placeholder: i32) {
    warn!("Fake __wbindgen_describe called with placeholder: {placeholder}");
}

pub fn fake_wbindgen_throw(x: i32, y: i32) {
    warn!("Fake __wbindgen_throw called with placeholder: {x}, {y}");
}

pub fn fake_wbindgen_externref_table_grow(x: i32) -> i32 {
    warn!("Fake __wbindgen_externref_table_grow called with placeholder: {x}");
    return 0;
}

pub fn fake_wbindgen_externref_table_set_null(placeholder: i32) {
    warn!("Fake __wbindgen_externref_table_set_null called with placeholder: {placeholder}");
}

pub fn link_functions_to_module(
    module: &Module,
    mut store: &mut Store,
    env: FunctionEnv<Env>,
) -> Result<Exports, LinkError> {
    let mut exports = Exports::new();

    for import in module.imports() {
        let function_name = import.name();

        if function_name.starts_with("__wbindgen") {
            continue;
        }

        let func = to_api_function(function_name, &mut store, env.clone());
        if let Some(func) = func {
            exports.insert(function_name.to_string(), func);
            continue;
        }

        return Err(LinkError::NoSuchFunction(function_name.to_string()));
    }
    Ok(exports)
}

pub fn create_bindgen_placeholder(mut store: &mut Store) -> Exports {
    let mut exports = Exports::new();

    exports.insert(
        "__wbindgen_describe",
        Function::new_typed(&mut store, fake_wbindgen_describe),
    );

    exports.insert(
        "__wbindgen_throw",
        Function::new_typed(&mut store, fake_wbindgen_throw),
    );

    // exports.insert("proc_exit", Function::new_typed(&mut store, fake_proc_exit));
    // exports.insert("main.main", Function::new_typed(&mut store, fake_main_main));
    // exports.insert(
    //     "arc4random",
    //     Function::new_typed(&mut store, fake_arc4random),
    // );
    // exports.insert("fd_write", Function::new_typed(&mut store, fake_fd_write));
    // exports.insert(
    //     "tinygo_getCurrentStackPointer",
    //     Function::new_typed(&mut store, fake_tinygo_getCurrentStackPointer),
    // );
    // exports.insert(
    //     "runtime.ticks",
    //     Function::new_typed(&mut store, fake_runtime_ticks),
    // );
    // exports.insert(
    //     "syscall/js.valueGet",
    //     Function::new_typed(&mut store, fake_syscall_js_valueGet),
    // );
    // exports.insert(
    //     "syscall/js.valuePrepareString",
    //     Function::new_typed(&mut store, fake_syscall_js_valuePrepareString),
    // );
    // exports.insert(
    //     "syscall/js.valueLoadString",
    //     Function::new_typed(&mut store, fake_syscall_js_valueLoadString),
    // );
    // exports.insert(
    //     "syscall/js.finalizeRef",
    //     Function::new_typed(&mut store, fake_syscall_js_finalizeRef),
    // );

    exports
}

pub fn create_stdio_placehodler(mut store: &mut Store) -> Exports {
    let mut exports = Exports::new();

    exports.insert(
        "__stdio_exit",
        Function::new_typed(&mut store, fake_stdio_exit),
    );

    exports
}

pub fn create_bindgen_externref_xform(mut store: &mut Store) -> Exports {
    let mut exports = Exports::new();

    exports.insert(
        "__stdio_exit",
        Function::new_typed(&mut store, fake_stdio_exit),
    );

    exports.insert(
        "__wbindgen_externref_table_grow",
        Function::new_typed(&mut store, fake_wbindgen_externref_table_grow),
    );

    exports.insert(
        "__wbindgen_externref_table_set_null",
        Function::new_typed(&mut store, fake_wbindgen_externref_table_set_null),
    );

    exports
}
