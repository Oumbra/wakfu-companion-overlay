//! Méthode 2 — Windows Graphics Capture (`Windows.Graphics.Capture`, WinRT, Windows 10 1803+).
//!
//! La capture passe par la composition DWM : elle voit ce que l'application PRODUIT, pas ce qui
//! est affiché à l'écran — c'est ce qui la rend candidate pour une fenêtre occultée. Une session
//! par fenêtre, ouverte une fois ; DWM ne livre une nouvelle image que quand le contenu change,
//! d'où le compteur d'images reçues entre deux ticks : il vaut zéro si le client a cessé de
//! peindre, et c'est une information en soi.
//!
//! Sous Windows 10, un liseré jaune entoure la fenêtre capturée ; sous Windows 11 il se désactive
//! (`SetIsBorderRequired(false)`), mais seulement pour une application qui a demandé l'accès
//! `Borderless` — on tente, et on ignore le refus : c'est cosmétique.

use windows::core::Interface;
use windows::Graphics::Capture::{Direct3D11CaptureFramePool, GraphicsCaptureItem, GraphicsCaptureSession};
use windows::Graphics::DirectX::Direct3D11::IDirect3DDevice;
use windows::Graphics::DirectX::DirectXPixelFormat;
use windows::Win32::Foundation::HWND;
use windows::Win32::Graphics::Direct3D::D3D_DRIVER_TYPE_HARDWARE;
use windows::Win32::Graphics::Direct3D11::{
    D3D11CreateDevice, ID3D11Device, ID3D11DeviceContext, ID3D11Texture2D,
    D3D11_CPU_ACCESS_READ, D3D11_CREATE_DEVICE_BGRA_SUPPORT, D3D11_MAPPED_SUBRESOURCE,
    D3D11_MAP_READ, D3D11_SDK_VERSION, D3D11_TEXTURE2D_DESC, D3D11_USAGE_STAGING,
};
use windows::Win32::Graphics::Dxgi::IDXGIDevice;
use windows::Win32::System::WinRT::Direct3D11::{
    CreateDirect3D11DeviceFromDXGIDevice, IDirect3DDxgiInterfaceAccess,
};
use windows::Win32::System::WinRT::Graphics::Capture::IGraphicsCaptureItemInterop;
use windows::Win32::System::WinRT::{RoInitialize, RO_INIT_MULTITHREADED};

use crate::Frame;

struct Shared {
    device: ID3D11Device,
    context: ID3D11DeviceContext,
    winrt_device: IDirect3DDevice,
}

static mut SHARED: Option<Shared> = None;

fn shared() -> &'static Shared {
    // Spike mono-thread : l'unique accès concurrent est celui du pool WGC, qui ne touche pas à
    // cette structure (il ne fait que déposer des images).
    unsafe { (*std::ptr::addr_of!(SHARED)).as_ref().expect("wgc::init() d'abord") }
}

pub fn init() -> windows::core::Result<()> {
    unsafe {
        // Déjà initialisé (autre apartment) n'est pas une erreur pour un spike.
        let _ = RoInitialize(RO_INIT_MULTITHREADED);

        let mut device: Option<ID3D11Device> = None;
        let mut context: Option<ID3D11DeviceContext> = None;
        D3D11CreateDevice(
            None,
            D3D_DRIVER_TYPE_HARDWARE,
            Default::default(),
            D3D11_CREATE_DEVICE_BGRA_SUPPORT,
            None,
            D3D11_SDK_VERSION,
            Some(&mut device),
            None,
            Some(&mut context),
        )?;
        let device = device.expect("device D3D11");
        let context = context.expect("contexte D3D11");
        let dxgi: IDXGIDevice = device.cast()?;
        let winrt_device: IDirect3DDevice = CreateDirect3D11DeviceFromDXGIDevice(&dxgi)?.cast()?;
        SHARED = Some(Shared {
            device,
            context,
            winrt_device,
        });
    }
    Ok(())
}

pub struct Session {
    _item: GraphicsCaptureItem,
    pool: Direct3D11CaptureFramePool,
    _session: GraphicsCaptureSession,
    last: Option<Frame>,
}

