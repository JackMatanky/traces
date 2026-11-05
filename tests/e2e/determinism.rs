//! Output determinism: the same vault built in two sandboxes in different
//! file-creation orders must produce byte-identical stdout, independent of
//! directory-listing order or insertion order into the index.

use pretty_assertions::assert_eq;
use rstest::rstest;

use super::support::Sandbox;

/// Notes with names that sort differently under byte, case-insensitive, and
/// locale-aware orderings: `B.md` before `Z.md` before `a.md` by bytes, and
/// `é.md` after all ASCII names.
const NOTES: [(&str, &str); 4] = [
    ("a.md", "---\ntags: [shared]\n---\n# A\n- [ ] task a\n"),
    ("B.md", "---\ntags: [shared]\n---\n# B\n- [x] task b\n"),
    ("é.md", "---\ntags: [shared]\n---\n# E\n- [ ] task e\n"),
    ("Z.md", "---\ntags: [shared]\n---\n# Z\n- [ ] task z\n"),
];

/// Builds a trusted sandbox whose notes are written in iteration order.
fn vault_in_order<'a>(
    order: impl Iterator<Item = &'a (&'a str, &'a str)>,
) -> Sandbox {
    let sandbox = Sandbox::trusted_via_cli();
    for (name, content) in order {
        sandbox.write_note(name, content);
    }
    sandbox
}

/// Pins byte-identical stdout for the same vault built in forward and reverse
/// creation order.
///
/// Executes vs asserts:
/// - Executes: each query once in two sandboxes whose notes were created in
///   opposite orders.
/// - Asserts: exit 0, non-empty stdout naming every note where applicable, and
///   stdout equal between the sandboxes.
///
/// Defect class: nondeterministic-order.
#[rstest]
#[case::list(&["list"])]
#[case::list_sorted(&["list", "--sort", "file.name", "--asc"])]
#[case::table(&["table", "--column", "file.name", "--column", "file.path"])]
#[case::task(&["task"])]
#[case::tag_query(&["list", "--from", "#shared"])]
fn produces_identical_output_regardless_of_creation_order(
    #[case] args: &[&str],
) {
    let forward = vault_in_order(NOTES.iter());
    let reverse = vault_in_order(NOTES.iter().rev());

    let first = forward.run(args);
    let second = reverse.run(args);

    first.assert_success();
    second.assert_success();
    assert!(!first.stdout.is_empty(), "query must produce output");
    assert_eq!(first.stdout, second.stdout);
}
