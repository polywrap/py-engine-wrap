mod wrap;
use eval::eval_and_parse;
use serde_json::Value;
use wrap::{*, module::serialization::ArgsEvalWithGlobals};
mod eval;
pub use wrap::{EvalResult, GlobalVar, ArgsEval, ModuleTrait, Module};
use getrandom::register_custom_getrandom;

fn custom_getrandom(_: &mut [u8]) -> Result<(), getrandom::Error> {
    return Ok(());
}

register_custom_getrandom!(custom_getrandom);

impl ModuleTrait for Module {
    fn eval(args: ArgsEval) -> Result<EvalResult, String> {
        let result = eval_and_parse(&args.src, None);

        match result {
            Ok(result) => {
                Ok(EvalResult {
                    value: Some(result),
                    error: None
                })
            },
            Err(err) => {
                Ok(EvalResult {
                    value: None,
                    error: Some(err)
                })
            }
        }
    }

    fn eval_with_globals(args: ArgsEvalWithGlobals) -> Result<EvalResult, String> {
        let result = eval_and_parse(&args.src, Some(args.globals));

        match result {
            Ok(result) => {
                Ok(EvalResult {
                    value: Some(result),
                    error: None
                })
            },
            Err(err) => {
                Ok(EvalResult {
                    value: None,
                    error: Some(err)
                })
            }
        }
    }
}

pub fn msgpack_to_json(bytes: &[u8]) -> String {
    let value: rmpv::Value = rmp_serde::from_slice(&bytes).unwrap();
    serde_json::to_string(&value).unwrap()
}

pub fn json_to_msgpack(value: &Value) -> Vec<u8> {
    rmp_serde::encode::to_vec(value).unwrap()
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use polywrap_client::{msgpack, wasm::wasm_wrapper::WasmWrapper, core::{file_reader::SimpleFileReader, wrapper::Wrapper}};
    use serde_json::json;

    use crate::{EvalResult, tests::test_utils::{get_client_with_module, load_wrap}};

    mod test_utils;

//     #[test]
//     fn sanity() {
//         let (_manifest, module) = load_wrap("./bin");

//         let client = get_client_with_module(&module);

//         // let result = invoke_client("mock/test", "eval", &msgpack::msgpack!({
//         //     "src": "const temp = 'Hello world'; temp"
//         // }), &client);
//         let wrapper = Arc::new(WasmWrapper::new(module.clone(), Arc::new(SimpleFileReader::new())));
//         let result = wrapper.invoke("eval", Some(&msgpack::msgpack!({
// "src": r#"x = {
//     "name": "John",
//     "age": 30,
//     "city": "New York"
// }
// x["name"]"#
//         })), 
//             None,
//             Arc::new(client.clone()),
//         None).unwrap();

//         let result: EvalResult = rmp_serde::from_slice(&result).unwrap();

//         assert_eq!(result.value.unwrap(), EvalResult {
//             value: Some(json!("Hello world")),
//             error: None
//         }.value.unwrap());
//     }
}