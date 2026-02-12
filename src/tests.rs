use super::*;
use proptest::prelude::*;
use rand::rngs::StdRng;
use rand::SeedableRng;

#[test]
fn generate_password_has_expected_length_and_categories() {
    let mut rng = StdRng::seed_from_u64(42);
    let password = generate_password(4, 3, 2, 5, &mut rng);

    assert_eq!(password.len(), 14);
    assert!(password.chars().any(|ch| ch.is_ascii_lowercase()));
    assert!(password.chars().any(|ch| ch.is_ascii_uppercase()));
    assert!(password.chars().any(|ch| ch.is_ascii_digit()));
    assert!(password.chars().any(|ch| SYMBOLS.contains(&(ch as u8))));
}

#[test]
fn generate_password_all_zero_is_empty() {
    let mut rng = StdRng::seed_from_u64(7);
    let password = generate_password(0, 0, 0, 0, &mut rng);
    assert!(password.is_empty());
}

#[test]
fn generate_password_only_letters_has_lowercase() {
    let mut rng = StdRng::seed_from_u64(9);
    let password = generate_password(6, 0, 0, 0, &mut rng);
    assert_eq!(password.len(), 6);
    assert!(password.chars().all(|ch| ch.is_ascii_lowercase()));
}

#[test]
fn generate_password_only_uppercase_has_uppercase() {
    let mut rng = StdRng::seed_from_u64(11);
    let password = generate_password(0, 5, 0, 0, &mut rng);
    assert_eq!(password.len(), 5);
    assert!(password.chars().all(|ch| ch.is_ascii_uppercase()));
}

#[test]
fn generate_password_only_numbers_has_digits() {
    let mut rng = StdRng::seed_from_u64(13);
    let password = generate_password(0, 0, 0, 8, &mut rng);
    assert_eq!(password.len(), 8);
    assert!(password.chars().all(|ch| ch.is_ascii_digit()));
}

#[test]
fn generate_password_only_symbols_has_symbols() {
    let mut rng = StdRng::seed_from_u64(15);
    let password = generate_password(0, 0, 6, 0, &mut rng);
    assert_eq!(password.len(), 6);
    assert!(password.chars().all(|ch| SYMBOLS.contains(&(ch as u8))));
}

#[test]
fn strength_is_strong_when_all_criteria_met() {
    let password = "Aa1!aaaaaa";
    assert_eq!(check_password_strength(password), "Strong");
}

#[test]
fn strength_is_moderate_when_three_criteria_met() {
    let password = "Aa1bbbbbbb";
    assert_eq!(check_password_strength(password), "Moderate");
}

#[test]
fn strength_is_weak_when_two_or_fewer_criteria_met() {
    let password = "Aa1bbbb";
    assert_eq!(check_password_strength(password), "Weak");
}

#[test]
fn strength_is_do_not_use_when_few_criteria_met() {
    let password = "aaaa";
    assert_eq!(check_password_strength(password), "Do not use!!!!");
}

proptest! {
    #[test]
    fn generated_password_length_matches_sum(
        letters in 0i32..20,
        uppercase in 0i32..20,
        symbols in 0i32..20,
        numbers in 0i32..20,
        seed in any::<u64>(),
    ) {
        let mut rng = StdRng::seed_from_u64(seed);
        let password = generate_password(letters, uppercase, symbols, numbers, &mut rng);
        let expected_len = (letters + uppercase + symbols + numbers) as usize;
        prop_assert_eq!(password.len(), expected_len);
    }

    #[test]
    fn generated_password_only_has_allowed_chars(
        letters in 0i32..20,
        uppercase in 0i32..20,
        symbols in 0i32..20,
        numbers in 0i32..20,
        seed in any::<u64>(),
    ) {
        let mut rng = StdRng::seed_from_u64(seed);
        let password = generate_password(letters, uppercase, symbols, numbers, &mut rng);
        for ch in password.chars() {
            let is_lower = ch.is_ascii_lowercase();
            let is_upper = ch.is_ascii_uppercase();
            let is_digit = ch.is_ascii_digit();
            let is_symbol = SYMBOLS.contains(&(ch as u8));
            prop_assert!(is_lower || is_upper || is_digit || is_symbol);
        }
    }
}
