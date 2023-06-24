use std::{collections::HashMap, borrow::Borrow};

use polywrap_wasm_rs::subinvoke;
use rustpython_vm as vm;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use vm::{
    builtins::{PyBaseException, PyTupleRef, PyStrRef, PyDict, PyIntRef, PyStr, PyBool, PyInt, PyFloat, PyNone, PyList}, 
    TryFromObject, convert::ToPyObject, py_serde::PyObjectSerializer, PyObjectRef, PyObject, VirtualMachine, PyResult, function::{FuncArgs, PyNativeFunc, IntoPyNativeFunc}, PyRef, match_class,
};

use crate::{GlobalVar, json_to_msgpack, msgpack_to_json};
use num_traits::cast::ToPrimitive;
// let code_obj = vm
//     .compile(src, vm::compiler::Mode::Exec, "<embedded>".to_owned())
//     .map_err(|err| vm.new_syntax_error(&err, Some(src))).unwrap();

// let result = vm.run_code_obj(code_obj, scope).unwrap();
fn subinvoke(args: FuncArgs, vm: &VirtualMachine) -> PyResult<PyObjectRef> {
    let arg = args.args.get(0).unwrap();
    let arg = arg.str(vm)?;
    let uri: String = arg.to_string();
    
    let arg = args.args.get(1).unwrap();
    let arg = arg.str(vm)?;
    let method: String = arg.to_string();

    let arg = args.args.get(2).unwrap();
    let args = pyobj_to_json(vm, arg.clone());
    let args = json_to_msgpack(&args);

    let result: Result<Vec<u8>, String> = subinvoke::wrap_subinvoke(
        &uri,
        &method,
        args,
    );

    let json = match result {
        Ok(result) => msgpack_to_json(&result),
        Err(err) => {
            serde_json::to_string(&err).unwrap()
        }
    };

    let result = match serde_json::from_str(&json) {
        Ok(json) => json_to_pyobj(vm, &json),
        Err(err) => {
            let json = serde_json::to_string(&err.to_string()).unwrap();
            let json = serde_json::from_str(&json).unwrap();
            json_to_pyobj(vm, &json)
        }
    };

    Ok(result)
}

