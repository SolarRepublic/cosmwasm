extern crate hackatom;

use std::fs;
use std::str::from_utf8;

use wasmer_runtime::{compile_with, Ctx, Func, func, imports};
use wasmer_runtime_core::{Instance};
use wasmer_clif_backend::CraneliftCompiler;

use hackatom::types::{mock_params, coin};
use hackatom::contract::{RegenInitMsg};

#[test]
fn test_coin() {
    let c = hackatom::types::coin("123", "tokens");
    assert_eq!(c.len(), 1);
    assert_eq!(c.get(0).unwrap().amount, "123");
}

/**
This integration test tries to run and call the generated wasm.
It depends on a release build being available already. You can create that with:

cargo wasm && wasm-gc ./target/wasm32-unknown-unknown/release/hackatom.wasm

Then running `cargo test` will validate we can properly call into that generated data.
**/

#[test]
fn run_contract() {
    let wasm_file = "./target/wasm32-unknown-unknown/release/hackatom.wasm";
    let wasm = fs::read(wasm_file).unwrap();
    assert!(wasm.len() > 100000);

    // TODO: set up proper callback for read and write here
    let import_object = imports! {
        "env" => {
            "c_read" => func!(do_read),
            "c_write" => func!(do_write),
        },
    };

    // create the instance
    let module = compile_with(&wasm, &CraneliftCompiler::new()).unwrap();
    let mut instance = module.instantiate (&import_object).unwrap();

    // prepare arguments
    let params = mock_params("creator", &coin("1000", "earth"), &[]);
    let mut json_params = serde_json::to_vec(&params).unwrap();
    // currently we need to 0 pad it
    json_params.push(0);

    let msg = &RegenInitMsg {
        verifier: String::from("verifies"),
        beneficiary: String::from("benefits"),
    };
    let mut json_msg = serde_json::to_vec(&msg).unwrap();
    json_msg.push(0);

    // place data in the instance memory
    let param_offset = allocate(&mut instance, &json_params);
    let msg_offset = allocate(&mut instance, &json_msg);

    // call the instance
    let alloc: Func<(i32, i32, i32), (i32)> = instance.func("init_wrapper").unwrap();
    let res_offset = alloc.call(15, param_offset, msg_offset).unwrap();
    assert!(res_offset > 1000);

    // read the return value
    let res = read_memory(instance.context(), res_offset, 11);
    let str_res = from_utf8(&res).unwrap();
    assert_eq!(str_res , "{\"msgs\":[]}");
}

// write_mem allocates memory in the instance and copies the given data in
// returns the memory offset, to be passed as an argument
// panics on any error (TODO, use result?)
fn allocate(instance: &mut Instance, data: &[u8]) -> i32 {
    // allocate
    let alloc: Func<(i32), (i32)> = instance.func("allocate").unwrap();
    let offset = alloc.call(data.len() as i32).unwrap();
    write_memory(instance.context(), offset, data);
    offset
}
// TODO: free_mem

fn read_memory(ctx: &Ctx, offset: i32, len: i32) -> Vec<u8> {
    let start = offset as usize;
    let end = start + len as usize;
    // TODO: there must be a faster way to copy memory
    let memory = &ctx.memory(0).view::<u8>()[start..end];

    let ulen = len as usize;
    let mut result = vec![0; ulen];
    for i in 0..ulen {
        result[i] = memory[i].get();
    }
    result
}

fn write_memory(ctx: &Ctx, offset: i32, data: &[u8]) {
    let start = offset as usize;
    let end = start + data.len();
    // TODO: there must be a faster way to copy memory
    let memory = &ctx.memory(0).view::<u8>()[start..end];
    for i in 0..data.len() {
        memory[i].set(data[i])
    }
}


fn do_read(ctx: &mut Ctx, dbref: i32, key: i32) -> i32 {
    15
}

fn do_write(ctx: &mut Ctx, dbref: i32, key: i32, value: i32) {
}