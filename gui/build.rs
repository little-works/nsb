extern crate embed_resource;

fn main() {
    let result = embed_resource::compile("nsb-gui.rc", embed_resource::NONE);
    if let embed_resource::CompilationResult::Failed(err) = result {
        panic!("failed to embed resource file: {err}");
    }
}