fn abort(args: FuncArgs, vm: &VirtualMachine) -> PyResult<PyObjectRef> {
    let arg = args.args.get(0).unwrap();
    let arg = arg.str(vm)?;
    let message: String = arg.to_string();
    
    panic!("{}", message);
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
pub struct MockType {
    pub prop: String,
}
fn mock_subinvoke(args: FuncArgs, vm: &VirtualMachine) -> PyResult<PyObjectRef> {
    let arg = args.args.get(0).unwrap();
    let arg = arg.str(vm)?;
    let uri: String = arg.to_string();
    
    let arg = args.args.get(1).unwrap();
    let arg = arg.str(vm)?;
    let method: String = arg.to_string();

    let arg = args.args.get(2).unwrap();
    let args = pyobj_to_json(vm, arg.clone());
    let _args = json_to_msgpack(&args);

    let input = MockType {
        prop: uri + "/" + &method + "/" + serde_json::to_string(&args).unwrap().as_str(),
    };
    let result = rmp_serde::encode::to_vec(&input).unwrap();

    let json = msgpack_to_json(&result);

    let result = match serde_json::from_str(&json) {
        Ok(json) => json_to_pyobj(vm, &json),
        Err(err) => {
            let json = serde_json::to_string(&err.to_string()).unwrap();
            let json = serde_json::from_str(&json).unwrap();
            json_to_pyobj(vm, &json)
        }
    };

    Ok(result)
}

fn pyobj_to_json(vm: &VirtualMachine, pyobj: PyObjectRef) -> Value {
    if let Ok(string) = pyobj.clone().downcast::<PyStr>() {
        Value::String(string.as_str().to_owned())
    } else if let Ok(int) = pyobj.clone().downcast::<PyInt>() {
        Value::Number(serde_json::Number::from(int.as_bigint().to_i64().expect("Conversion to i64 failed")))
    } else if let Ok(float) = pyobj.clone().downcast::<PyFloat>() {
        Value::Number(serde_json::Number::from_f64(float.to_f64()).unwrap())
    } else if let Ok(_) = pyobj.clone().downcast::<PyNone>() {
        Value::Null
    } else if let Ok(list) = pyobj.clone().downcast::<PyList>() {
        let vec: Vec<Value> = list
            .borrow_vec()
            .iter()
            .map(|obj| pyobj_to_json(vm, obj.clone()))
            .collect();

        Value::Array(vec)
    } else if let Ok(dict) = pyobj.clone().downcast::<PyDict>() {
        let obj = dict
            .into_iter()
            .map(|(k, v)| {
                let key = match k.downcast::<PyStr>() {
                    Ok(string) => string.as_str().to_owned(),
                    Err(_) => panic!("Unsupported Python type"),
                };
                (key, pyobj_to_json(vm, v))
            })
            .collect::<serde_json::Map<String, Value>>();

        Value::Object(obj)
    } else if let Ok(boolean) = pyobj.clone().try_to_bool(vm) {
        Value::Bool(boolean)
    } else {
        panic!("Unsupported Python type")
    }
}

pub fn eval_and_parse(src: &str, globals: Option<Vec<GlobalVar>>) -> Result<Value, String> {
    let mut my_globals: HashMap<String, PyObjectRef> = HashMap::new();

    vm::Interpreter::with_init(Default::default(), |vm| {
        if globals.is_some() {
            for GlobalVar { name, value } in globals.unwrap() {
                let py_value = json_to_pyobj(vm, &value);
                my_globals.insert(name, py_value);
            }
        }
    }).enter(|vm| {
        let scope = vm.new_scope_with_builtins();
        scope.globals.set_item("__wrap_subinvoke", vm.ctx.new_function("__wrap_subinvoke", subinvoke).into(), vm).unwrap();
        scope.globals.set_item("__wrap_abort", vm.ctx.new_function("__wrap_abort", abort).into(), vm).unwrap();
        scope.globals.set_item("__mock_subinvoke", vm.ctx.new_function("__mock_subinvoke", mock_subinvoke).into(), vm).unwrap();

        for (name, py_value) in my_globals {
            scope.globals.set_item(&name, py_value, vm).unwrap();
        }

        let result = vm.run_block_expr(scope, src).map_err(|e| exception_to_string(&e, vm))?;

        let result = PyObjectSerializer::new(vm, &result);
        let result = serde_json::to_value(&result)
            .map_err(|e| e.to_string());

        result
    })
}
fn json_to_pyobj(vm: &VirtualMachine, json: &Value) -> PyObjectRef {
    match json {
        Value::Null => vm.ctx.none(),
        Value::Bool(b) => vm.ctx.new_bool(*b).to_pyobject(vm),
        Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                vm.ctx.new_int(i).to_pyobject(vm)
            } else if let Some(f) = n.as_f64() {
                vm.ctx.new_float(f).to_pyobject(vm)
            } else {
                vm.ctx.none()
            }
        }
        Value::String(s) => vm.ctx.new_str(s.clone()).to_pyobject(vm),
        Value::Array(a) => vm.ctx.new_list(
            a.iter()
                .map(|v| json_to_pyobj(vm, v))
                .collect(),
        ).to_pyobject(vm),
        Value::Object(o) => {
            let map = o
                .iter()
                .map(|(k, v)| (vm.ctx.intern_str(k.as_str()), json_to_pyobj(vm, v)))
                .collect();

            let dict = PyDict::from_attributes(map, vm).unwrap();

            dict.to_pyobject(vm)
        },
        _ => panic!("Unsupported JSON type") 
    }
}

fn exception_to_string(exception: &PyBaseException, vm: &vm::VirtualMachine) -> String {
        let args = match PyTupleRef::try_from_object(&vm, exception.args().to_pyobject(vm)) {
            Ok(args) => args,
            Err(_) => return "<error obtaining exception arguments>".to_string(),
        };
        let mut error_message = String::new();
        for arg in args.iter() {
            match PyStrRef::try_from_object(&vm, arg.clone()) {
                Ok(s) => {
                    error_message.push_str(&s.as_str());
                    error_message.push_str(" ");
                },
                Err(_) => {
                    error_message.push_str("<unprintable>");
                    error_message.push_str(" ");
                },
            }
        }
        error_message.trim().to_string()
    }

#[cfg(test)]
mod tests {
    use core::panic;

    use serde_json::{Value, json};

    use crate::{eval::eval_and_parse, GlobalVar};

    #[test]
    fn eval_aaa() {
        let src = r#"
# create class
class Animal:
    name = "yo"
    gear = 0
    def to_json(self):
        return {
            "name": self.name,
            "gear": self.gear
        }

class Bike:
    name = "biker"
    animal = Animal()
    def to_json(self):
        return {
            "name": self.name,
            "animal": self.animal.to_json()
        }

# create objects of class
bike1 = Bike()

mock_subinvoke("uriaaa", "metth", {"a": 4, "b": 2})
"#;
        // let src = "return 2+2";
    
        let result = eval_and_parse(src, Some(vec![
            GlobalVar {
                name: "test".to_string(),
                value: json!("hey you")
            }
        ]));
        // Unwrap and turn to pretty string
        let result = result.unwrap();
        
        println!("result: {:?}", result);
        panic!("test");

        // assert_eq!(result, 4.into());

        // match result {
        //     Ok(_) => panic!("Expected error for undefined variable, but didn't get one"),
        //     Err(e) => assert!(e.contains("notDefinedVariable is not defined"), "Unexpected error message: {}", e),
        // }
    }
}
