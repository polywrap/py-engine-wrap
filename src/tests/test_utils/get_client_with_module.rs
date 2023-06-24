use std::sync::Arc;

use polywrap_client::{client::PolywrapClient, wasm::wasm_wrapper::WasmWrapper, core::{uri::Uri, file_reader::SimpleFileReader, client::UriRedirect}, msgpack, builder::PolywrapClientConfig};

pub fn get_client_with_module(module: &[u8]) -> PolywrapClient {
  let config = {
      PolywrapClientConfig {
          interfaces: None,
          envs: None,
          wrappers: Some(vec![
            //   (
            //       Uri::try_from("wrap://mock/test").unwrap(),
            //       Arc::new(WasmWrapper::new(module.to_vec(), Arc::new(SimpleFileReader::new()))),
            //   )
          ]),
          packages: None,
          redirects: None,
          resolvers: None,
      }
  };
  let client = PolywrapClient::new(config.into());

  client
}
