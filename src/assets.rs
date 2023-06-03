use rust_embed::RustEmbed;

#[derive(RustEmbed)]
#[folder = "res/"]
#[prefix = "res/"]
pub struct Res;
