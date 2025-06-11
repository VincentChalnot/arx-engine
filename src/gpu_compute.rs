// This example demonstrates how to use WGPU to perform a simple compute operation
// @todo implement the game logic in a compute shader
use wgpu::util::DeviceExt;

#[tokio::main]
async fn main() {
    let instance = wgpu::Instance::default();
    let adapter = instance.request_adapter(&wgpu::RequestAdapterOptions::default()).await.unwrap();
    let (device, queue) = adapter.request_device(&Default::default()).await.unwrap();

    const BOARD_SIZE: usize = 81;
    const MAX_MOVES: usize = 1024;

    // Board 9x9 (flattened)
    let board: [u8; BOARD_SIZE] = [0; BOARD_SIZE];
    let board_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("Board Buffer"),
        contents: bytemuck::cast_slice(&board),
        usage: wgpu::BufferUsages::STORAGE,
    });

    let output_buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("Output Buffer"),
        size: (MAX_MOVES * std::mem::size_of::<u16>()) as u64,
        usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
        mapped_at_creation: false,
    });

    let staging_buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("Staging Buffer"),
        size: (MAX_MOVES * std::mem::size_of::<u16>()) as u64,
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });

    let shader_src = r#"
        struct Board {
            data: array<u32>
        };
        struct Moves {
            data: array<u32>
        };

        @group(0) @binding(0)
        var<storage, read> board: Board;
        @group(0) @binding(1)
        var<storage, read_write> moves: Moves;

        @compute @workgroup_size(1)
        fn main(@builtin(global_invocation_id) id: vec3<u32>) {
            let index = id.x;
            if (index >= 81u) { return; }
            let value = board.data[index];

            // exemple bidon : encoder un coup (index << 8 | value) en u16
            let encoded = (index << 8u) | (value & 0xFFu);

            // chaque thread écrit 1 coup à son index
            moves.data[index] = encoded;
        }
    "#;

    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("Compute Shader"),
        source: wgpu::ShaderSource::Wgsl(shader_src.into()),
    });

    let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("Bind Group Layout"),
        entries: &[
            wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::COMPUTE,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Storage { read_only: true },
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                binding: 1,
                visibility: wgpu::ShaderStages::COMPUTE,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Storage { read_only: false },
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            },
        ],
    });

    let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("Bind Group"),
        layout: &bind_group_layout,
        entries: &[
            wgpu::BindGroupEntry { binding: 0, resource: board_buffer.as_entire_binding() },
            wgpu::BindGroupEntry { binding: 1, resource: output_buffer.as_entire_binding() },
        ],
    });

    let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("Pipeline Layout"),
        bind_group_layouts: &[&bind_group_layout],
        push_constant_ranges: &[],
    });

    let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
        label: Some("Compute Pipeline"),
        layout: Some(&pipeline_layout),
        module: &shader,
        entry_point: Some("main"),
        compilation_options: Default::default(),
        cache: None,
    });

    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
        label: Some("Command Encoder"),
    });

    {
        let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
            label: Some("Compute Pass"),
            timestamp_writes: None,
        });
        pass.set_pipeline(&pipeline);
        pass.set_bind_group(0, &bind_group, &[]);
        pass.dispatch_workgroups(BOARD_SIZE as u32, 1, 1);
    }

    encoder.copy_buffer_to_buffer(&output_buffer, 0, &staging_buffer, 0, staging_buffer.size());
    queue.submit(Some(encoder.finish()));

    let buffer_slice = staging_buffer.slice(..);
    buffer_slice.map_async(wgpu::MapMode::Read, |_| {});
    let _ = device.poll(wgpu::MaintainBase::Wait);
    let data = buffer_slice.get_mapped_range();
    let results: &[u16] = bytemuck::cast_slice(&data);

    println!("GPU results: {:?}", &results[0..32.min(results.len())]);
}
