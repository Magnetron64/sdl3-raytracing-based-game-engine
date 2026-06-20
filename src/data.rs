use std::{collections::HashMap, error::Error, sync::LazyLock};

use crate::state::Quant;
use cgmath::{Matrix4, Vector3};
use sdl3::gpu::{Buffer, BufferRegion, CommandBuffer, Device, TransferBufferLocation};
use std::fmt::Debug;

const QUANT_NUMBER: usize = 8192 * 2;

pub enum QuantType {
    Stone = 0,
    Water = 1,
}

pub static QUANT_PROPERTIES: LazyLock<HashMap<u32, (f32, f32, Vector3<f32>)>> =
    LazyLock::new(|| {
        let mut map = HashMap::new();
        map.insert(
            QuantType::Stone as u32,
            (
                12.0 as f32,
                0.1 as f32,
                Vector3::new(0.8 as f32, 0.8 as f32, 0.5 as f32),
            ),
        );
        map.insert(
            QuantType::Water as u32,
            (
                18.0 as f32,
                0.1 as f32,
                Vector3::new(0.0 as f32, 0.3 as f32, 0.9 as f32),
            ),
        );
        map
    });

pub fn convert_mat4_to_16(input: Matrix4<f32>) -> [f32; 16] {
    let farray: [[f32; 4]; 4] = input.into();
    let mut output: Vec<f32> = vec![];
    for row in farray {
        for value in row {
            output.push(value)
        }
    }
    let array: [f32; 16] = output.try_into().unwrap();
    array
}

pub fn vec3_to_3(vec3: Vector3<f32>) -> [f32; 3] {
    [vec3.x, vec3.y, vec3.z]
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct Quants {
    count: u32,
    padding: [u32; 3],
    planet_quants: [[f32; 4]; QUANT_NUMBER],
}

impl Quants {
    pub fn new(quants: Vec<Quant>) -> Self {
        let mut arr: [[f32; 4]; QUANT_NUMBER] = [[0.0; 4]; QUANT_NUMBER];
        let mut i = 0;
        for quant in quants.clone().into_iter() {
            let pos3d = vec3_to_3(quant.pos);
            let color3d = vec3_to_3(quant.color);
            let radius3d = quant.radius;

            arr[i] = [pos3d[0], pos3d[1], pos3d[2], radius3d];
            arr[i + 1] = [color3d[0], color3d[1], color3d[2], 0.0];

            i += 2;
        }

        Self {
            count: quants.len() as u32,
            padding: [0; 3],
            planet_quants: arr,
        }
    }
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct Constants {
    pub aspect: f32,
    padding: [f32; 3],
    pub k: f32,
    pub ki: f32,
    pub ck: f32,
    pub cki: f32,
    pub cam_pos: [f32; 4],
    pub view: [f32; 16],
}

impl Constants {
    pub fn new(
        aspect: f32,
        k: f32,
        ki: f32,
        ck: f32,
        cki: f32,
        cam_pos: Vector3<f32>,
        view: Matrix4<f32>,
    ) -> Self {
        let pos3d = vec3_to_3(cam_pos);
        let pos4d = [pos3d[0], pos3d[1], pos3d[2], 0.0];
        Self {
            aspect,
            padding: [0.0; 3],
            k,
            ki,
            ck,
            cki,
            cam_pos: pos4d,
            view: convert_mat4_to_16(view),
        }
    }
}

pub fn upload_to_storage<T: Clone + Copy + Debug>(
    device: &mut Device,
    command_buffer: &mut CommandBuffer,
    data: T,
) -> Result<Buffer, Box<dyn Error>> {
    let size = std::mem::size_of_val(&data);
    // println!("size {:?}", size);
    let buffer = device
        .create_buffer()
        .with_usage(sdl3::gpu::BufferUsageFlags::GraphicsStorageRead)
        .with_size(size as u32)
        .build()?;
    let transfer = device
        .create_transfer_buffer()
        .with_usage(sdl3::gpu::TransferBufferUsage::Upload)
        .with_size(size as u32)
        .build()?;
    let mut mapped = transfer.map::<T>(device, true);
    // println!("mapped 0 {:?}", mapped.mem());

    mapped.mem_mut().copy_from_slice(&[data]);
    // println!("mapped 1 {:?}", mapped.mem());

    mapped.unmap();

    let pass = device.begin_copy_pass(command_buffer)?;
    pass.upload_to_gpu_buffer(
        TransferBufferLocation::default()
            .with_transfer_buffer(&transfer)
            .with_offset(0),
        BufferRegion::default()
            .with_buffer(&buffer)
            .with_size(size as u32)
            .with_offset(0),
        true,
    );
    device.end_copy_pass(pass);

    Ok(buffer)
}
