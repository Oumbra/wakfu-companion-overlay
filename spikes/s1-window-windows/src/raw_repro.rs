//! Repro minimal, indépendant de `wgpu`/`wgpu-hal` — spike S1 (docs/plan-architecture.md §12).
//!
//! Après deux corrections confirmées côté wgpu-hal (SwapEffect=FLIP_SEQUENTIAL requis par
//! `CreateSwapChainForComposition`, ALLOW_TEARING/FRAME_LATENCY_WAITABLE_OBJECT exclus), la
//! création de swapchain composition échoue TOUJOURS avec DXGI_ERROR_INVALID_CALL, avec un
//! descripteur qui respecte pourtant chaque exigence documentée par Microsoft — et sans que la
//! couche de debug D3D12/DXGI (indisponible sans élévation dans cet environnement) ne révèle le
//! moindre message détaillé.
//!
//! Ce binaire élimine `wgpu`/`wgpu-hal`/`windows-rs`-au-travers-de-wgpu de l'équation : appel
//! Win32/DXGI/D3D12 brut, avec le MÊME device/queue/factory, pour trancher :
//! `CreateSwapChainForComposition` échoue-t-il aussi hors de wgpu-hal (bug/limite système, pas
//! wgpu) ? Et `CreateSwapChainForHwnd`, lui, réussit-il avec ce même device (confirmerait que le
//! device/queue/factory sont sains, comme déjà observé via wgpu-hal en mode DxgiFromHwnd) ?
//!
//! Résultat : voir README.md de ce dossier.

use std::ffi::c_void;

use windows::core::PCWSTR;
use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, WPARAM};
use windows::Win32::Graphics::Direct3D::D3D_FEATURE_LEVEL_11_0;
use windows::Win32::Graphics::Direct3D12::{
    D3D12CreateDevice, ID3D12CommandQueue, ID3D12Device, D3D12_COMMAND_QUEUE_DESC,
    D3D12_COMMAND_QUEUE_FLAG_NONE, D3D12_COMMAND_LIST_TYPE_DIRECT,
};
use windows::Win32::Graphics::Dxgi::Common::{
    DXGI_ALPHA_MODE_STRAIGHT, DXGI_FORMAT_B8G8R8A8_UNORM, DXGI_SAMPLE_DESC,
};
use windows::Win32::Graphics::Dxgi::{
    CreateDXGIFactory2, IDXGIFactory4, IDXGIOutput, DXGI_CREATE_FACTORY_FLAGS,
    DXGI_SCALING_STRETCH, DXGI_SWAP_CHAIN_DESC1, DXGI_SWAP_CHAIN_FLAG,
    DXGI_SWAP_EFFECT_FLIP_DISCARD, DXGI_SWAP_EFFECT_FLIP_SEQUENTIAL,
    DXGI_USAGE_RENDER_TARGET_OUTPUT,
};
use windows::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, RegisterClassW, CW_USEDEFAULT, WNDCLASSW, WS_OVERLAPPEDWINDOW,
};

unsafe extern "system" fn wndproc(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) }
}

fn create_dummy_hwnd() -> HWND {
    unsafe {
        let class_name = windows::core::w!("s1-raw-repro");
        let wc = WNDCLASSW {
            lpfnWndProc: Some(wndproc),
            lpszClassName: PCWSTR(class_name.as_ptr()),
            ..Default::default()
        };
        RegisterClassW(&wc);
        CreateWindowExW(
            Default::default(),
            PCWSTR(class_name.as_ptr()),
            windows::core::w!("s1-raw-repro"),
            WS_OVERLAPPEDWINDOW,
            CW_USEDEFAULT,
            CW_USEDEFAULT,
            420,
            220,
            None,
            None,
            None,
            None,
        )
        .expect("CreateWindowExW")
    }
}

fn base_desc(swap_effect: windows::Win32::Graphics::Dxgi::DXGI_SWAP_EFFECT, alpha_mode: windows::Win32::Graphics::Dxgi::Common::DXGI_ALPHA_MODE) -> DXGI_SWAP_CHAIN_DESC1 {
    DXGI_SWAP_CHAIN_DESC1 {
        Width: 420,
        Height: 220,
        Format: DXGI_FORMAT_B8G8R8A8_UNORM,
        Stereo: false.into(),
        SampleDesc: DXGI_SAMPLE_DESC { Count: 1, Quality: 0 },
        BufferUsage: DXGI_USAGE_RENDER_TARGET_OUTPUT,
        BufferCount: 3,
        Scaling: DXGI_SCALING_STRETCH,
        SwapEffect: swap_effect,
        AlphaMode: alpha_mode,
        Flags: DXGI_SWAP_CHAIN_FLAG(0).0 as u32,
    }
}

