//! 3D view of Genoflora. The cells are drawn where they live, on the GPU, straight from the
//! simulation's buffers: nothing is copied or read back, and the simulation never depends on
//! this crate.
//!
//! The renderer draws into any texture, so the same code serves the window, the still captures
//! and the tests.

pub mod png;

use geno_gpu::{Drawn, GpuSim};
use geno_sim::{EngineError, Params};
use wgpu::{Buffer, Device, RenderPipeline, TextureFormat, TextureView};

/// Depth buffer format of every view.
pub const DEPTH: TextureFormat = TextureFormat::Depth32Float;
/// Colour of the air behind the plant.
pub const AIR: [f64; 3] = [0.965, 0.957, 0.933];

/// What the colour of a cell shows. The numbers are the `mode` of `shaders/cells.wgsl`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode {
    /// Light received: dark in the soil or in the shade of other cells.
    Light = 0,
    /// Youth: above one half the cell can divide.
    Youth = 1,
    Auxin = 2,
    Cytokinin = 3,
    /// Which way the pumps point: blue down, red up.
    PumpsVertical = 4,
    /// How strongly the pumps gather on one side.
    PumpsStrength = 5,
    /// What keeps the cells of a shoot tip young.
    Relay = 6,
    /// What the skin of a shoot tip sends back to hold the relay.
    Restraint = 7,
}

impl Mode {
    pub const ALL: [Mode; 8] = [
        Mode::Light,
        Mode::Youth,
        Mode::Auxin,
        Mode::Cytokinin,
        Mode::PumpsVertical,
        Mode::PumpsStrength,
        Mode::Relay,
        Mode::Restraint,
    ];

    /// The hormone this mode shows (its index in a cell's hormones), if it shows one.
    pub fn hormone(self) -> Option<usize> {
        match self {
            Mode::Auxin => Some(geno_sim::cells::AUXIN),
            Mode::Cytokinin => Some(geno_sim::cells::CYTOKININ),
            Mode::Restraint => Some(geno_sim::cells::RESTRAINT),
            Mode::Relay => Some(geno_sim::cells::RELAY),
            _ => None,
        }
    }

    /// Name shown to the user.
    pub fn label(self) -> &'static str {
        match self {
            Mode::Light => "Lumière reçue",
            Mode::Youth => "Jeunesse",
            Mode::Auxin => "Auxine",
            Mode::Cytokinin => "Cytokinine",
            Mode::PumpsVertical => "Pompes, sens vertical",
            Mode::PumpsStrength => "Pompes, force",
            Mode::Relay => "Relais",
            Mode::Restraint => "Signal de retenue",
        }
    }
}

/// A camera that turns around a point.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Camera {
    pub target: [f32; 3],
    /// Angle around the vertical, in radians.
    pub yaw: f32,
    /// Angle above the horizon, in radians.
    pub pitch: f32,
    pub distance: f32,
}

/// Vertical field of view, in radians.
const FIELD: f32 = 0.6;

