// SPDX-FileCopyrightText: Contributors to the Continue project
// SPDX-License-Identifier: Apache-2.0

use hkdf::Hkdf;
use sha2::Sha256;

const SAS_INFO_LABEL: &[u8] = b"CONTINUE_SAS_WORDS_V1";

/// 256 phonetically distinct words for visual Short Authentication String (SAS) verification.
pub const WORDLIST: [&str; 256] = [
    "acorn",
    "action",
    "anchor",
    "anthem",
    "apron",
    "archer",
    "arrow",
    "artist",
    "atlas",
    "atom",
    "author",
    "badge",
    "baker",
    "bamboo",
    "banner",
    "bark",
    "beacon",
    "beetle",
    "bell",
    "berry",
    "bison",
    "blade",
    "blanket",
    "blaze",
    "blossom",
    "bonfire",
    "book",
    "bottle",
    "boulder",
    "breeze",
    "brick",
    "bridge",
    "brook",
    "bucket",
    "cabin",
    "cactus",
    "canyon",
    "canvas",
    "cargo",
    "castle",
    "cedar",
    "celery",
    "chalk",
    "charm",
    "cherry",
    "chest",
    "chimney",
    "cider",
    "circle",
    "cliff",
    "cloak",
    "clover",
    "cobalt",
    "comet",
    "compass",
    "copper",
    "coral",
    "corner",
    "cotton",
    "crater",
    "creek",
    "cricket",
    "crystal",
    "curtain",
    "daisy",
    "dancer",
    "delta",
    "desert",
    "diamond",
    "dolphin",
    "dragon",
    "drift",
    "drum",
    "eagle",
    "earth",
    "echo",
    "ember",
    "engine",
    "falcon",
    "feather",
    "fern",
    "ferry",
    "field",
    "filter",
    "finch",
    "finger",
    "fire",
    "flame",
    "flint",
    "flower",
    "forest",
    "fountain",
    "fox",
    "frost",
    "garden",
    "garlic",
    "gateway",
    "gecko",
    "geyser",
    "ginger",
    "glacier",
    "globe",
    "glow",
    "goblet",
    "granite",
    "grape",
    "grass",
    "gravel",
    "grove",
    "guitar",
    "harbor",
    "harvest",
    "haven",
    "hawk",
    "hazel",
    "helmet",
    "hero",
    "heron",
    "honey",
    "horizon",
    "humming",
    "hunter",
    "husky",
    "igloo",
    "island",
    "jacket",
    "jaguar",
    "jasper",
    "jelly",
    "jigsaw",
    "jungle",
    "jupiter",
    "kettle",
    "key",
    "kite",
    "kiwi",
    "knight",
    "ladder",
    "lagoon",
    "lantern",
    "lark",
    "laurel",
    "lava",
    "leaf",
    "lemon",
    "leopard",
    "lighthouse",
    "lily",
    "lion",
    "lizard",
    "llama",
    "lobster",
    "lotus",
    "lumber",
    "lunar",
    "magnet",
    "magpie",
    "mallet",
    "mango",
    "mantle",
    "maple",
    "marble",
    "marsh",
    "meadow",
    "melon",
    "meteor",
    "mirror",
    "monarch",
    "moon",
    "moss",
    "mountain",
    "muffin",
    "nebula",
    "nectar",
    "needle",
    "nest",
    "nickel",
    "night",
    "ninja",
    "north",
    "novel",
    "oasis",
    "ocean",
    "olive",
    "onyx",
    "opal",
    "orange",
    "orbit",
    "orchid",
    "otter",
    "owl",
    "paddle",
    "palace",
    "panther",
    "papaya",
    "parrot",
    "path",
    "peach",
    "peacock",
    "pebble",
    "pelican",
    "pepper",
    "petal",
    "phoenix",
    "pillow",
    "pilot",
    "pine",
    "planet",
    "plasma",
    "plaza",
    "polar",
    "poplar",
    "prism",
    "puzzle",
    "quartz",
    "quiver",
    "rabbit",
    "radar",
    "rainbow",
    "raven",
    "reef",
    "relay",
    "ridge",
    "river",
    "robin",
    "rocket",
    "ruby",
    "saddle",
    "safari",
    "sail",
    "salmon",
    "sand",
    "sapphire",
    "saturn",
    "scale",
    "scarlet",
    "scout",
    "shadow",
    "shield",
    "sierra",
    "silver",
    "skater",
    "sky",
    "solar",
    "sparrow",
    "sphere",
    "spider",
    "spire",
    "spring",
    "spruce",
    "star",
    "summit",
    "sun",
    "surf",
    "swan",
    "timber",
];

/// Derives 4 visual verification words from the pairing shared secret and transcript.
///
/// Both peers calculate these words independently. Comparing them out-of-band allows
/// users to visually confirm that no machine-in-the-middle has intercepted or altered
/// the pairing exchange.
pub fn derive_verification_words(shared_secret: &[u8], transcript: &[u8]) -> [&'static str; 4] {
    let hk = Hkdf::<Sha256>::new(Some(transcript), shared_secret);
    let mut indices = [0u8; 4];
    hk.expand(SAS_INFO_LABEL, &mut indices)
        .expect("4 bytes is within valid HKDF expansion limit");

    [
        WORDLIST[indices[0] as usize],
        WORDLIST[indices[1] as usize],
        WORDLIST[indices[2] as usize],
        WORDLIST[indices[3] as usize],
    ]
}

/// Formats 4 verification words as a hyphen-delimited string (e.g. "acorn-blade-copper-eagle").
pub fn format_verification_phrase(words: &[&str; 4]) -> String {
    words.join("-")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn wordlist_has_256_unique_entries() {
        assert_eq!(WORDLIST.len(), 256);
        let mut seen = HashSet::new();
        for &word in &WORDLIST {
            assert!(!word.is_empty(), "word cannot be empty");
            assert!(
                seen.insert(word),
                "wordlist contains duplicate entry: {word}"
            );
        }
    }

    #[test]
    fn deterministic_derivation() {
        let secret = [42u8; 32];
        let transcript = b"sample_pairing_transcript_initiator_responder_v1";

        let words1 = derive_verification_words(&secret, transcript);
        let words2 = derive_verification_words(&secret, transcript);

        assert_eq!(words1, words2);
        assert_eq!(
            format_verification_phrase(&words1),
            format_verification_phrase(&words2)
        );
    }

    #[test]
    fn transcript_variation_alters_words() {
        let secret = [7u8; 32];
        let transcript1 = b"transcript_session_alpha";
        let transcript2 = b"transcript_session_beta";

        let words1 = derive_verification_words(&secret, transcript1);
        let words2 = derive_verification_words(&secret, transcript2);

        assert_ne!(words1, words2);
    }

    #[test]
    fn secret_variation_alters_words() {
        let secret1 = [1u8; 32];
        let secret2 = [2u8; 32];
        let transcript = b"identical_transcript";

        let words1 = derive_verification_words(&secret1, transcript);
        let words2 = derive_verification_words(&secret2, transcript);

        assert_ne!(words1, words2);
    }

    #[test]
    fn phrase_formatting() {
        let words = ["anchor", "beacon", "falcon", "planet"];
        assert_eq!(
            format_verification_phrase(&words),
            "anchor-beacon-falcon-planet"
        );
    }
}
