#![cfg(target_os = "linux")]

//! Wayland global object binding and registry handling.

use wayland_client::globals::GlobalList;
use wayland_client::protocol::{wl_data_device_manager, wl_keyboard, wl_pointer, wl_seat, wl_shm};
use wayland_client::{Proxy, QueueHandle};
use wayland_protocols::wp::fractional_scale::v1::client::wp_fractional_scale_manager_v1;
use wayland_protocols::wp::primary_selection::zv1::client::zwp_primary_selection_device_manager_v1;
use wayland_protocols::wp::text_input::zv3::client::zwp_text_input_manager_v3;
use wayland_protocols::wp::viewporter::client::wp_viewporter;
use wayland_protocols::xdg::activation::v1::client::xdg_activation_v1;
use wayland_protocols::xdg::decoration::zv1::client::zxdg_decoration_manager_v1;
use wayland_protocols::xdg::shell::client::xdg_wm_base;
use wayland_protocols_plasma::server_decoration::client::org_kde_kwin_server_decoration_manager;

use super::shell::WaylandClientState;

#[cfg(feature = "global-menu")]
use wayland_protocols_plasma::appmenu::client::org_kde_kwin_appmenu_manager;

const COMPOSITOR_VERSION: u32 = 4;
const XDG_WM_BASE_VERSION: u32 = 6;
const ZXDG_DECORATION_VERSION: u32 = 1;
const KDE_SERVER_DECORATION_VERSION: u32 = 1;
#[cfg(feature = "global-menu")]
const KDE_APPMENU_MANAGER_VERSION: u32 = 2;
const WL_SHM_VERSION: u32 = 1;
const WL_SEAT_VERSION: u32 = 7;
const WL_DATA_DEVICE_MANAGER_VERSION: u32 = 3;
const ZWP_PRIMARY_SELECTION_DEVICE_MANAGER_V1_VERSION: u32 = 1;
const ZWP_TEXT_INPUT_MANAGER_V3_VERSION: u32 = 1;
const WP_FRACTIONAL_SCALE_MANAGER_V1_VERSION: u32 = 1;
const WP_VIEWPORTER_VERSION: u32 = 1;
const XDG_ACTIVATION_V1_VERSION: u32 = 1;

/// Wayland global objects bound from the registry.
#[derive(Clone)]
#[allow(dead_code)]
pub struct WaylandGlobals {
    pub compositor: wayland_client::protocol::wl_compositor::WlCompositor,
    pub wm_base: xdg_wm_base::XdgWmBase,
    pub decoration_manager: Option<zxdg_decoration_manager_v1::ZxdgDecorationManagerV1>,
    pub kde_server_decoration_manager:
        Option<org_kde_kwin_server_decoration_manager::OrgKdeKwinServerDecorationManager>,
    #[cfg(feature = "global-menu")]
    pub appmenu_manager: Option<org_kde_kwin_appmenu_manager::OrgKdeKwinAppmenuManager>,
    pub shm: Option<wl_shm::WlShm>,
    pub seat: Option<wl_seat::WlSeat>,
    pub pointer: Option<wl_pointer::WlPointer>,
    pub keyboard: Option<wl_keyboard::WlKeyboard>,
    pub data_device_manager: Option<wl_data_device_manager::WlDataDeviceManager>,
    pub primary_selection_manager:
        Option<zwp_primary_selection_device_manager_v1::ZwpPrimarySelectionDeviceManagerV1>,
    pub text_input_manager: Option<zwp_text_input_manager_v3::ZwpTextInputManagerV3>,
    pub fractional_scale_manager: Option<wp_fractional_scale_manager_v1::WpFractionalScaleManagerV1>,
    pub viewporter: Option<wp_viewporter::WpViewporter>,
    pub activation: Option<xdg_activation_v1::XdgActivationV1>,
}