fn sub(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

fn dot(a: [f32; 3], b: [f32; 3]) -> f32 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

fn cross(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

fn unit(a: [f32; 3]) -> [f32; 3] {
    let len = dot(a, a).sqrt().max(1e-12);
    a.map(|v| v / len)
}

/// Where the camera stands and how it is turned: eye, right, up and forward unit vectors.
struct Frame {
    eye: [f32; 3],
    right: [f32; 3],
    up: [f32; 3],
    forward: [f32; 3],
}

impl Camera {
    /// A camera that frames a plant reaching `up` above and `down` below the origin.
    pub fn framing(up: f32, down: f32) -> Self {
        let height = up + down;
        Self {
            target: [0.0, (up - down) * 0.5, 0.0],
            yaw: 0.6,
            pitch: 0.15,
            distance: (height * 0.5 + 4.0) / (FIELD * 0.5).tan() * 1.1,
        }
    }

    fn frame(&self, sky: [f32; 3]) -> Frame {
        // Two directions across the vertical, so the camera turns around `sky` wherever it points.
        let seed = if sky[0].abs() < 0.9 {
            [1.0, 0.0, 0.0]
        } else {
            [0.0, 0.0, 1.0]
        };
        let side = unit(cross(sky, seed));
        let other = cross(sky, side);
        let (cp, sp) = (self.pitch.cos(), self.pitch.sin());
        let (cy, sy) = (self.yaw.cos(), self.yaw.sin());
        let away = [0, 1, 2].map(|k| cp * (sy * side[k] + cy * other[k]) + sp * sky[k]);
        let eye = [0, 1, 2].map(|k| self.target[k] + self.distance * away[k]);
        let forward = unit(sub(self.target, eye));
        let right = unit(cross(forward, sky));
        let up = cross(right, forward);
        Frame {
            eye,
            right,
            up,
            forward,
        }
    }
}

/// Everything one picture depends on, besides the cells.
#[derive(Clone, Copy, Debug)]
pub struct Look {
    pub camera: Camera,
    pub mode: Mode,
    /// Hides the half of the plant nearer than the point the camera looks at, to see inside.
    pub cut: bool,
    /// Concentrations shown as the top of the colour scale: auxin, cytokinin, restraint signal,
    /// relay (the order of a cell's hormones).
    pub tops: [f32; 4],
}

impl Look {
    pub fn new(camera: Camera) -> Self {
        Self {
            camera,
            mode: Mode::Light,
            cut: false,
            tops: [8.0, 4.0, 0.3, 2.0],
        }
    }

    /// Uniform block of the shader. Layout mirrors `View` in `shaders/cells.wgsl`.
    fn bytes(&self, p: &Params, aspect: f32) -> Vec<u8> {
        let sky = p.gravity.map(|g| -g);
        let f = self.camera.frame(sky);
        let (near, far) = (0.5f32, 5000.0f32);
        let focal = 1.0 / (FIELD * 0.5).tan();
        // Column-major view-projection (right-handed, depth from 0 to 1).
        let depth = far / (near - far);
        let rows = [f.right, f.up, f.forward.map(|v| -v)];
        let shift = [-dot(f.right, f.eye), -dot(f.up, f.eye), dot(f.forward, f.eye)];
        let scale = [focal / aspect, focal, depth];
        let mut m = [0.0f32; 16];
        for col in 0..3 {
            for row in 0..3 {
                m[col * 4 + row] = rows[row][col] * scale[row];
            }
            m[col * 4 + 3] = -rows[2][col];
        }
        for row in 0..3 {
            m[12 + row] = shift[row] * scale[row];
        }
        m[14] += near * depth;
        m[15] = -shift[2];
        let v4 = |v: [f32; 3], w: f32| [v[0], v[1], v[2], w];
        let blocks = [
            v4(f.eye, 0.0),
            v4(f.right, 0.0),
            v4(f.up, 0.0),
            v4(sky, 0.0),
            v4(self.camera.target, f32::from(u8::from(self.cut))),
            [
                self.mode as u32 as f32,
                p.surface,
                self.tops[self.mode.hormone().unwrap_or(0)],
                0.0,
            ],
        ];
        let mut out: Vec<u8> = bytemuck::cast_slice(&m).to_vec();
        out.extend_from_slice(bytemuck::cast_slice(&blocks));
        out
    }
}

/// Draws the cells of a simulation, then the soil surface over them.
pub struct Renderer {
    cells: RenderPipeline,
    ground: RenderPipeline,
    view: Buffer,
}

impl Renderer {
    /// A renderer for colour targets of the given format (use a non-sRGB one: the shader writes
    /// display colours).
    pub fn new(device: &Device, format: TextureFormat) -> Self {
        let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("cells"),
            source: wgpu::ShaderSource::Wgsl(include_str!("../shaders/cells.wgsl").into()),
        });
        let pipeline = |vertex: &str, fragment: &str, blend: Option<wgpu::BlendState>, write: bool| {
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some(vertex),
                layout: None,
                vertex: wgpu::VertexState {
                    module: &module,
                    entry_point: Some(vertex),
                    compilation_options: Default::default(),
                    buffers: &[],
                },
                primitive: Default::default(),
                depth_stencil: Some(wgpu::DepthStencilState {
                    format: DEPTH,
                    depth_write_enabled: Some(write),
                    depth_compare: Some(wgpu::CompareFunction::Less),
                    stencil: Default::default(),
                    bias: Default::default(),
                }),
                multisample: Default::default(),
                fragment: Some(wgpu::FragmentState {
                    module: &module,
                    entry_point: Some(fragment),
                    compilation_options: Default::default(),
                    targets: &[Some(wgpu::ColorTargetState {
                        format,
                        blend,
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                }),
                multiview_mask: None,
                cache: None,
            })
        };
        let view = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("view"),
            size: 160,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        Self {
            cells: pipeline("cell", "paint", None, true),
            ground: pipeline("ground", "earth", Some(wgpu::BlendState::ALPHA_BLENDING), false),
            view,
        }
    }

    /// Draws the current cells of `sim` into `colour` (cleared first), `width / height` being its
    /// aspect ratio. The queued simulation steps are handed to the GPU first.
    pub fn draw(
        &self,
        sim: &mut GpuSim,
        p: &Params,
        look: &Look,
        colour: &TextureView,
        depth: &TextureView,
        aspect: f32,
    ) {
        let capacity = sim.capacity() as u32;
        let (device, queue) = (sim.device().clone(), sim.queue().clone());
        queue.write_buffer(&self.view, 0, &look.bytes(p, aspect));
        let drawn = sim.drawn();
        let group = |pipeline: &RenderPipeline, buffers: &[&Buffer]| {
            let entries: Vec<wgpu::BindGroupEntry> = buffers
                .iter()
                .enumerate()
                .map(|(binding, b)| wgpu::BindGroupEntry {
                    binding: binding as u32,
                    resource: b.as_entire_binding(),
                })
                .collect();
            device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: None,
                layout: &pipeline.get_bind_group_layout(0),
                entries: &entries,
            })
        };
        let Drawn {
            state,
            pos,
            shape,
            hormones,
            pin,
            youth,
        } = drawn;
        let cells = group(
            &self.cells,
            &[&self.view, state, pos, shape, hormones, pin, youth],
        );
        let ground = group(&self.ground, &[&self.view]);
        let mut encoder = device.create_command_encoder(&Default::default());
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("plant"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: colour,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: AIR[0],
                            g: AIR[1],
                            b: AIR[2],
                            a: 1.0,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: depth,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
            pass.set_pipeline(&self.cells);
            pass.set_bind_group(0, &cells, &[]);
            // One quad per slot; the shader throws away the slots beyond the live count.
            pass.draw(0..6, 0..capacity);
            pass.set_pipeline(&self.ground);
            pass.set_bind_group(0, &ground, &[]);
            pass.draw(0..6, 0..1);
        }
        queue.submit([encoder.finish()]);
    }
}