impl Session {
    pub fn open(hwnd: HWND) -> windows::core::Result<Self> {
        let interop = windows::core::factory::<GraphicsCaptureItem, IGraphicsCaptureItemInterop>()?;
        let item: GraphicsCaptureItem = unsafe { interop.CreateForWindow(hwnd)? };
        let size = item.Size()?;
        let pool = Direct3D11CaptureFramePool::CreateFreeThreaded(
            &shared().winrt_device,
            DirectXPixelFormat::B8G8R8A8UIntNormalized,
            2,
            size,
        )?;
        let session = pool.CreateCaptureSession(&item)?;
        let _ = session.SetIsCursorCaptureEnabled(false);
        let _ = session.SetIsBorderRequired(false);
        session.StartCapture()?;
        Ok(Self {
            _item: item,
            pool,
            _session: session,
            last: None,
        })
    }

    /// Draine toutes les images arrivées depuis le dernier appel, garde la plus récente, et
    /// renvoie `(dernière image connue, nombre d'images reçues depuis le tick précédent)`.
    /// Une image « connue » peut dater d'un tick précédent si DWM n'a rien livré entre-temps :
    /// c'est le compteur qui dit si la fenêtre vit encore.
    pub fn latest(&mut self) -> (Option<Frame>, u32) {
        let mut received = 0u32;
        let mut newest = None;
        while let Ok(frame) = self.pool.TryGetNextFrame() {
            received += 1;
            newest = Some(frame);
        }
        if let Some(frame) = newest {
            match read_frame(&frame) {
                Ok(f) => self.last = Some(f),
                Err(e) => eprintln!("[wgc] lecture d'image : {e}"),
            }
        }
        (
            self.last.as_ref().map(|f| Frame {
                width: f.width,
                height: f.height,
                rgba: f.rgba.clone(),
            }),
            received,
        )
    }
}

fn read_frame(frame: &windows::Graphics::Capture::Direct3D11CaptureFrame) -> windows::core::Result<Frame> {
    let shared = shared();
    let content = frame.ContentSize()?;
    let surface = frame.Surface()?;
    let access: IDirect3DDxgiInterfaceAccess = surface.cast()?;
    let tex: ID3D11Texture2D = unsafe { access.GetInterface()? };

    let mut desc = D3D11_TEXTURE2D_DESC::default();
    unsafe { tex.GetDesc(&mut desc) };
    // Le contenu peut être plus petit que la texture du pool (redimensionnement en cours) : on
    // ne lit que la partie utile.
    let w = (content.Width.max(0) as u32).min(desc.Width);
    let h = (content.Height.max(0) as u32).min(desc.Height);

    let staging_desc = D3D11_TEXTURE2D_DESC {
        Usage: D3D11_USAGE_STAGING,
        CPUAccessFlags: D3D11_CPU_ACCESS_READ.0 as u32,
        BindFlags: 0,
        MiscFlags: 0,
        ..desc
    };
    let mut staging: Option<ID3D11Texture2D> = None;
    unsafe {
        shared
            .device
            .CreateTexture2D(&staging_desc, None, Some(&mut staging))?
    };
    let staging = staging.expect("texture staging");
    unsafe { shared.context.CopyResource(&staging, &tex) };

    let mut mapped = D3D11_MAPPED_SUBRESOURCE::default();
    unsafe {
        shared
            .context
            .Map(&staging, 0, D3D11_MAP_READ, 0, Some(&mut mapped))?
    };
    let mut rgba = Vec::with_capacity((w * h * 4) as usize);
    unsafe {
        let base = mapped.pData as *const u8;
        for y in 0..h {
            let row = std::slice::from_raw_parts(
                base.add((y * mapped.RowPitch) as usize),
                (w * 4) as usize,
            );
            for px in row.chunks_exact(4) {
                rgba.extend_from_slice(&[px[2], px[1], px[0], 255]);
            }
        }
        shared.context.Unmap(&staging, 0);
    }
    Ok(Frame {
        width: w,
        height: h,
        rgba,
    })
}
