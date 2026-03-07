use std::fs;

fn main() {
    let version = env!("CARGO_PKG_VERSION");

    let template = fs::read_to_string("assets/sw.template.js")
        .expect("Should have been able to read the template file");

    let sw_content = template.replace("{{VERSION}}", version);

    fs::write("assets/sw.js", sw_content).expect("Unable to write sw.js");
}
