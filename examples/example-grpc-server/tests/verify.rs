use by_grpc::{MethodKind, Mismatch};
use prost::Message;

fn descriptor_set() -> prost_types::FileDescriptorSet {
    prost_types::FileDescriptorSet::decode(example_grpc_server::DESCRIPTOR_SET).unwrap()
}

fn method_mut<'a>(
    set: &'a mut prost_types::FileDescriptorSet,
    package: &str,
    name: &str,
) -> &'a mut prost_types::MethodDescriptorProto {
    set.file
        .iter_mut()
        .find(|f| f.package() == package)
        .unwrap()
        .service[0]
        .method
        .iter_mut()
        .find(|m| m.name() == name)
        .unwrap()
}

fn verify(set: &prost_types::FileDescriptorSet) -> Result<(), Vec<Mismatch>> {
    by_grpc::verify(&[set.encode_to_vec().as_slice()])
}

#[test]
fn registry_matches_descriptors() {
    by_grpc::verify(&[example_grpc_server::DESCRIPTOR_SET]).unwrap();
    assert_eq!(by_grpc::methods().count(), 8);
}

#[test]
fn reports_proto_methods_without_a_handler() {
    let mut set = descriptor_set();
    let mut extra = method_mut(&mut set, "biyard.greeter.v1", "SayHello").clone();
    extra.name = Some("Wave".into());
    set.file
        .iter_mut()
        .find(|f| f.package() == "biyard.greeter.v1")
        .unwrap()
        .service[0]
        .method
        .push(extra);
    assert_eq!(
        verify(&set).unwrap_err(),
        vec![Mismatch::Unregistered {
            path: "/biyard.greeter.v1.Greeter/Wave".into()
        }]
    );
}

#[test]
fn reports_request_type_drift() {
    let mut set = descriptor_set();
    method_mut(&mut set, "biyard.greeter.v2", "SayHello").input_type =
        Some(".biyard.greeter.v1.HelloRequest".into());
    assert_eq!(
        verify(&set).unwrap_err(),
        vec![Mismatch::RequestType {
            path: "/biyard.greeter.v2.Greeter/SayHello".into(),
            proto: "biyard.greeter.v1.HelloRequest".into(),
            rust: "biyard.greeter.v2.HelloRequest".into(),
        }]
    );
}

#[test]
fn reports_streaming_kind_drift() {
    let mut set = descriptor_set();
    method_mut(&mut set, "biyard.greeter.v1", "Countdown").server_streaming = Some(false);
    method_mut(&mut set, "biyard.greeter.v1", "Chat").client_streaming = Some(false);
    assert_eq!(
        verify(&set).unwrap_err(),
        vec![
            Mismatch::Kind {
                path: "/biyard.greeter.v1.Greeter/Countdown".into(),
                proto: MethodKind::Unary,
                rust: MethodKind::ServerStreaming,
            },
            Mismatch::Kind {
                path: "/biyard.greeter.v1.Greeter/Chat".into(),
                proto: MethodKind::ServerStreaming,
                rust: MethodKind::Bidi,
            },
        ]
    );
}

#[test]
fn reports_handlers_missing_from_the_proto() {
    let mut set = descriptor_set();
    set.file.retain(|f| f.package() != "biyard.greeter.v2");
    assert_eq!(
        verify(&set).unwrap_err(),
        vec![Mismatch::UnknownMethod {
            path: "/biyard.greeter.v2.Greeter/SayHello".into(),
            source: "example_grpc_server::methods::v2::say_hello::say_hello_v2",
        }]
    );
}
