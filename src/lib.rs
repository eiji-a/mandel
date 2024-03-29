//
//use std::{borrow::Cow, convert::TryInto};
use std::convert::TryInto;
use std::fmt;
use wgpu::util::DeviceExt;
use image::{ImageBuffer, Rgba, RgbaImage};


#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct Vector4 {
  pub v: [f32; 4],
}

impl fmt::Display for Vector4 {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    write!(f, "V4[{},{},{},{}]", self.v[0], self.v[1], self.v[2], self.v[3])
  }
}

impl Vector4 {
  fn new(x: f32, y: f32, z: f32, w: f32) -> Self {
    Self {v: [x, y, z, w]}
  }
}

//
struct State {
  device: wgpu::Device,
  queue: wgpu::Queue,
  compute_pipeline: wgpu::ComputePipeline,
  storage_buffer: wgpu::Buffer,
  staging_buffer: wgpu::Buffer,
  bind_group: wgpu::BindGroup,
}

impl State {
  async fn new() -> Self {
    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
      backends: wgpu::Backends::all(),
      dx12_shader_compiler: Default::default(),
    });
    let adapter = instance
      .request_adapter(&wgpu::RequestAdapterOptions::default())
      .await
      .unwrap();
    let (device, queue) = adapter
      .request_device(
        &wgpu::DeviceDescriptor {
          label: None,
          features: wgpu::Features::empty(),
          limits: wgpu::Limits::default(),
        },
        None,
      )
      .await
      .unwrap();

    /*
    let mut flags = wgpu::ShaderFlags::VALIDATION;
    match adapter.get_info().backend {
      wgpu::Backend::Vulkan | wgpu::Backend::Metal | wgpu::Backend::Gl => {
        flags |= wgpu::ShaderFlags::EXPERIMENTAL_TRANSLATION;
      }
      _ => {}
    }
    */
    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
      label: Some("Shader"),
      source: wgpu::ShaderSource::Wgsl(include_str!("shader.wgsl").into()),
    });

    let compute_pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
      label: Some("Compute"),
      layout: None,
      module: &shader,
      entry_point: "main",
    });

    const BUFFSIZE: usize = std::mem::size_of::<Vector4>() * 65536;

    let buffsize = BUFFSIZE as wgpu::BufferAddress;

    let staging_buffer = device.create_buffer(&wgpu::BufferDescriptor {
      label: None,
      size: buffsize,
      usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
      mapped_at_creation: false,
    });

    /*
    let storage_buffer = self.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
      label: Some("Storage Buffer"),
      contents: bytemuck::cast_slice(&points),
      usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::COPY_SRC,
    });
    */
    let storage_buffer = device.create_buffer(&wgpu::BufferDescriptor {
      label: Some("Storage Buffer"),
      size: buffsize,
      usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::COPY_SRC,
      mapped_at_creation: false,
    });

    let bind_group_layout = compute_pipeline.get_bind_group_layout(0);
    let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
      label: None,
      layout: &bind_group_layout,
      entries: &[wgpu::BindGroupEntry {
        binding: 0,
        resource: storage_buffer.as_entire_binding(),
      }],
    });

    Self {
      device,
      queue,
      compute_pipeline,
      storage_buffer,
      staging_buffer,
      bind_group,
    }

  }


  async fn compute(&mut self, points: &Vec<Vector4>) -> Result<Vec<Vector4>, wgpu::SurfaceError> {

    let mut encoder = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
      label: Some("Compute Encoder"),
    });
  
    {
      let mut cpass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
        label: Some("Compute Pass"),
      });
      cpass.set_pipeline(&self.compute_pipeline);
      cpass.set_bind_group(0, &self.bind_group, &[]);
      cpass.insert_debug_marker("compute mandel");
      //cpass.dispatch_workgroups(points.len() as u32, 1, 1);
      cpass.dispatch_workgroups(256, 1, 1);
    }
  
    let datasize = (points.len() * std::mem::size_of::<Vector4>()) as wgpu::BufferAddress;
    self.queue.write_buffer(&self.storage_buffer, 0, bytemuck::cast_slice(&points));

    encoder.copy_buffer_to_buffer(&self.storage_buffer, 0, &self.staging_buffer, 0, datasize);
    self.queue.submit(Some(encoder.finish()));
  
    let buffer_slice = self.staging_buffer.slice(0..datasize);
    let (tx, rx) = futures_intrusive::channel::shared::oneshot_channel();
    buffer_slice.map_async(wgpu::MapMode::Read, move |result| {
      tx.send(result).unwrap();
    });
    self.device.poll(wgpu::Maintain::Wait);
    rx.receive().await.unwrap().unwrap();
  
    let data = buffer_slice.get_mapped_range();
    let result: Vec<f32> = data
      .chunks_exact(4)
      .map(|b| f32::from_ne_bytes(b.try_into().unwrap()))
      .collect();

  
    drop(data);
    self.staging_buffer.unmap();

    let vectors = result.chunks(4).map(|v| Vector4::new(v[0], v[1], v[2], v[3])).collect();
    Ok(vectors)
  
  }

}

const LIMIT: u32 = 256;
const THRE: f32 = 4.0;

fn solve_mandel(points: &Vec<Vector4>) -> Result<Vec<Vector4>, bool> {
  let mut res: Vec<Vector4> = vec![];

  for c in points {
    let mut a: f32 = 0.0;
    let mut b: f32 = 0.0;
    let mut d: f32 = 0.0;
    let mut r: Vector4 = Vector4::new(0.0, 0.0, 0.0, 250.0);
    for i in 0..LIMIT {
      let a2 = a * a - (b * b) + c.v[0];
      let b2 = 2.0 * a * b + c.v[1];
      a = a2;
      b = b2;
      d = a * a + b * b;
      if d > THRE {
        let g = (i as f32) * 5.0;
        //let h = 255.0 / (1.0 + f32::exp(-f32::log(d)));
        r = Vector4::new(g, g, 0.0, 250.0);
        break
      }
    }
    res.push(r);
  }
  Ok(res)
}

const WIDTH:  f32 = 4.0;
const HEIGHT: f32 = 4.0;

pub async fn run(x: &u32, y: &u32) {
  let mut state = State::new().await;
  let stepx = WIDTH  / (*x as f32);
  let stepy = HEIGHT / (*y as f32);
  let dx = stepx / 2.0;
  let dy = stepy / 2.0;
  //let img: RgbaImage = ImageBuffer::from_raw(*x, *y, pixels).unwrap();
  let mut img: RgbaImage = ImageBuffer::new(*x, *y);
  let mut points: Vec<Vector4>;
  for j in 0..*y {
    let j2 = *y - 1 - j;
    points = Vec::new();
    for i in 0..*x {
      let vx = (i  as f32) * stepx + dx - (WIDTH  / 2.0);
      let vy = (j2 as f32) * stepy + dy - (HEIGHT / 2.0);
      points.push(Vector4::new(vx, vy, 0.0, 0.0))
    }
    match state.compute(&points).await {
    //match solve_mandel(&points) {
      Ok(result) => {
        let pixels: Vec<[u8; 4]> = result.iter()
          .map(|v| [v.v[0] as u8, v.v[1] as u8, v.v[2] as u8, v.v[3] as u8])
          .collect();
        for (i, val) in pixels.iter().enumerate() {
          img.put_pixel(i as u32, j2, Rgba::<u8>::from(*val))
        }
      }
      Err(_) => log::warn!("compute error"),
    }
  }

  img.save("mandel.png").unwrap();

}

// real	0m11.522s
// user	0m6.110s
// sys	0m0.888s


