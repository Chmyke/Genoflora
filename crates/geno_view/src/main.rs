//! The window: one seed grows live on the graphics card and is drawn where it lives.
//!
//! Usage: geno_view                      opens the window
//!        geno_view --capture out.png [steps=21000] [mode=1..6] [cut]
//!                                       grows a seed without a window and writes one picture
//!
//! Keys: space pauses, 1 to 6 choose what the colour shows, C cuts the plant open, up and down
//! change the speed, F follows the plant again. Mouse: left drag turns, right drag moves up and
//! down, wheel zooms.

use geno_gpu::{Adapter, GpuSim};
use geno_sim::{Cells, Engine, EngineError, Params};
use geno_view::{Camera, DEPTH, Look, Mode, Renderer, STILL, png, still, target};
use std::sync::Arc;
use std::time::{Duration, Instant};
use winit::application::ApplicationHandler;
use winit::event::{ElementState, MouseButton, MouseScrollDelta, WindowEvent};
use winit::event_loop::{ActiveEventLoop, EventLoop};
use winit::keyboard::{KeyCode, PhysicalKey};
use winit::window::{Window, WindowId};

/// Most cells the plant can hold.
const CAPACITY: usize = 1 << 17;

type Failure = Box<dyn std::error::Error>;

/// A camera that frames the whole plant, from the cells read back.
fn framing(cells: &Cells, p: &Params) -> Camera {
    let height = |q: &[f32; 4]| -(q[0] * p.gravity[0] + q[1] * p.gravity[1] + q[2] * p.gravity[2]);
    let (mut up, mut down) = (2.0f32, 2.0f32);
    for q in cells.alive() {
        up = up.max(height(q));
        down = down.max(-height(q));
    }
    Camera::framing(up + 2.0, down + 2.0)
}

fn capture(args: &[String]) -> Result<(), Failure> {
    let out = args.first().ok_or("missing output file")?;
    let (mut steps, mut mode, mut cut) = (21_000u32, Mode::Light, false);
    for arg in &args[1..] {
        match arg.split_once('=') {
            Some(("steps", v)) => steps = v.parse()?,
            Some(("mode", v)) => {
                let n: usize = v.parse()?;
                mode = *Mode::ALL.get(n.wrapping_sub(1)).ok_or("mode goes from 1 to 6")?;
            }
            None if arg == "cut" => cut = true,
            _ => return Err(format!("unknown argument {arg}").into()),
        }
    }
    let p = Params::default();
    let mut sim = GpuSim::new(&p, &Cells::seed(&p, CAPACITY, 1.0), Adapter::Hardware)?;
    geno_sim::checks::run(&mut sim, steps, Some(p.divide_every));
    let cells = sim.snapshot()?;
    let renderer = Renderer::new(sim.device(), STILL);
    let look = Look {
        mode,
        cut,
        ..Look::new(framing(&cells, &p))
    };
    let (width, height) = (800, 1000);
    let rgba = still(&mut sim, &renderer, &p, &look, width, height)?;
    std::fs::write(out, png::encode(width, height, &rgba))?;
    println!("{out}: {} cells, {}", cells.count, mode.label());
    Ok(())
}

/// Everything that lives as long as the window.
struct Live {
    window: Arc<Window>,
    surface: wgpu::Surface<'static>,
    config: wgpu::SurfaceConfiguration,
    depth: wgpu::TextureView,
    sim: GpuSim,
    renderer: Renderer,
}

struct App {
    p: Params,
    look: Look,
    live: Option<Live>,
    paused: bool,
    /// Simulation steps per picture.
    speed: u32,
    /// The camera keeps the whole plant in view until the user moves it.
    follow: bool,
    steps: u64,
    cells: usize,
    last_look: Instant,
    cursor: (f64, f64),
    turning: bool,
    lifting: bool,
    failure: Option<String>,
}

