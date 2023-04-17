//
//use std::{borrow::Cow, convert::TryInto};
use std::convert::TryInto;
use std::fmt;
use wgpu::util::DeviceExt;
use image::{ImageBuffer, RgbaImage};


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


    Self {
      device,
      queue,
      compute_pipeline,
    }

  }

  async fn compute(&mut self, points: &Vec<Vector4>) -> Result<Vec<Vector4>, wgpu::SurfaceError> {

    let slice_size = points.len() * std::mem::size_of::<Vector4>();
    let size = slice_size as wgpu::BufferAddress;

    let staging_buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
      label: None,
      size,
      usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
      mapped_at_creation: false,
    });

    let storage_buffer = self.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
      label: Some("Storage Buffer"),
      contents: bytemuck::cast_slice(&points),
      usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::COPY_SRC,
    });

    let bind_group_layout = self.compute_pipeline.get_bind_group_layout(0);
    let bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
      label: None,
      layout: &bind_group_layout,
      entries: &[wgpu::BindGroupEntry {
        binding: 0,
        resource: storage_buffer.as_entire_binding(),
      }],
    });


    let mut encoder = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
      label: Some("Compute Encoder"),
    });
  
    {
      let mut cpass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
        label: Some("Compute Pass"),
      });
      cpass.set_pipeline(&self.compute_pipeline);
      cpass.set_bind_group(0, &bind_group, &[]);
      cpass.insert_debug_marker("compute mandel");
      cpass.dispatch_workgroups(points.len() as u32, 1, 1);
    }
  
    encoder.copy_buffer_to_buffer(&storage_buffer, 0, &staging_buffer, 0, size);
    self.queue.submit(Some(encoder.finish()));
  
    let buffer_slice = staging_buffer.slice(..);
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
    let vectors = result.chunks(4).map(|v| Vector4::new(v[0], v[1], v[2], v[3])).collect();

  
    drop(data);
    staging_buffer.unmap();
    Ok(vectors)
  
  }

}

const WIDTH:  f32 = 4.0;
const HEIGHT: f32 = 4.0;

pub async fn run(x: &u32, y: &u32) {
  let mut state = State::new().await;
//  let result = state.compute(*numbers);
//  let disp_result = result.iter()
//    .map(|&n| match n {
//      OVERFLOW => "OVERFLOW".to_string(),
//      _ => n.to_string(),
//    })
//    .collect();
//  println!("Steps: [{}]", disp_result.join(", "));
//  log::info!("Steps: [{}]", disp_result.join(", "));

  let stepx = WIDTH  / (*x as f32);
  let stepy = HEIGHT / (*y as f32);
  let dx = stepx / 2.0;
  let dy = stepy / 2.0;
  let mut points: Vec<Vector4> = Vec::new();
  for j in 0..*y {
    for i in 0..*x {
      let vx = (i       as f32) * stepx + dx - (WIDTH  / 2.0);
      let vy = ((y - j) as f32) * stepy - dy - (HEIGHT / 2.0);
      points.push(Vector4::new(vx, vy, 0.0, 0.0))
    }
  }

  /*
  let disp_before: Vec<String> = points.iter()
    .map(|&n| n.to_string())
    .collect();
  println!("Before: [{}]", disp_before.join(", "));
  */

  match state.compute(&points).await {
    Ok(result) => {
      /*
      let disp_result: Vec<String> = result.iter()
        .map(|&n| match n {
          //OVERFLOW => "OVERFLOW".to_string(),
          _ => n.to_string(),
        })
        .collect();

      println!("Steps: [{}]", disp_result.join(", "));
      log::info!("Steps: [{}]", disp_result.join(", "));
      */
      let pixels: Vec<u8> = result.iter()
        .map(|v| [v.v[0] as u8, v.v[1] as u8, v.v[2] as u8, v.v[3] as u8])
        .flatten()
        .collect();
      let img: RgbaImage = ImageBuffer::from_raw(*x, *y, pixels).unwrap();
      img.save("mandel.png").unwrap();

    }
    Err(_) => log::warn!("compute error"),
  }


}

