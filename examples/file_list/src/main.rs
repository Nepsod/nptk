use nptk::prelude::*;
use std::path::PathBuf;

struct FileListApp;

impl Application for FileListApp {
    type Theme = SystemTheme;
    type State = ();

    fn build(_: AppContext, _: Self::State) -> impl Widget {
        let current_dir = std::env::current_dir().unwrap_or(PathBuf::from("."));
        FileList::new(current_dir)
    }
}

#[tokio::main]
async fn main() {
    FileListApp.run(());
}