impl App {
    fn open(&self, event_loop: &ActiveEventLoop) -> Result<Live, Failure> {
        let attributes = Window::default_attributes()
            .with_title("Genoflora")
            .with_inner_size(winit::dpi::LogicalSize::new(900.0, 1000.0));
        let window = Arc::new(event_loop.create_window(attributes)?);
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle_from_env());
        let surface = instance.create_surface(window.clone())?;
        let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            compatible_surface: Some(&surface),
            ..Default::default()
        }))?;
        let sim = GpuSim::on_adapter(&self.p, &Cells::seed(&self.p, CAPACITY, 1.0), &adapter)?;
        let size = window.inner_size();
        let caps = surface.get_capabilities(&adapter);
        // The shader writes display colours: take a format that does not convert them again.
        let format = caps
            .formats
            .iter()
            .copied()
            .find(|f| !f.is_srgb())
            .or(caps.formats.first().copied())
            .ok_or("the window offers no colour format")?;
        let mut config = surface
            .get_default_config(&adapter, size.width.max(1), size.height.max(1))
            .ok_or("the graphics card cannot draw in this window")?;
        config.format = format;
        surface.configure(sim.device(), &config);
        let renderer = Renderer::new(sim.device(), format);
        let depth = depth_view(sim.device(), &config);
        Ok(Live {
            window,
            surface,
            config,
            depth,
            sim,
            renderer,
        })
    }

    fn title(&self) -> String {
        format!(
            "Genoflora | {} | {} cellules | pas {} | {}{}",
            self.look.mode.label(),
            self.cells,
            self.steps,
            if self.paused {
                "en pause".to_string()
            } else {
                format!("{} pas par image", self.speed)
            },
            if self.look.cut { " | coupe" } else { "" },
        )
    }

    /// Once a second: reads the cells back to count them and, when following, to frame them.
    fn look_again(&mut self) -> Result<(), EngineError> {
        let Some(live) = self.live.as_mut() else {
            return Ok(());
        };
        let cells = live.sim.snapshot()?;
        self.cells = cells.count;
        if self.follow {
            let wanted = framing(&cells, &self.p);
            let c = &mut self.look.camera;
            c.distance += 0.2 * (wanted.distance - c.distance);
            for k in 0..3 {
                c.target[k] += 0.2 * (wanted.target[k] - c.target[k]);
            }
        }
        let title = self.title();
        if let Some(live) = self.live.as_ref() {
            live.window.set_title(&title);
        }
        Ok(())
    }

    fn draw(&mut self) -> Result<(), Failure> {
        if self.last_look.elapsed() >= Duration::from_secs(1) {
            self.last_look = Instant::now();
            self.look_again()?;
        }
        let Some(live) = self.live.as_mut() else {
            return Ok(());
        };
        if !self.paused {
            for _ in 0..self.speed {
                live.sim.step();
                live.sim.divide();
            }
            self.steps += u64::from(self.speed);
        }
        use wgpu::CurrentSurfaceTexture as T;
        let frame = match live.surface.get_current_texture() {
            T::Success(frame) | T::Suboptimal(frame) => frame,
            T::Outdated | T::Lost => {
                live.surface.configure(live.sim.device(), &live.config);
                return Ok(());
            }
            _ => return Ok(()),
        };
        let aspect = live.config.width as f32 / live.config.height as f32;
        let view = frame.texture.create_view(&Default::default());
        live.renderer
            .draw(&mut live.sim, &self.p, &self.look, &view, &live.depth, aspect);
        live.sim.queue().present(frame);
        Ok(())
    }

    fn key(&mut self, code: KeyCode) {
        match code {
            KeyCode::Space => self.paused = !self.paused,
            KeyCode::KeyC => self.look.cut = !self.look.cut,
            KeyCode::KeyF => self.follow = true,
            KeyCode::ArrowUp => self.speed = (self.speed * 2).min(256),
            KeyCode::ArrowDown => self.speed = (self.speed / 2).max(1),
            KeyCode::Digit1 | KeyCode::Numpad1 => self.look.mode = Mode::ALL[0],
            KeyCode::Digit2 | KeyCode::Numpad2 => self.look.mode = Mode::ALL[1],
            KeyCode::Digit3 | KeyCode::Numpad3 => self.look.mode = Mode::ALL[2],
            KeyCode::Digit4 | KeyCode::Numpad4 => self.look.mode = Mode::ALL[3],
            KeyCode::Digit5 | KeyCode::Numpad5 => self.look.mode = Mode::ALL[4],
            KeyCode::Digit6 | KeyCode::Numpad6 => self.look.mode = Mode::ALL[5],
            _ => return,
        }
        let title = self.title();
        if let Some(live) = self.live.as_ref() {
            live.window.set_title(&title);
        }
    }
}

