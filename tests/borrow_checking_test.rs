use gaiarusted::borrowchecker::BorrowChecker;

#[test]
fn test_borrow_checker_initialization() {
    let checker = BorrowChecker::new();
    // If this compiles and runs, BorrowChecker initializes correctly
    let _ = checker;
}

#[test]
fn test_binding_creation() {
    let mut checker = BorrowChecker::new();
    let result = checker.check_items(&[]);
    assert!(result.is_ok());
}

#[test]
fn test_ownership_state_types() {
    use gaiarusted::borrowchecker::OwnershipState;
    let owned = OwnershipState::Owned;
    let moved = OwnershipState::Moved;
    let _ = (owned, moved);
}