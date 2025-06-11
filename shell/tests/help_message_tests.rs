use shell::help_message;

#[test]
fn test_help_message_content() {
    let message = help_message();

    // Check that the message contains all the expected commands
    assert!(
        message.contains(".help"),
        "Help message should mention .help command"
    );
    assert!(
        message.contains(".mode"),
        "Help message should mention .mode command"
    );
    assert!(
        message.contains(".list"),
        "Help message should mention .list command"
    );
    assert!(
        message.contains(".delete"),
        "Help message should mention .delete command"
    );
    assert!(
        message.contains(".del"),
        "Help message should mention .del command"
    );
    assert!(
        message.contains(".reset"),
        "Help message should mention .reset command"
    );
    assert!(
        message.contains("Ctrl+C"),
        "Help message should mention Ctrl+C"
    );
    assert!(
        message.contains(".quit"),
        "Help message should mention .quit command"
    );

    // Check that the message contains descriptions for each command
    assert!(
        message.contains("Show this help message"),
        "Help message should describe .help command"
    );
    assert!(
        message.contains("Toggle between multiline and single line modes"),
        "Help message should describe .mode command"
    );
    assert!(
        message.contains("List all edited lines"),
        "Help message should describe .list command"
    );
    assert!(
        message.contains("Remove the last edited line"),
        "Help message should describe .delete command"
    );
    assert!(
        message.contains("Interrupt current input"),
        "Help message should describe .reset command"
    );
    assert!(
        message.contains("Exit the shell"),
        "Help message should describe .quit command"
    );
}
