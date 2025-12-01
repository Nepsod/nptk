use nptk::prelude::*;
use nptk_widgets::file_list::FileListViewMode;
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

struct FileListGridIconsApp;

impl Application for FileListGridIconsApp {
    type Theme = SystemTheme;
    type State = ();

    fn build(_: AppContext, _: Self::State) -> impl Widget {
        let current_dir = std::env::current_dir().unwrap_or(PathBuf::from("."));
        FileList::new(current_dir)
            .with_view_mode(FileListViewMode::Icon)
    }
}

#[tokio::main]
async fn main() {
    // FileListApp.run(());
    FileListGridIconsApp.run(());
}
