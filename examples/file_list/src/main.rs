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
    // Check for environment variable to determine view mode
    let view_mode = std::env::var("NPTK_FILE_VIEW_MODE")
        .unwrap_or_default()
        .to_lowercase();
    
    if view_mode == "icon" {
        println!("Running File List in Grid Icons mode");
        FileListGridIconsApp.run(());
    } else {
        println!("Running File List in List mode");
        FileListApp.run(());
    }
    // FileListApp.run(());
    // FileListGridIconsApp.run(());
}
