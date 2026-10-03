// ─── why ────────────────────────────────────────────────────────
// FNV-1a, twelve lines, because what trains hashes — the `dedupe_key` — has to
// be STABLE ACROSS VERSIONS and std's hasher explicitly is not: it is randomly
// seeded per process, so a key written today would not match the one computed
// for the same event tomorrow.
//
// `bytes` is the same function over a whole file, and it is a Dokument's
// IDENTITY: the same bytes dropped twice are one document. A collision would
// show a new file as „bereits vorhanden“ — visible, never a silent merge.
//
// It is not a security boundary. A collision would merge two events, and that
// is visible in the preview before anything is written, which is why 64 bits is
// plenty and nobody should harden this into something undebuggable.
// ────────────────────────────────────────────────────────────────

const OFFSET: u64 = 0xcbf2_9ce4_8422_2325;
const PRIME: u64 = 0x0100_0000_01b3;

pub fn fnv1a(input: &str) -> String {
    bytes(input.as_bytes())
}

pub fn bytes(input: &[u8]) -> String {
    let mut hash = OFFSET;
    for byte in input {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(PRIME);
    }
    format!("{hash:016x}")
}

pub fn join(parts: &[&str]) -> String {
    fnv1a(&parts.join("\u{1f}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_same_input_always_hashes_the_same_way() {
        assert_eq!(fnv1a("abc"), fnv1a("abc"));
        assert_eq!(fnv1a(""), fnv1a(""));
    }

    #[test]
    fn different_inputs_hash_differently() {
        assert_ne!(fnv1a("abc"), fnv1a("abd"));
        assert_ne!(fnv1a(""), fnv1a("a"));
    }

    /// The separator is what stops `["ab", "c"]` and `["a", "bc"]` colliding —
    /// which for a dedupe key would silently merge two different events.
    #[test]
    fn the_separator_keeps_the_parts_apart() {
        assert_ne!(join(&["ab", "c"]), join(&["a", "bc"]));
    }

    #[test]
    fn a_file_hashes_like_its_text() {
        assert_eq!(bytes(b"abc"), fnv1a("abc"));
        assert_ne!(bytes(b"abc"), bytes(b"abc\n"));
    }

    #[test]
    fn a_hash_is_a_fixed_width_hex_string() {
        assert_eq!(fnv1a("anything").len(), 16);
        assert!(fnv1a("anything").chars().all(|c| c.is_ascii_hexdigit()));
    }
}