fn depth_view(device: &wgpu::Device, config: &wgpu::SurfaceConfiguration) -> wgpu::TextureView {
    target(
        device,
        config.width,
        config.height,
        DEPTH,
        wgpu::TextureUsages::RENDER_ATTACHMENT,
    )
    .create_view(&Default::default())
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.live.is_some() {
            return;
        }
        match self.open(event_loop) {
            Ok(live) => {
                live.window.request_redraw();
                self.live = Some(live);
            }
            Err(e) => {
                self.failure = Some(e.to_string());
                event_loop.exit();
            }
        }
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::Resized(size) => {
                if let Some(live) = self.live.as_mut() {
                    live.config.width = size.width.max(1);
                    live.config.height = size.height.max(1);
                    live.surface.configure(live.sim.device(), &live.config);
                    live.depth = depth_view(live.sim.device(), &live.config);
                }
            }
            WindowEvent::RedrawRequested => {
                if let Err(e) = self.draw() {
                    self.failure = Some(e.to_string());
                    event_loop.exit();
                }
                if let Some(live) = self.live.as_ref() {
                    live.window.request_redraw();
                }
            }
            WindowEvent::KeyboardInput { event, .. } => {
                if let (ElementState::Pressed, PhysicalKey::Code(code)) = (event.state, event.physical_key) {
                    self.key(code);
                }
            }
            WindowEvent::MouseInput { state, button, .. } => {
                let down = state == ElementState::Pressed;
                match button {
                    MouseButton::Left => self.turning = down,
                    MouseButton::Right => self.lifting = down,
                    _ => {}
                }
            }
            WindowEvent::CursorMoved { position, .. } => {
                let (dx, dy) = (position.x - self.cursor.0, position.y - self.cursor.1);
                self.cursor = (position.x, position.y);
                let c = &mut self.look.camera;
                if self.turning {
                    c.yaw -= dx as f32 * 0.008;
                    c.pitch = (c.pitch + dy as f32 * 0.008).clamp(-1.5, 1.5);
                }
                if self.lifting {
                    let lift = dy as f32 * c.distance * 0.0012;
                    for k in 0..3 {
                        c.target[k] -= lift * self.p.gravity[k];
                    }
                    self.follow = false;
                }
            }
            WindowEvent::MouseWheel { delta, .. } => {
                let notches = match delta {
                    MouseScrollDelta::LineDelta(_, y) => y,
                    MouseScrollDelta::PixelDelta(p) => p.y as f32 / 40.0,
                };
                let c = &mut self.look.camera;
                c.distance = (c.distance * 0.88f32.powf(notches)).clamp(3.0, 3000.0);
                self.follow = false;
            }
            _ => {}
        }
    }
}

fn main() -> Result<(), Failure> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.first().is_some_and(|a| a == "--capture") {
        return capture(&args[1..]);
    }
    let p = Params::default();
    let mut app = App {
        p,
        look: Look::new(Camera::framing(6.0, 6.0)),
        live: None,
        paused: false,
        speed: 8,
        follow: true,
        steps: 0,
        cells: 1,
        last_look: Instant::now(),
        cursor: (0.0, 0.0),
        turning: false,
        lifting: false,
        failure: None,
    };
    EventLoop::new()?.run_app(&mut app)?;
    match app.failure {
        Some(why) => Err(why.into()),
        None => Ok(()),
    }
}