fn main() {
    println!("=== Repro brut Win32/DXGI/D3D12 — spike S1 ===\n");

    let hwnd = create_dummy_hwnd();
    println!("HWND créée : {hwnd:?}");

    let device: ID3D12Device =
        unsafe {
            let mut result: Option<ID3D12Device> = None;
            D3D12CreateDevice(None, D3D_FEATURE_LEVEL_11_0, &mut result)
                .expect("D3D12CreateDevice");
            result.expect("device non nul")
        };
    println!("ID3D12Device créé.");

    let queue: ID3D12CommandQueue = unsafe {
        device
            .CreateCommandQueue(&D3D12_COMMAND_QUEUE_DESC {
                Type: D3D12_COMMAND_LIST_TYPE_DIRECT,
                Flags: D3D12_COMMAND_QUEUE_FLAG_NONE,
                ..Default::default()
            })
            .expect("CreateCommandQueue")
    };
    println!("ID3D12CommandQueue créée.");

    let factory: IDXGIFactory4 =
        unsafe { CreateDXGIFactory2(DXGI_CREATE_FACTORY_FLAGS(0)).expect("CreateDXGIFactory2") };
    println!("IDXGIFactory4 créée.\n");

    // --- Test 1 : CreateSwapChainForHwnd (contrôle — doit réussir, comme déjà observé via
    //     wgpu-hal en mode DxgiFromHwnd) ---
    let desc_hwnd = base_desc(DXGI_SWAP_EFFECT_FLIP_DISCARD, windows::Win32::Graphics::Dxgi::Common::DXGI_ALPHA_MODE_IGNORE);
    let result_hwnd = unsafe {
        factory.CreateSwapChainForHwnd(&queue, hwnd, &desc_hwnd, None, None::<&IDXGIOutput>)
    };
    match &result_hwnd {
        Ok(_) => println!("[Test 1] CreateSwapChainForHwnd : OK (succès attendu, contrôle)"),
        Err(err) => println!("[Test 1] CreateSwapChainForHwnd : ÉCHEC inattendu — {err}"),
    }

    // --- Test 2 : CreateSwapChainForComposition, desc strictement conforme aux exigences
    //     documentées (FLIP_SEQUENTIAL, STRETCH, AlphaMode STRAIGHT — pas UNSPECIFIED). ---
    let desc_comp = base_desc(DXGI_SWAP_EFFECT_FLIP_SEQUENTIAL, DXGI_ALPHA_MODE_STRAIGHT);
    let result_comp =
        unsafe { factory.CreateSwapChainForComposition(&queue, &desc_comp, None::<&IDXGIOutput>) };
    match &result_comp {
        Ok(_) => println!(
            "[Test 2] CreateSwapChainForComposition : OK — succès HORS wgpu-hal ⇒ bug/limite \
             d'intégration wgpu-hal, pas du système."
        ),
        Err(err) => println!(
            "[Test 2] CreateSwapChainForComposition : ÉCHEC ({err}) — reproduit HORS wgpu-hal \
             ⇒ limite système/pilote, pas un bug wgpu-hal."
        ),
    }

    // --- Test 3 (repli, uniquement si le test 2 échoue) : même desc mais AlphaMode Premultiplied
    //     au lieu de Straight — élimine une dernière hypothèse sur l'AlphaMode. ---
    if result_comp.is_err() {
        let desc_premult = base_desc(
            DXGI_SWAP_EFFECT_FLIP_SEQUENTIAL,
            windows::Win32::Graphics::Dxgi::Common::DXGI_ALPHA_MODE_PREMULTIPLIED,
        );
        let result_premult = unsafe {
            factory.CreateSwapChainForComposition(&queue, &desc_premult, None::<&IDXGIOutput>)
        };
        match &result_premult {
            Ok(_) => println!("[Test 3] CreateSwapChainForComposition (AlphaMode Premultiplied) : OK"),
            Err(err) => println!("[Test 3] CreateSwapChainForComposition (AlphaMode Premultiplied) : ÉCHEC ({err})"),
        }
    }

    let _ = std::hint::black_box((&device, &queue, &factory));
    println!("\n=== Fin du repro ===");
}

// Interface manquante référencée ci-dessus pour le typage de `None::<&IDXGIOutput>` — évite un
// import inutilisé signalé par le compilateur si jamais retiré ailleurs.
#[allow(dead_code)]
fn _keep_import(_: *const c_void) {}
