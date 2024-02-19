#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct ScaleFactorUniform {
    pub scale_factor: [f32; 2],
}

// #[repr(C)]
// #[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
// pub struct PositionsUniform {
//     pub positions: [[f32; 2]; 100]
// }