use std::path::{Path, PathBuf};
use std::sync::{Arc, LazyLock, RwLock};

use gpui::{App, Global, RenderImage, SharedString, SvgRenderer};
use image::{Frame, RgbaImage};
use npio::file::local::LocalFile;
use npio::service::icon::{CachedIcon, IconError, IconRegistry};
use npio::service::thumbnail::{ThumbnailImage, ThumbnailService};
use npio::ThumbnailSize;
use smallvec::SmallVec;

/// Default pixel size for file-type icons in lists and tabs.
pub const DEFAULT_ICON_SIZE: u32 = 16;

/// How a resolved XDG icon can be shown in the UI.
#[derive(Clone, Debug)]
pub enum FileIconPresentation {
    /// Raster image on disk (PNG, etc.).
    RasterPath(SharedString),
    /// Absolute path to an SVG file on disk (use with `svg().external_path`, not inline XML).
    SvgPath(SharedString),
    /// Decoded raster pixels (PNG/XPM in memory, or rasterized SVG).
    RenderImage(Arc<RenderImage>),
}

struct FileIconServiceInner {
    registry: Arc<IconRegistry>,
    thumbnail_service: Arc<ThumbnailService>,
}

/// Global XDG icon theme service (freedesktop icon themes via npio).
#[derive(Clone)]
pub struct FileIconService {
    inner: Arc<RwLock<FileIconServiceInner>>,
}

impl Global for FileIconService {}

impl FileIconService {
    /// Install the global icon service using the given freedesktop icon theme name.
    pub fn init(cx: &mut App, theme_name: impl Into<String>) {
        let service = match Self::new(theme_name.into()) {
            Ok(service) => service,
            Err(error) => {
                log::error!("failed to initialize file icon theme: {error}");
                return;
            }
        };
        cx.set_global(service);
    }

    /// Returns the global service if [`Self::init`] has been called.
    pub fn global(cx: &App) -> Option<&Self> {
        cx.try_global::<Self>()
    }

    /// Create a service for the given XDG icon theme name, with fallbacks.
    pub fn new(theme_name: String) -> Result<Self, IconError> {
        let registry = open_registry(&theme_name)?;
        Ok(Self {
            inner: Arc::new(RwLock::new(FileIconServiceInner {
                registry: Arc::new(registry),
                thumbnail_service: Arc::new(ThumbnailService::new()),
            })),
        })
    }

    /// Switch to another XDG icon theme and clear the icon cache.
    pub fn set_theme(&self, theme_name: String) -> Result<(), IconError> {
        let mut inner = self.inner.write().expect("file icon service lock");
        inner.registry = Arc::new(open_registry(&theme_name)?);
        Ok(())
    }

    /// Current XDG icon theme name.
    pub fn theme_name(&self) -> String {
        self.inner
            .read()
            .expect("file icon service lock")
            .registry
            .theme()
            .to_string()
    }

    /// Resolve an icon for a filesystem path (blocking).
    pub fn presentation_for_path(
        &self,
        path: &Path,
        size: u32,
    ) -> Option<FileIconPresentation> {
        let cached = if path.is_dir() {
            self.inner
                .read()
                .expect("file icon service lock")
                .registry
                .get_icon("folder", size)?
        } else {
            let file = LocalFile::new(path.to_path_buf());
            smol::block_on(async {
                self.resolve_file_icon(&file, size, false).await
            })?
        };
        presentation_from_cached(&cached)
    }

    /// Resolve an icon for a path suffix or file name (e.g. `rs` or `file.rs`).
    pub fn presentation_for_file_name(
        &self,
        file_name: &Path,
        size: u32,
    ) -> Option<FileIconPresentation> {
        let path = file_name_for_icon_lookup(file_name);
        self.presentation_for_path(&path, size)
    }

    /// Resolve a freedesktop theme icon by name (e.g. `edit-copy`, `folder`).
    pub fn presentation_for_icon_name(
        &self,
        icon_name: &str,
        size: u32,
    ) -> Option<FileIconPresentation> {
        let cached = self
            .inner
            .read()
            .expect("file icon service lock")
            .registry
            .get_icon(icon_name, size)?;
        presentation_from_cached(&cached)
    }

    /// Resolve a freedesktop theme icon on the async runtime (safe from GPUI `Tokio::spawn`).
    pub async fn resolve_theme_icon(
        &self,
        icon_name: &str,
        size: u32,
    ) -> Option<FileIconPresentation> {
        let registry = self.inner.read().expect("file icon service lock").registry.clone();
        let cached = registry.get_icon_async(icon_name, size).await?;
        presentation_from_cached(&cached)
    }

    /// Resolve a path icon on the async runtime (safe from GPUI `Tokio::spawn`).
    pub async fn resolve_path_icon(&self, path: &Path, size: u32) -> Option<FileIconPresentation> {
        if path.is_dir() {
            return self.resolve_theme_icon("folder", size).await;
        }
        let cached = self.resolve_icon(path, size, false).await?;
        presentation_from_cached(&cached)
    }

    /// Async resolution with thumbnail support for file managers.
    pub async fn resolve_icon(
        &self,
        path: &Path,
        size: u32,
        is_directory: bool,
    ) -> Option<CachedIcon> {
        if is_directory {
            let registry = self.inner.read().expect("file icon service lock").registry.clone();
            return registry.get_icon("folder", size);
        }

        let file = LocalFile::new(path.to_path_buf());
        self.resolve_file_icon(&file, size, true).await
    }

