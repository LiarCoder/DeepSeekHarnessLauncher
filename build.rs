fn main() {
    let _ = embed_resource::compile("resources/app.rc", embed_resource::NONE);
    println!("cargo:rerun-if-changed=resources/app.rc");
    println!("cargo:rerun-if-changed=assets/dsh-logo.ico");
}
