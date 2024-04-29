mod entry_module;
pub use entry_module::entry_module::Entry;

mod websocket_entry;
pub use websocket_entry::websocket_entry::WebsocketEntry;

mod pipeline_module;
pub use pipeline_module::{
    pipeline::BoxedClone, pipeline::Pipeline, pipeline::PipelineDirection, pipeline::PipelineStep,
};

mod websocket_step;
pub use websocket_step::{ws_destination::WebsocketDestination, wss_destination::WssDestination};

mod stdio_step;
pub use stdio_step::io_step::STDioStep;

mod base64_step;
pub use base64_step::base64::Base64;

mod macros {
    // use os_info;

    // #[macro_export]
    // macro_rules! os_version_check {
    //     ($os:expr, $version:expr, $code:block) => {
    //         #[cfg(all(target_os = $os, target_version = $version))]
    //         {
    //             $code
    //         }
    //     };
    // }

    // #[macro_export]
    // macro_rules! conditional_os_check {
    //     ($($os:ident $( ($version:expr) )? )|+ => $code:block) => {
    //     $(
    //         let info = os_info::get();
    //         // if $os == info.os_type().to_string() {
    //             // $code;
    //         // }
    //         let name = stringify!($os);
    //         #[cfg(all(target_os = name $(, target_version = $version)? ))]
    //     )+
    // };
    // }

    // #[macro_export]
    // macro_rules! conditional_os_check {
    // ($($os:ident $( ($version:expr) )? )|+ => $code:block) => {
    //     $(
    //         #[cfg(all(target_os = stringify!($os) $(, target_version = $version )? ))]
    //         $code
    //     )+
    // };
    // }

    // #[macro_export]
    // macro_rules! conditional_os_check {
    // ($($os:ident $(($version:expr))?),* => $code:block) => {
    //     $(
    //         #[cfg(all(target_os = stringify!($os) $(, target_version = $version)?))]
    //         $code
    //     )*
    // };
    // }
}
pub use macros::*;
