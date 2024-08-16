use supports_hyperlinks::supports_hyperlinks;

pub mod pack;
pub mod index;
pub mod versions;
pub mod modrinth;
pub mod files;

// using https://crates.io/crates/supports-hyperlinks
// to test if hyperlinks in terminal are supported and use a link if they are
pub fn to_hyperlink(link: &str, placeholder: &str) -> String {
    if supports_hyperlinks() {
        format!("\u{1b}]8;;{}\u{1b}\\{}\u{1b}]8;;\u{1b}\\", link, placeholder)
    } else {
        placeholder.into()
    }
}