// SPDX-FileCopyrightText: Contributors to the Continue project
// SPDX-License-Identifier: Apache-2.0

//! Four words both devices show for a pairing, made from both identities, so people can check
//! they paired with the device they meant to without scanning anything.

use sha2::{Digest, Sha256};

const DOMAIN: &[u8] = b"PAIRING_WORDS_V1";

/// The same four words on both devices, whichever one asks.
pub fn pairing_words(one: &str, other: &str) -> [&'static str; 4] {
    let (first, second) = if one <= other {
        (one, other)
    } else {
        (other, one)
    };
    // Both identities are public, so a hash is enough; there's no secret to derive a key from.
    let hash = Sha256::new()
        .chain_update(DOMAIN)
        .chain_update(first)
        .chain_update(b"\0")
        .chain_update(second)
        .finalize();
    std::array::from_fn(|i| WORDS[usize::from(hash[i])])
}

/// How long a presence token lasts before the phone moves to the next one.
pub const PRESENCE_WINDOW_SECS: u64 = 600;

/// What a phone broadcasts over Bluetooth so its paired computers can tell it's nearby. Made
/// from its fingerprint, which only paired devices know, and the time window, so it changes and
/// can't be used to follow the phone around.
pub fn presence_token(phone: &str, unix_secs: u64) -> [u8; 8] {
    let window = (unix_secs / PRESENCE_WINDOW_SECS).to_le_bytes();
    let key = crypto::hkdf::derive_key(phone.as_bytes(), &window, b"PRESENCE_V1")
        .expect("32 bytes is always valid");
    std::array::from_fn(|i| key[i])
}

/// Short, common and hard to mix up when read aloud.
const WORDS: [&str; 256] = [
    "acorn", "actor", "adult", "agent", "alarm", "album", "alley", "amber", "angle", "ankle",
    "apple", "apron", "arena", "arrow", "atlas", "attic", "award", "bacon", "badge", "bagel",
    "baker", "banjo", "barn", "basin", "beach", "beard", "bench", "berry", "bison", "blade",
    "bloom", "board", "bonus", "boot", "bottle", "bread", "brick", "bridge", "brush", "bucket",
    "bugle", "cabin", "cable", "cactus", "camel", "candle", "canoe", "canvas", "carpet", "castle",
    "cedar", "chalk", "cherry", "chess", "cider", "circle", "clock", "cloud", "clover", "coast",
    "cobra", "comet", "copper", "coral", "cotton", "cradle", "crane", "crayon", "cricket", "crown",
    "cube", "dairy", "daisy", "delta", "desert", "diamond", "dinner", "dolphin", "donkey",
    "dragon", "drum", "eagle", "easel", "echo", "elbow", "ember", "engine", "fabric", "falcon",
    "feather", "fence", "ferry", "fiddle", "field", "flame", "flute", "forest", "fossil",
    "fountain", "frog", "galaxy", "garden", "garlic", "gecko", "giant", "ginger", "glacier",
    "globe", "glove", "goat", "goose", "grape", "gravel", "guitar", "hammer", "harbor", "harp",
    "hazel", "helmet", "heron", "hill", "honey", "horizon", "hotel", "igloo", "island", "ivory",
    "jacket", "jaguar", "jelly", "jewel", "jungle", "kayak", "kettle", "kitten", "kiwi", "ladder",
    "lagoon", "lantern", "lemon", "lens", "lily", "lion", "lizard", "lobster", "locket", "lotus",
    "magnet", "mango", "maple", "marble", "meadow", "melon", "mirror", "mitten", "monkey", "moose",
    "mosaic", "mountain", "muffin", "napkin", "nutmeg", "needle", "nest", "noodle", "oasis",
    "ocean", "olive", "onion", "orange", "orbit", "orchid", "otter", "owl", "oyster", "paddle",
    "palace", "panda", "paper", "parrot", "peach", "pearl", "pebble", "pepper", "piano", "pickle",
    "pigeon", "pillow", "pirate", "planet", "plum", "pocket", "pony", "potato", "pumpkin",
    "puzzle", "quilt", "rabbit", "radar", "rainbow", "raven", "ribbon", "river", "robin", "rocket",
    "saddle", "salmon", "sandal", "saturn", "scarf", "shadow", "shell", "silver", "sketch",
    "sleigh", "spider", "spoon", "squid", "stamp", "statue", "stone", "summit", "sunset", "swan",
    "table", "tablet", "teapot", "temple", "thunder", "tiger", "timber", "toast", "tomato",
    "torch", "tower", "tractor", "trumpet", "tulip", "tunnel", "turtle", "valley", "velvet",
    "violin", "volcano", "wagon", "walnut", "whale", "wheat", "willow", "window", "winter",
    "wizard", "yacht", "yogurt", "zebra", "zipper",
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn both_devices_show_the_same_words_and_other_pairs_differ() {
        let phone = "2KNW2LQ4-phone";
        let computer = "7XKQ9ZPA-computer";
        assert_eq!(
            pairing_words(phone, computer),
            pairing_words(computer, phone)
        );
        assert_ne!(
            pairing_words(phone, computer),
            pairing_words(phone, "someone-else")
        );
    }

    #[test]
    fn every_word_is_distinct() {
        let mut words = WORDS.to_vec();
        words.sort_unstable();
        words.dedup();
        assert_eq!(words.len(), WORDS.len());
    }

    #[test]
    fn a_presence_token_changes_with_the_window_and_the_phone() {
        let now = 1_700_000_000;
        assert_eq!(
            presence_token("phone", now),
            presence_token("phone", now + 1)
        );
        assert_ne!(
            presence_token("phone", now),
            presence_token("phone", now + PRESENCE_WINDOW_SECS)
        );
        assert_ne!(presence_token("phone", now), presence_token("other", now));
    }
}
