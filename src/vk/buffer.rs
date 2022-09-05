use std::{
    mem::size_of,
    sync::{Arc, RwLock},
};

use bytemuck::Pod;
use log::debug;
use vulkano::{
    buffer::{BufferAccess, BufferUsage, CpuAccessibleBuffer},
    descriptor_set::WriteDescriptorSet,
    device::Device,
};

use super::HasDescriptor;

pub trait BufferType: Send + Sync + Pod {}

const USAGE: BufferUsage = BufferUsage {
    transfer_src: true,
    uniform_buffer: true,
    storage_buffer: true,
    ..BufferUsage::none()
};

pub struct Buffer<T>
where
    T: BufferType,
{
    device:        Arc<Device>,
    buffers:       RwLock<Vec<Arc<CpuAccessibleBuffer<[T]>>>>,
    active_buffer: RwLock<usize>,
}

impl<T> Buffer<T>
where
    T: BufferType,
{
    pub(super) fn new(device: Arc<Device>, num_buffers: usize, len: u64) -> Self {
        let mut buffers = Vec::with_capacity(num_buffers);
        unsafe {
            for _ in 0..num_buffers {
                buffers
                    .push(CpuAccessibleBuffer::<[T]>::uninitialized_array(device.clone(), len, USAGE, false).unwrap());
            }
        }
        Self {
            device,
            buffers: RwLock::new(buffers),
            active_buffer: RwLock::new(0),
        }
    }

    pub fn write(&self, data: &[T]) {
        let mut buffers_writer = self.buffers.write().unwrap();
        let mut active_buffer = self.active_buffer.write().unwrap();

        *active_buffer += 1;
        *active_buffer %= buffers_writer.len();

        let buffer = &mut buffers_writer[*active_buffer];
        if data.len() != (buffer.size() as usize / (size_of::<T>())) {
            *buffer = CpuAccessibleBuffer::from_iter(self.device.clone(), USAGE, false, data.to_owned()).unwrap();
        } else {
            match buffer.write() {
                Ok(mut writer) => writer.copy_from_slice(data),
                Err(e) => {
                    // if the frame rate is super high, we could be trying to write to this buffer *while* the previous frame is still copying
                    // from the buffer to the image! In this case just log it and skip over
                    debug!("buffer write error: {}", e);
                }
            }
        }
    }
}

impl<T: BufferType> HasDescriptor for Buffer<T> {
    fn get_descriptor(&self, binding: u32, _frame_number: usize) -> WriteDescriptorSet {
        let buffers_reader = self.buffers.read().unwrap();
        let active_buffer = *self.active_buffer.read().unwrap();

        WriteDescriptorSet::buffer(binding, buffers_reader[active_buffer].clone())
    }
}