/// A texture view to draw into, with the given format and usage.
pub fn target(
    device: &Device,
    width: u32,
    height: u32,
    format: TextureFormat,
    usage: wgpu::TextureUsages,
) -> wgpu::Texture {
    device.create_texture(&wgpu::TextureDescriptor {
        label: None,
        size: wgpu::Extent3d {
            width: width.max(1),
            height: height.max(1),
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format,
        usage,
        view_formats: &[],
    })
}

/// Format of the still pictures.
pub const STILL: TextureFormat = TextureFormat::Rgba8Unorm;

/// Draws one still picture off screen and reads it back: `width * height` RGBA pixels, rows from
/// the top. `renderer` must have been made for `STILL`.
pub fn still(
    sim: &mut GpuSim,
    renderer: &Renderer,
    p: &Params,
    look: &Look,
    width: u32,
    height: u32,
) -> Result<Vec<u8>, EngineError> {
    use wgpu::TextureUsages as T;
    let device = sim.device().clone();
    let colour = target(&device, width, height, STILL, T::RENDER_ATTACHMENT | T::COPY_SRC);
    let depth = target(&device, width, height, DEPTH, T::RENDER_ATTACHMENT);
    renderer.draw(
        sim,
        p,
        look,
        &colour.create_view(&Default::default()),
        &depth.create_view(&Default::default()),
        width as f32 / height as f32,
    );
    // Rows of a texture copy are padded to 256 bytes.
    let row = (width * 4).div_ceil(256) * 256;
    let staging = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("still"),
        size: u64::from(row) * u64::from(height),
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    let mut encoder = device.create_command_encoder(&Default::default());
    encoder.copy_texture_to_buffer(
        colour.as_image_copy(),
        wgpu::TexelCopyBufferInfo {
            buffer: &staging,
            layout: wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(row),
                rows_per_image: Some(height),
            },
        },
        colour.size(),
    );
    let index = sim.queue().submit([encoder.finish()]);
    let fail = |what: &str, why: &dyn std::fmt::Display| EngineError(format!("{what}: {why}"));
    let slice = staging.slice(..);
    let (sender, receiver) = std::sync::mpsc::channel();
    slice.map_async(wgpu::MapMode::Read, move |result| {
        // The receiver only goes away if this function already returned an error.
        let _ = sender.send(result);
    });
    device
        .poll(wgpu::PollType::Wait {
            submission_index: Some(index),
            timeout: Some(std::time::Duration::from_secs(120)),
        })
        .map_err(|e| fail("GPU wait failed", &e))?;
    receiver
        .recv()
        .map_err(|e| fail("still never came back", &e))?
        .map_err(|e| fail("still refused", &e))?;
    let padded = slice.get_mapped_range().map_err(|e| fail("still unmapped", &e))?;
    let mut out = Vec::with_capacity((width * height * 4) as usize);
    for line in padded.chunks_exact(row as usize) {
        out.extend_from_slice(&line[..(width * 4) as usize]);
    }
    Ok(out)
}