    async fn resolve_file_icon(
        &self,
        file: &LocalFile,
        size: u32,
        use_thumbnails: bool,
    ) -> Option<CachedIcon> {
        let (registry, thumbnail_service) = {
            let inner = self.inner.read().expect("file icon service lock");
            (
                inner.registry.clone(),
                use_thumbnails.then(|| inner.thumbnail_service.clone()),
            )
        };

        if let Some(thumbnail_service) = thumbnail_service {
            let thumb_size = thumbnail_size_for_pixels(size);
            if thumbnail_service
                .is_supported(file, None)
                .await
                .unwrap_or(false)
            {
                if let Ok(thumbnail) = thumbnail_service
                    .get_thumbnail_image(file, thumb_size, None)
                    .await
                {
                    return Some(cached_icon_from_thumbnail(thumbnail));
                }
            }
        }

        registry.get_file_icon(file, size).await
    }
}

/// Initialize or reload the global icon service from an XDG theme name.
pub fn init_global(cx: &mut App, theme_name: String) {
    if let Some(service) = FileIconService::global(cx) {
        if let Err(error) = service.set_theme(theme_name) {
            log::error!("failed to set file icon theme: {error}");
        }
    } else {
        FileIconService::init(cx, theme_name);
    }
}

/// Backward-compatible API: raster path only (SVG and in-memory icons return `None`).
pub struct FileIcons;

impl FileIcons {
    pub fn presentation_for_path(
        path: &Path,
        size: u32,
        cx: &App,
    ) -> Option<FileIconPresentation> {
        FileIconService::global(cx)?.presentation_for_path(path, size)
    }

    pub fn get_icon(path: &Path, cx: &App) -> Option<SharedString> {
        Self::presentation_for_path(path, DEFAULT_ICON_SIZE, cx).and_then(|presentation| {
            match presentation {
                FileIconPresentation::RasterPath(path) => Some(path),
                FileIconPresentation::SvgPath(_) | FileIconPresentation::RenderImage(_) => None,
            }
        })
    }
}

/// Converts a resolved npio icon into a UI-ready presentation.
pub fn icon_presentation_from_cached(icon: &CachedIcon) -> Option<FileIconPresentation> {
    presentation_from_cached(icon)
}

pub fn render_image_from_cached(icon: &CachedIcon) -> Option<Arc<RenderImage>> {
    match icon {
        CachedIcon::Image { data, width, height } => {
            rgba_to_render_image(data.as_ref(), *width, *height)
        }
        CachedIcon::Svg(_) | CachedIcon::Path(_) => None,
    }
}

fn open_registry(theme_name: &str) -> Result<IconRegistry, IconError> {
    if let Ok(registry) = IconRegistry::with_theme(Some(theme_name.to_string())) {
        return Ok(registry);
    }

    log::warn!(
        "icon theme {theme_name:?} not found, falling back to Adwaita"
    );
    if let Ok(registry) = IconRegistry::with_theme(Some("Adwaita".to_string())) {
        return Ok(registry);
    }

    IconRegistry::new()
}

fn file_name_for_icon_lookup(file_name: &Path) -> PathBuf {
    let file_name = file_name.as_os_str().to_string_lossy();
    if file_name.contains('.') {
        PathBuf::from(file_name.as_ref())
    } else {
        PathBuf::from(format!("file.{file_name}"))
    }
}

fn presentation_from_cached(cached: &CachedIcon) -> Option<FileIconPresentation> {
    match cached {
        CachedIcon::Path(path) => {
            if path_is_svg(path) {
                Some(FileIconPresentation::SvgPath(path.display().to_string().into()))
            } else {
                Some(FileIconPresentation::RasterPath(
                    path.display().to_string().into(),
                ))
            }
        }
        CachedIcon::Svg(source) => render_svg_bytes(source.as_bytes())
            .map(FileIconPresentation::RenderImage),
        CachedIcon::Image { data, width, height } => rgba_to_render_image(data.as_ref(), *width, *height)
            .map(FileIconPresentation::RenderImage),
    }
}

fn path_is_svg(path: &Path) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| extension.eq_ignore_ascii_case("svg"))
}

fn svg_renderer() -> &'static SvgRenderer {
    static RENDERER: LazyLock<SvgRenderer> = LazyLock::new(|| SvgRenderer::new(Arc::new(())));
    &RENDERER
}

fn render_svg_bytes(bytes: &[u8]) -> Option<Arc<RenderImage>> {
    svg_renderer().render_single_frame(bytes, 1.0).ok()
}

fn cached_icon_from_thumbnail(thumbnail: ThumbnailImage) -> CachedIcon {
    CachedIcon::Image {
        data: Arc::new(thumbnail.data),
        width: thumbnail.width,
        height: thumbnail.height,
    }
}

fn thumbnail_size_for_pixels(size: u32) -> ThumbnailSize {
    match size {
        0..=128 => ThumbnailSize::Normal,
        129..=256 => ThumbnailSize::Large,
        257..=512 => ThumbnailSize::XLarge,
        _ => ThumbnailSize::XXLarge,
    }
}

fn rgba_to_render_image(data: &[u8], width: u32, height: u32) -> Option<Arc<RenderImage>> {
    let mut buffer = data.to_vec();
    for pixel in buffer.chunks_exact_mut(4) {
        pixel.swap(0, 2);
    }
    let image = RgbaImage::from_raw(width, height, buffer)?;
    let frame = Frame::new(image);
    Some(Arc::new(RenderImage::new(SmallVec::from_elem(frame, 1))))
}