impl WaylandGlobals {
    pub fn bind_all(
        globals: &GlobalList,
        qh: &QueueHandle<WaylandClientState>,
    ) -> Result<Self, String> {
        let compositor: wayland_client::protocol::wl_compositor::WlCompositor = globals
            .bind(qh, 1..=COMPOSITOR_VERSION, ())
            .map_err(|e| format!("Failed to bind wl_compositor: {:?}", e))?;

        let wm_base: xdg_wm_base::XdgWmBase = globals
            .bind(qh, 1..=XDG_WM_BASE_VERSION, ())
            .map_err(|e| format!("Failed to bind xdg_wm_base: {:?}", e))?;

        let decoration_manager = match globals
            .bind::<zxdg_decoration_manager_v1::ZxdgDecorationManagerV1, _, _>(
                qh,
                1..=ZXDG_DECORATION_VERSION,
                (),
            ) {
            Ok(mgr) => Some(mgr),
            Err(wayland_client::globals::BindError::NotPresent) => None,
            Err(err) => {
                return Err(format!(
                    "Failed to bind zxdg_decoration_manager_v1: {:?}",
                    err
                ));
            },
        };
        let kde_server_decoration_manager = match globals.bind::<
            org_kde_kwin_server_decoration_manager::OrgKdeKwinServerDecorationManager,
            _,
            _,
        >(qh, 1..=KDE_SERVER_DECORATION_VERSION, ()) {
            Ok(mgr) => Some(mgr),
            Err(wayland_client::globals::BindError::NotPresent) => None,
            Err(err) => {
                return Err(format!(
                    "Failed to bind org_kde_kwin_server_decoration_manager: {:?}",
                    err
                ));
            }
        };

        #[cfg(feature = "global-menu")]
        let appmenu_manager = match globals.bind::<
            org_kde_kwin_appmenu_manager::OrgKdeKwinAppmenuManager,
            _,
            _,
        >(qh, 1..=KDE_APPMENU_MANAGER_VERSION, ()) {
            Ok(mgr) => {
                log::info!("Bound to org.kde.kwin.appmenu_manager");
                Some(mgr)
            },
            Err(wayland_client::globals::BindError::NotPresent) => {
                log::debug!("org.kde.kwin.appmenu_manager not available (not on KWin?)");
                None
            },
            Err(err) => {
                log::warn!("Failed to bind org.kde.kwin.appmenu_manager: {:?}", err);
                None
            },
        };

        let shm = match globals.bind::<wl_shm::WlShm, _, _>(qh, 1..=WL_SHM_VERSION, ()) {
            Ok(s) => Some(s),
            Err(wayland_client::globals::BindError::NotPresent) => None,
            Err(err) => return Err(format!("Failed to bind wl_shm: {:?}", err)),
        };

        let seat = match globals.bind::<wl_seat::WlSeat, _, _>(qh, 1..=WL_SEAT_VERSION, ()) {
            Ok(s) => Some(s),
            Err(wayland_client::globals::BindError::NotPresent) => None,
            Err(err) => return Err(format!("Failed to bind wl_seat: {:?}", err)),
        };

        let mut pointer = None;
        let mut keyboard = None;
        if let Some(ref seat) = seat {
            pointer = Some(seat.get_pointer(qh, ()));
            keyboard = Some(seat.get_keyboard(qh, ()));
            log::info!("Wayland keyboard created: {:?}", keyboard.as_ref().map(|k| k.id()));
        } else {
            log::warn!("No Wayland seat available, keyboard input will not work");
        }

        let data_device_manager = match globals.bind::<wl_data_device_manager::WlDataDeviceManager, _, _>(
            qh,
            1..=WL_DATA_DEVICE_MANAGER_VERSION,
            (),
        ) {
            Ok(mgr) => Some(mgr),
            Err(wayland_client::globals::BindError::NotPresent) => None,
            Err(err) => return Err(format!("Failed to bind wl_data_device_manager: {:?}", err)),
        };

        let primary_selection_manager = match globals.bind::<
            zwp_primary_selection_device_manager_v1::ZwpPrimarySelectionDeviceManagerV1,
            _,
            _,
        >(qh, 1..=ZWP_PRIMARY_SELECTION_DEVICE_MANAGER_V1_VERSION, ()) {
            Ok(mgr) => Some(mgr),
            Err(wayland_client::globals::BindError::NotPresent) => None,
            Err(err) => {
                return Err(format!(
                    "Failed to bind zwp_primary_selection_device_manager_v1: {:?}",
                    err
                ));
            }
        };

        let text_input_manager = match globals.bind::<
            zwp_text_input_manager_v3::ZwpTextInputManagerV3,
            _,
            _,
        >(qh, 1..=ZWP_TEXT_INPUT_MANAGER_V3_VERSION, ()) {
            Ok(mgr) => Some(mgr),
            Err(wayland_client::globals::BindError::NotPresent) => None,
            Err(err) => {
                return Err(format!(
                    "Failed to bind zwp_text_input_manager_v3: {:?}",
                    err
                ));
            }
        };

        let fractional_scale_manager = match globals.bind::<
            wp_fractional_scale_manager_v1::WpFractionalScaleManagerV1,
            _,
            _,
        >(qh, 1..=WP_FRACTIONAL_SCALE_MANAGER_V1_VERSION, ()) {
            Ok(mgr) => Some(mgr),
            Err(wayland_client::globals::BindError::NotPresent) => None,
            Err(err) => {
                return Err(format!(
                    "Failed to bind wp_fractional_scale_manager_v1: {:?}",
                    err
                ));
            }
        };

        let viewporter = match globals.bind::<wp_viewporter::WpViewporter, _, _>(
            qh,
            1..=WP_VIEWPORTER_VERSION,
            (),
        ) {
            Ok(mgr) => Some(mgr),
            Err(wayland_client::globals::BindError::NotPresent) => None,
            Err(err) => return Err(format!("Failed to bind wp_viewporter: {:?}", err)),
        };

        let activation = match globals.bind::<xdg_activation_v1::XdgActivationV1, _, _>(
            qh,
            1..=XDG_ACTIVATION_V1_VERSION,
            (),
        ) {
            Ok(mgr) => Some(mgr),
            Err(wayland_client::globals::BindError::NotPresent) => None,
            Err(err) => return Err(format!("Failed to bind xdg_activation_v1: {:?}", err)),
        };

        Ok(Self {
            compositor,
            wm_base,
            decoration_manager,
            kde_server_decoration_manager,
            #[cfg(feature = "global-menu")]
            appmenu_manager,
            shm,
            seat,
            pointer,
            keyboard,
            data_device_manager,
            primary_selection_manager,
            text_input_manager,
            fractional_scale_manager,
            viewporter,
            activation,
        })
    }
}

