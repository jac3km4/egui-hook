use std::mem::transmute;

pub use egui_d3d11::DirectX11App;
use kiero4rs::ffi::{RenderType, Status};
use kiero4rs::methods::D3D11;
pub use windows::core::HRESULT;
pub use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, PSTR, WPARAM};
pub use windows::Win32::Graphics::Dxgi::IDXGISwapChain;
pub use windows::Win32::System::LibraryLoader::GetModuleHandleA;
pub use windows::Win32::UI::WindowsAndMessaging::{CallWindowProcW, SetWindowLongPtrA, GWLP_WNDPROC, WNDPROC};
pub use {log, startup};

pub type PresentFn = unsafe extern "stdcall" fn(IDXGISwapChain, u32, u32) -> HRESULT;

#[macro_export]
macro_rules! egui_hook {
    ($app:ty, $ui:expr) => {
        static mut APP: Option<$crate::DirectX11App<$app>> = None;

        static mut OLD_WNDPROC: Option<$crate::WNDPROC> = None;
        static mut OLD_PRESENT: $crate::PresentFn = hook_present;

        $crate::startup::on_startup! {
            std::thread::spawn(|| unsafe { $crate::init(hook_present, &mut OLD_PRESENT) });
        }

        unsafe extern "stdcall" fn hook_present(
            swap_chain: $crate::IDXGISwapChain,
            sync_interval: u32,
            flags: u32,
        ) -> $crate::HRESULT {
            if APP.is_none() {
                APP = Some($crate::DirectX11App::new_with_default($ui, &swap_chain));

                let desc = swap_chain.GetDesc().unwrap();
                if desc.OutputWindow.is_invalid() {
                    $crate::log::error!("Invalid window handle.");
                }

                let proc = $crate::SetWindowLongPtrA(
                    desc.OutputWindow,
                    $crate::GWLP_WNDPROC,
                    hookk_wnd_proc as usize as _,
                );
                OLD_WNDPROC = Some(std::mem::transmute(proc));
            }

            APP.as_ref().unwrap().present(&swap_chain, sync_interval, flags);

            OLD_PRESENT(swap_chain, sync_interval, flags)
        }

        unsafe extern "stdcall" fn hookk_wnd_proc(
            hwnd: $crate::HWND,
            msg: u32,
            wparam: $crate::WPARAM,
            lparam: $crate::LPARAM,
        ) -> $crate::LRESULT {
            let app = APP.as_ref().unwrap();

            APP.as_ref().unwrap().wnd_proc(msg, wparam, lparam);
            if app.state().is_active() {
                $crate::LRESULT(0)
            } else {
                $crate::CallWindowProcW(OLD_WNDPROC.unwrap(), hwnd, msg, wparam, lparam)
            }
        }
    };
}

pub fn init(present: PresentFn, original: &mut PresentFn) {
    let res = match kiero4rs::init(RenderType::D3D11) {
        Status::Success => kiero4rs::bind(D3D11::Present, unsafe { transmute(original) }, present as *mut _),
        other => other,
    };
    match res {
        Status::Success => {
            log::info!("Successfully bound the egui present function")
        }
        other => {
            log::error!("Failed to bind the present function: {:?}", other);
        }
    }
}

#[macro_export]
macro_rules! import_foreign {
    ($addr:expr, $name:ident() -> $ty:ty) => {
        fn $name() -> $ty {
            let module = unsafe { $crate::GetModuleHandleA($crate::PSTR(std::ptr::null())) };
            let ptr = module.0 + $addr;
            let func: extern "C" fn() -> $ty = unsafe { std::mem::transmute(ptr) };
            func()
        }
    };
}
