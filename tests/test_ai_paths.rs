use easycopy::config::dirs::Directories;
use easycopy::store::paths;

#[test]
fn chat_paths_under_data_dir() {
    let dirs = Directories::discover();
    assert_eq!(paths::chat_db(&dirs), dirs.data_dir.join("chat.db"));
    assert_eq!(paths::chat_state(&dirs), dirs.data_dir.join("chat_state.json"));
}
