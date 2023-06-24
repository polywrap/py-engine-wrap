use std::fs;

pub fn load_wrap(path: &str) -> (Vec<u8>, Vec<u8>) {
  let module = fs::read(format!("{path}/wrap.wasm")).expect("Unable to read wrap module file");

  (vec![], module)
}
