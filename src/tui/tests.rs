use super::App;

#[test]
fn test_app_creation() {
    let app = App::new();
    assert!(app.is_ok());

    let app = app.unwrap();
    assert_eq!(app.should_quit, false);
}

#[test]
fn test_app_state() {
    let mut app = App::new().unwrap();

    // Initially should not be quitting
    assert_eq!(app.should_quit, false);

    // Simulate quit
    app.should_quit = true;
    assert_eq!(app.should_quit, true);
}