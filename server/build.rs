//! build script to generate .rs from .proto

///generates .rs files in src directory
fn main() -> Result<(), Box<dyn std::error::Error>> {
    // TODO Also import common interface files

    // Build the Client
    tonic_build::configure()
        .build_server(false)
        .out_dir("../client/src/")
        .type_attribute("QueryIsReady", "#[derive(Eq)]")
        .type_attribute("ReadyResponse", "#[derive(Eq)]")
        .compile(&["../proto/grpc.proto"], &["../proto"])?;

    // Build the Server
    tonic_build::configure()
        .build_client(false)
        .out_dir("src/")
        .compile(&["../proto/grpc.proto"], &["../proto"])?;

    Ok(())
}
