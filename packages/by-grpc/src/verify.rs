use std::collections::HashSet;

use prost::Message;
use prost_types::FileDescriptorSet;

/// One disagreement between the compiled proto descriptors and the `#[grpc]` registry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Mismatch {
    /// The proto declares the method but no `#[grpc]` function handles it.
    Unregistered { path: String },
    /// A `#[grpc]` function handles a path the descriptors do not declare.
    UnknownMethod { path: String, source: &'static str },
    /// The proto request type differs from the function's request message.
    RequestType { path: String, proto: String, rust: String },
    /// The proto response type differs from the function's response message.
    ResponseType { path: String, proto: String, rust: String },
    /// Streaming methods are not supported by `#[grpc]` yet.
    Streaming { path: String },
    /// A descriptor set could not be decoded.
    InvalidDescriptor,
}

/// Checks the registry against `FileDescriptorSet` bytes emitted by `tonic-build`
/// (`file_descriptor_set_path`). Call once at boot so a proto/handler drift fails the
/// deploy instead of a request.
pub fn verify(descriptor_sets: &[&[u8]]) -> Result<(), Vec<Mismatch>> {
    let mut mismatches = Vec::new();
    let mut declared = HashSet::new();
    for bytes in descriptor_sets {
        let Ok(set) = FileDescriptorSet::decode(*bytes) else {
            mismatches.push(Mismatch::InvalidDescriptor);
            continue;
        };
        for file in &set.file {
            let package = file.package();
            for service in &file.service {
                for method in &service.method {
                    let path = format!("/{package}.{}/{}", service.name(), method.name());
                    declared.insert(path.clone());
                    if method.client_streaming() || method.server_streaming() {
                        mismatches.push(Mismatch::Streaming { path });
                        continue;
                    }
                    let Some(registered) = crate::methods().find(|m| m.path == path) else {
                        mismatches.push(Mismatch::Unregistered { path });
                        continue;
                    };
                    let proto_req = method.input_type().trim_start_matches('.').to_string();
                    let rust_req = (registered.request_type)();
                    if proto_req != rust_req {
                        mismatches.push(Mismatch::RequestType {
                            path: path.clone(),
                            proto: proto_req,
                            rust: rust_req,
                        });
                    }
                    let proto_res = method.output_type().trim_start_matches('.').to_string();
                    let rust_res = (registered.response_type)();
                    if proto_res != rust_res {
                        mismatches.push(Mismatch::ResponseType {
                            path,
                            proto: proto_res,
                            rust: rust_res,
                        });
                    }
                }
            }
        }
    }
    for registered in crate::methods() {
        if !declared.contains(registered.path) {
            mismatches.push(Mismatch::UnknownMethod {
                path: registered.path.to_string(),
                source: registered.source,
            });
        }
    }
    if mismatches.is_empty() {
        Ok(())
    } else {
        Err(mismatches)
    }
}
