//! Feeds arbitrary Markdown, including frontmatter, to the note parser. It
//! must not panic and must keep the note path.
#![no_main]

use std::path::Path;

use libfuzzer_sys::fuzz_target;
use traces::{MarkdownParserInput, parse_markdown};

fuzz_target!(|input: &str| {
    let path = Path::new("fuzz.md");
    let note = parse_markdown(&MarkdownParserInput::for_test(path, input));
    assert_eq!(note.path(), path);
});
